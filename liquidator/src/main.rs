use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anchor_client::Cluster;
use clap::Parser;
use client::{chain_data, keypair_from_cli, Client, MangoClient, MangoGroupContext};
use log::*;
use mango_v4::state::{PerpMarketIndex, TokenIndex};

use itertools::Itertools;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;

pub mod liquidate;
pub mod metrics;
pub mod rebalance;
pub mod snapshot_source;
pub mod util;
pub mod websocket_source;

use crate::util::{is_mango_account, is_mango_bank, is_mint_info, is_perp_market};

// jemalloc seems to be better at keeping the memory footprint reasonable over
// longer periods of time
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

trait AnyhowWrap {
    type Value;
    fn map_err_anyhow(self) -> anyhow::Result<Self::Value>;
}

impl<T, E: std::fmt::Debug> AnyhowWrap for Result<T, E> {
    type Value = T;
    fn map_err_anyhow(self) -> anyhow::Result<Self::Value> {
        self.map_err(|err| anyhow::anyhow!("{:?}", err))
    }
}

#[derive(Parser, Debug)]
#[clap()]
struct CliDotenv {
    // When --dotenv <file> is passed, read the specified dotenv file before parsing args
    #[clap(long)]
    dotenv: std::path::PathBuf,

    remaining_args: Vec<std::ffi::OsString>,
}

#[derive(Parser)]
#[clap()]
struct Cli {
    #[clap(short, long, env)]
    rpc_url: String,

    // TODO: different serum markets could use different serum programs, should come from registered markets
    #[clap(long, env)]
    serum_program: Pubkey,

    #[clap(long, env)]
    liqor_mango_account: Pubkey,

    #[clap(long, env)]
    liqor_owner: String,

    #[clap(long, env, default_value = "300")]
    snapshot_interval_secs: u64,

    /// how many getMultipleAccounts requests to send in parallel
    #[clap(long, env, default_value = "10")]
    parallel_rpc_requests: usize,

    /// typically 100 is the max number of accounts getMultipleAccounts will retrieve at once
    #[clap(long, env, default_value = "100")]
    get_multiple_accounts_count: usize,

    /// liquidator health ratio should not fall below this value
    #[clap(long, env, default_value = "50")]
    min_health_ratio: f64,

    #[clap(long, env, default_value = "1")]
    rebalance_slippage: f64,
}

pub fn encode_address(addr: &Pubkey) -> String {
    bs58::encode(&addr.to_bytes()).into_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = if let Ok(cli_dotenv) = CliDotenv::try_parse() {
        dotenv::from_path(cli_dotenv.dotenv)?;
        cli_dotenv.remaining_args
    } else {
        dotenv::dotenv().ok();
        std::env::args_os().collect()
    };
    let cli = Cli::parse_from(args);

    let liqor_owner = keypair_from_cli(&cli.liqor_owner);

    let rpc_url = cli.rpc_url;
    let ws_url = rpc_url.replace("https", "wss");

    let rpc_timeout = Duration::from_secs(10);
    let cluster = Cluster::Custom(rpc_url.clone(), ws_url.clone());
    let commitment = CommitmentConfig::processed();
    let client = Client::new(cluster.clone(), commitment, &liqor_owner, Some(rpc_timeout));

    // The representation of current on-chain account data
    let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
    // Reading accounts from chain_data
    let account_fetcher = Arc::new(chain_data::AccountFetcher {
        chain_data: chain_data.clone(),
        rpc: client.rpc_async(),
    });

    let mango_account = account_fetcher
        .fetch_fresh_mango_account(&cli.liqor_mango_account)
        .await?;
    let mango_group = mango_account.fixed.group;

    let group_context = MangoGroupContext::new_from_rpc(&client.rpc_async(), mango_group).await?;

    let mango_oracles = group_context
        .tokens
        .values()
        .map(|value| value.mint_info.oracle)
        .chain(group_context.perp_markets.values().map(|p| p.market.oracle))
        .unique()
        .collect::<Vec<Pubkey>>();

    //
    // feed setup
    //
    // FUTURE: decouple feed setup and liquidator business logic
    // feed should send updates to a channel which liquidator can consume

    let mango_program = mango_v4::ID;

    solana_logger::setup_with_default("info");
    info!("startup");

    let metrics = metrics::start();

    // Sourcing account and slot data from solana via websockets
    // FUTURE: websocket feed should take which accounts to listen to as an input
    let (websocket_sender, websocket_receiver) =
        async_channel::unbounded::<websocket_source::Message>();
    websocket_source::start(
        websocket_source::Config {
            rpc_ws_url: ws_url.clone(),
            mango_program,
            serum_program: cli.serum_program,
            open_orders_authority: mango_group,
        },
        mango_oracles.clone(),
        websocket_sender,
    );

    let first_websocket_slot = websocket_source::get_next_create_bank_slot(
        websocket_receiver.clone(),
        Duration::from_secs(10),
    )
    .await?;

    // Getting solana account snapshots via jsonrpc
    let (snapshot_sender, snapshot_receiver) =
        async_channel::unbounded::<snapshot_source::AccountSnapshot>();
    // FUTURE: of what to fetch a snapshot - should probably take as an input
    snapshot_source::start(
        snapshot_source::Config {
            rpc_http_url: rpc_url.clone(),
            mango_program,
            mango_group,
            get_multiple_accounts_count: cli.get_multiple_accounts_count,
            parallel_rpc_requests: cli.parallel_rpc_requests,
            snapshot_interval: std::time::Duration::from_secs(cli.snapshot_interval_secs),
            min_slot: first_websocket_slot + 10,
        },
        mango_oracles,
        snapshot_sender,
    );

    start_chain_data_metrics(chain_data.clone(), &metrics);

    // Addresses of the MangoAccounts belonging to the mango program.
    // Needed to check health of them all when the cache updates.
    let mut mango_accounts = HashSet::<Pubkey>::new();

    let mut mint_infos = HashMap::<TokenIndex, Pubkey>::new();
    let mut oracles = HashSet::<Pubkey>::new();
    let mut perp_markets = HashMap::<PerpMarketIndex, Pubkey>::new();

    // Is the first snapshot done? Only start checking account health when it is.
    let mut one_snapshot_done = false;

    let mut metric_websocket_queue_len = metrics.register_u64("websocket_queue_length".into());
    let mut metric_snapshot_queue_len = metrics.register_u64("snapshot_queue_length".into());
    let mut metric_mango_accounts = metrics.register_u64("mango_accounts".into());

    //
    // mango client setup
    //
    let mango_client = {
        Arc::new(MangoClient::new_detail(
            client,
            cli.liqor_mango_account,
            liqor_owner,
            group_context,
            account_fetcher.clone(),
        )?)
    };

    let liq_config = liquidate::Config {
        min_health_ratio: cli.min_health_ratio,
        // TODO: config
        refresh_timeout: Duration::from_secs(30),
    };

    let mut rebalance_interval = tokio::time::interval(Duration::from_secs(5));
    let rebalance_config = rebalance::Config {
        slippage: cli.rebalance_slippage,
        // TODO: config
        borrow_settle_excess: 1.05,
        refresh_timeout: Duration::from_secs(30),
    };

    let rebalancer = rebalance::Rebalancer {
        mango_client: &mango_client,
        account_fetcher: &account_fetcher,
        mango_account_address: cli.liqor_mango_account,
        config: rebalance_config,
    };

    let mut liquidation = LiquidationState {
        mango_client: &mango_client,
        account_fetcher: &account_fetcher,
        liquidation_config: liq_config,
        rebalancer: &rebalancer,
        accounts_with_errors: Default::default(),
        error_skip_threshold: 5,
        error_skip_duration: std::time::Duration::from_secs(120),
        error_reset_duration: std::time::Duration::from_secs(360),
    };

    info!("main loop");
    loop {
        tokio::select! {
            message = websocket_receiver.recv() => {

                metric_websocket_queue_len.set(websocket_receiver.len() as u64);
                let message = message.expect("channel not closed");

                // build a model of slots and accounts in `chain_data`
                websocket_source::update_chain_data(&mut chain_data.write().unwrap(), message.clone());

                // specific program logic using the mirrored data
                if let websocket_source::Message::Account(account_write) = message {

                    if is_mango_account(&account_write.account, &mango_program, &mango_group).is_some() {

                        // e.g. to render debug logs RUST_LOG="liquidator=debug"
                        log::debug!("change to mango account {}...", &account_write.pubkey.to_string()[0..3]);

                        // Track all MangoAccounts: we need to iterate over them later
                        mango_accounts.insert(account_write.pubkey);
                        metric_mango_accounts.set(mango_accounts.len() as u64);

                        if !one_snapshot_done {
                            continue;
                        }

                        liquidation.maybe_liquidate_one_and_rebalance(
                            std::iter::once(&account_write.pubkey),
                        ).await?;
                    }

                    if is_mango_bank(&account_write.account, &mango_program, &mango_group).is_some() || oracles.contains(&account_write.pubkey) {
                        if !one_snapshot_done {
                            continue;
                        }

                        if is_mango_bank(&account_write.account, &mango_program, &mango_group).is_some() {
                            log::debug!("change to bank {}", &account_write.pubkey);
                        }

                        if oracles.contains(&account_write.pubkey) {
                            log::debug!("change to oracle {}", &account_write.pubkey);
                        }

                        liquidation.maybe_liquidate_one_and_rebalance(
                            mango_accounts.iter(),
                        ).await?;
                    }
                }
            },

            message = snapshot_receiver.recv() => {
                metric_snapshot_queue_len.set(snapshot_receiver.len() as u64);
                let message = message.expect("channel not closed");

                // Track all mango account pubkeys
                for update in message.accounts.iter() {
                    if is_mango_account(&update.account, &mango_program, &mango_group).is_some() {
                        mango_accounts.insert(update.pubkey);
                    }
                    if let Some(mint_info) = is_mint_info(&update.account, &mango_program, &mango_group) {
                        mint_infos.insert(mint_info.token_index, update.pubkey);
                        oracles.insert(mint_info.oracle);
                    }
                    if let Some(perp_market) = is_perp_market(&update.account, &mango_program, &mango_group) {
                        perp_markets.insert(perp_market.perp_market_index, update.pubkey);
                    }
                }
                metric_mango_accounts.set(mango_accounts.len() as u64);

                snapshot_source::update_chain_data(&mut chain_data.write().unwrap(), message);
                one_snapshot_done = true;

                liquidation.maybe_liquidate_one_and_rebalance(
                    mango_accounts.iter(),
                ).await?;
            },

            _ = rebalance_interval.tick() => {
                if one_snapshot_done {
                    if let Err(err) = rebalancer.zero_all_non_quote().await {
                        log::error!("failed to rebalance liqor: {:?}", err);

                        // Workaround: We really need a sequence enforcer in the liquidator since we don't want to
                        // accidentally send a similar tx again when we incorrectly believe an earlier one got forked
                        // off. For now, hard sleep on error to avoid the most frequent error cases.
                        std::thread::sleep(Duration::from_secs(10));
                    }
                }
            }
        }
    }
}

struct ErrorTracking {
    count: u64,
    last_at: std::time::Instant,
}

struct LiquidationState<'a> {
    mango_client: &'a MangoClient,
    account_fetcher: &'a chain_data::AccountFetcher,
    rebalancer: &'a rebalance::Rebalancer<'a>,
    liquidation_config: liquidate::Config,
    accounts_with_errors: HashMap<Pubkey, ErrorTracking>,
    error_skip_threshold: u64,
    error_skip_duration: std::time::Duration,
    error_reset_duration: std::time::Duration,
}

impl<'a> LiquidationState<'a> {
    async fn maybe_liquidate_one_and_rebalance<'b>(
        &mut self,
        accounts_iter: impl Iterator<Item = &'b Pubkey>,
    ) -> anyhow::Result<()> {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();

        let mut accounts = accounts_iter.collect::<Vec<&Pubkey>>();
        accounts.shuffle(&mut rng);

        let mut liquidated_one = false;
        for pubkey in accounts {
            if self
                .maybe_liquidate_and_log_error(pubkey)
                .await
                .unwrap_or(false)
            {
                liquidated_one = true;
                break;
            }
        }
        if !liquidated_one {
            return Ok(());
        }

        if let Err(err) = self.rebalancer.zero_all_non_quote().await {
            log::error!("failed to rebalance liqor: {:?}", err);
        }
        Ok(())
    }

    async fn maybe_liquidate_and_log_error(&mut self, pubkey: &Pubkey) -> anyhow::Result<bool> {
        let now = std::time::Instant::now();

        // Skip a pubkey if there've been too many errors recently
        if let Some(error_entry) = self.accounts_with_errors.get(pubkey) {
            if error_entry.count >= self.error_skip_threshold
                && now.duration_since(error_entry.last_at) < self.error_skip_duration
            {
                log::trace!(
                    "skip checking account {pubkey}, had {} errors recently",
                    error_entry.count
                );
                return Ok(false);
            }
        }

        let result = liquidate::maybe_liquidate_account(
            self.mango_client,
            self.account_fetcher,
            pubkey,
            &self.liquidation_config,
        )
        .await;

        if let Err(err) = result.as_ref() {
            // Keep track of pubkeys that had errors
            let error_entry = self
                .accounts_with_errors
                .entry(*pubkey)
                .or_insert(ErrorTracking {
                    count: 0,
                    last_at: now,
                });
            if now.duration_since(error_entry.last_at) > self.error_reset_duration {
                error_entry.count = 0;
            }
            error_entry.count += 1;
            error_entry.last_at = now;

            // Not all errors need to be raised to the user's attention.
            let mut log_level = log::Level::Error;

            // Simulation errors due to liqee precondition failures on the liquidation instructions
            // will commonly happen if our liquidator is late or if there are chain forks.
            match err.downcast_ref::<client::MangoClientError>() {
                Some(client::MangoClientError::SendTransactionPreflightFailure { logs }) => {
                    if logs.contains("HealthMustBeNegative") || logs.contains("IsNotBankrupt") {
                        log_level = log::Level::Trace;
                    }
                }
                _ => {}
            };
            log::log!(log_level, "liquidating account {}: {:?}", pubkey, err);
        } else {
            self.accounts_with_errors.remove(pubkey);
        }

        result
    }
}

fn start_chain_data_metrics(chain: Arc<RwLock<chain_data::ChainData>>, metrics: &metrics::Metrics) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

    let mut metric_slots_count = metrics.register_u64("chain_data_slots_count".into());
    let mut metric_accounts_count = metrics.register_u64("chain_data_accounts_count".into());
    let mut metric_account_write_count =
        metrics.register_u64("chain_data_account_write_count".into());

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            let chain_lock = chain.read().unwrap();
            metric_slots_count.set(chain_lock.slots_count() as u64);
            metric_accounts_count.set(chain_lock.accounts_count() as u64);
            metric_account_write_count.set(chain_lock.account_writes_count() as u64);
        }
    });
}
