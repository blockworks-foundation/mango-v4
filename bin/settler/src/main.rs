use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anchor_client::Cluster;
use clap::Parser;
use mango_v4::state::{PerpMarketIndex, TokenIndex};
use mango_v4_client::{
    account_update_stream, chain_data, keypair_from_cli, priority_fees_cli, snapshot_source,
    websocket_source, Client, MangoClient, MangoGroupContext, TransactionBuilderConfig,
};
use tracing::*;

use itertools::Itertools;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;

pub mod metrics;
pub mod settle;
pub mod tcs_start;
pub mod util;

use crate::util::{is_mango_account, is_mint_info, is_perp_market};

// jemalloc seems to be better at keeping the memory footprint reasonable over
// longer periods of time
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

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

    #[clap(long, env)]
    settler_mango_account: Pubkey,

    #[clap(long, env)]
    settler_owner: String,

    #[clap(long, env, default_value = "300")]
    snapshot_interval_secs: u64,

    /// how many getMultipleAccounts requests to send in parallel
    #[clap(long, env, default_value = "10")]
    parallel_rpc_requests: usize,

    /// typically 100 is the max number of accounts getMultipleAccounts will retrieve at once
    #[clap(long, env, default_value = "100")]
    get_multiple_accounts_count: usize,

    #[clap(flatten)]
    prioritization_fee_cli: priority_fees_cli::PriorityFeeArgs,

    /// url to the lite-rpc websocket, optional
    #[clap(long, env, default_value = "")]
    lite_rpc_url: String,

    /// compute budget for each instruction
    #[clap(long, env, default_value = "250000")]
    compute_budget_per_instruction: u32,
}

pub fn encode_address(addr: &Pubkey) -> String {
    bs58::encode(&addr.to_bytes()).into_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mango_v4_client::tracing_subscriber_init();

    let args = if let Ok(cli_dotenv) = CliDotenv::try_parse() {
        dotenv::from_path(cli_dotenv.dotenv)?;
        cli_dotenv.remaining_args
    } else {
        dotenv::dotenv().ok();
        std::env::args_os().collect()
    };
    let cli = Cli::parse_from(args);

    let (prio_provider, prio_jobs) = cli
        .prioritization_fee_cli
        .make_prio_provider(cli.lite_rpc_url.clone())?;

    let settler_owner = Arc::new(keypair_from_cli(&cli.settler_owner));

    let rpc_url = cli.rpc_url;
    let ws_url = rpc_url.replace("https", "wss");

    let rpc_timeout = Duration::from_secs(10);
    let cluster = Cluster::Custom(rpc_url.clone(), ws_url.clone());
    let commitment = CommitmentConfig::processed();
    let client = Client::new(
        cluster.clone(),
        commitment,
        settler_owner.clone(),
        Some(rpc_timeout),
        TransactionBuilderConfig::builder()
            .compute_budget_per_instruction(Some(cli.compute_budget_per_instruction))
            .priority_fee_provider(prio_provider)
            .build()
            .unwrap(),
    );

    // The representation of current on-chain account data
    let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
    // Reading accounts from chain_data
    let account_fetcher = Arc::new(chain_data::AccountFetcher {
        chain_data: chain_data.clone(),
        rpc: client.new_rpc_async(),
    });

    let mango_account = account_fetcher
        .fetch_fresh_mango_account(&cli.settler_mango_account)
        .await?;
    let mango_group = mango_account.fixed.group;

    let group_context = MangoGroupContext::new_from_rpc(client.rpc_async(), mango_group).await?;

    let mango_oracles = group_context
        .tokens
        .values()
        .map(|value| value.oracle)
        .chain(group_context.perp_markets.values().map(|p| p.oracle))
        .unique()
        .collect::<Vec<Pubkey>>();

    let serum_programs = group_context
        .serum3_markets
        .values()
        .map(|s3| s3.serum_program)
        .unique()
        .collect_vec();

    //
    // feed setup
    //
    // FUTURE: decouple feed setup and liquidator business logic
    // feed should send updates to a channel which liquidator can consume

    solana_logger::setup_with_default("info");
    info!("startup");

    let metrics = metrics::start();

    let (account_update_sender, account_update_receiver) =
        async_channel::unbounded::<account_update_stream::Message>();

    // Sourcing account and slot data from solana via websockets
    // FUTURE: websocket feed should take which accounts to listen to as an input
    websocket_source::start(
        websocket_source::Config {
            rpc_ws_url: ws_url.clone(),
            serum_programs,
            open_orders_authority: mango_group,
        },
        mango_oracles.clone(),
        account_update_sender.clone(),
    );

    let first_websocket_slot = websocket_source::get_next_create_bank_slot(
        account_update_receiver.clone(),
        Duration::from_secs(10),
    )
    .await?;

    // Getting solana account snapshots via jsonrpc
    // FUTURE: of what to fetch a snapshot - should probably take as an input
    snapshot_source::start(
        snapshot_source::Config {
            rpc_http_url: rpc_url.clone(),
            mango_group,
            get_multiple_accounts_count: cli.get_multiple_accounts_count,
            parallel_rpc_requests: cli.parallel_rpc_requests,
            snapshot_interval: std::time::Duration::from_secs(cli.snapshot_interval_secs),
            min_slot: first_websocket_slot + 10,
        },
        mango_oracles,
        account_update_sender.clone(),
    );

    start_chain_data_metrics(chain_data.clone(), &metrics);

    let shared_state = Arc::new(RwLock::new(SharedState::default()));

    //
    // mango client setup
    //
    let mango_client = {
        Arc::new(MangoClient::new_detail(
            client,
            cli.settler_mango_account,
            settler_owner,
            group_context,
            account_fetcher.clone(),
        )?)
    };

    let settle_config = settle::Config {
        settle_cooldown: std::time::Duration::from_secs(10),
    };

    let mut settlement = settle::SettlementState {
        mango_client: mango_client.clone(),
        account_fetcher: account_fetcher.clone(),
        config: settle_config,
        recently_settled: Default::default(),
    };

    let mut tcs_start = tcs_start::State {
        mango_client: mango_client.clone(),
        account_fetcher: account_fetcher.clone(),
        config: tcs_start::Config {
            persistent_error_report_interval: Duration::from_secs(300),
        },
        errors: mango_v4_client::error_tracking::ErrorTracking::builder()
            .skip_threshold(2)
            .skip_duration(Duration::from_secs(60))
            .build()?,
    };

    info!("main loop");

    // Job to update chain_data and notify the liquidation job when a new check is needed.
    let data_job = tokio::spawn({
        use account_update_stream::Message;

        let shared_state = shared_state.clone();

        let mut metric_account_update_queue_len =
            metrics.register_u64("account_update_queue_length".into());
        let mut metric_mango_accounts = metrics.register_u64("mango_accounts".into());

        let mut mint_infos = HashMap::<TokenIndex, Pubkey>::new();
        let mut oracles = HashSet::<Pubkey>::new();
        let mut perp_markets = HashMap::<PerpMarketIndex, Pubkey>::new();

        async move {
            loop {
                let message = account_update_receiver
                    .recv()
                    .await
                    .expect("channel not closed");
                metric_account_update_queue_len.set(account_update_receiver.len() as u64);

                message.update_chain_data(&mut chain_data.write().unwrap());

                match message {
                    Message::Account(account_write) => {
                        let mut state = shared_state.write().unwrap();
                        if is_mango_account(&account_write.account, &mango_group).is_some() {
                            // e.g. to render debug logs RUST_LOG="liquidator=debug"
                            debug!(
                                "change to mango account {}...",
                                &account_write.pubkey.to_string()[0..3]
                            );

                            // Track all MangoAccounts: we need to iterate over them later
                            state.mango_accounts.insert(account_write.pubkey);
                            metric_mango_accounts.set(state.mango_accounts.len() as u64);
                        }
                    }
                    Message::Snapshot(snapshot) => {
                        let mut state = shared_state.write().unwrap();
                        // Track all mango account pubkeys
                        for update in snapshot.iter() {
                            if is_mango_account(&update.account, &mango_group).is_some() {
                                state.mango_accounts.insert(update.pubkey);
                            }
                            if let Some(mint_info) = is_mint_info(&update.account, &mango_group) {
                                mint_infos.insert(mint_info.token_index, update.pubkey);
                                oracles.insert(mint_info.oracle);
                            }
                            if let Some(perp_market) = is_perp_market(&update.account, &mango_group)
                            {
                                perp_markets.insert(perp_market.perp_market_index, update.pubkey);
                                oracles.insert(perp_market.oracle);
                            }
                        }
                        metric_mango_accounts.set(state.mango_accounts.len() as u64);

                        state.one_snapshot_done = true;
                    }
                    _ => {}
                }
            }
        }
    });

    let settle_job = tokio::spawn({
        let mut interval = mango_v4_client::delay_interval(Duration::from_millis(100));
        let shared_state = shared_state.clone();
        async move {
            loop {
                interval.tick().await;

                let account_addresses;
                {
                    let state = shared_state.read().unwrap();
                    if !state.one_snapshot_done {
                        continue;
                    }
                    account_addresses = state.mango_accounts.iter().cloned().collect();
                }

                if let Err(err) = settlement.settle(account_addresses).await {
                    warn!("settle error: {err:?}");
                }
            }
        }
    });

    let tcs_start_job = tokio::spawn({
        let mut interval = mango_v4_client::delay_interval(Duration::from_millis(100));
        let shared_state = shared_state.clone();
        async move {
            loop {
                interval.tick().await;

                let account_addresses;
                {
                    let state = shared_state.read().unwrap();
                    if !state.one_snapshot_done {
                        continue;
                    }
                    account_addresses = state.mango_accounts.iter().cloned().collect();
                }

                if let Err(err) = tcs_start.run_pass(account_addresses).await {
                    warn!("tcs-start error: {err:?}");
                }
            }
        }
    });

    let check_changes_for_abort_job =
        tokio::spawn(MangoClient::loop_check_for_context_changes_and_abort(
            mango_client.clone(),
            Duration::from_secs(300),
        ));

    use futures::StreamExt;
    let mut jobs: futures::stream::FuturesUnordered<_> = vec![
        data_job,
        settle_job,
        tcs_start_job,
        check_changes_for_abort_job,
    ]
    .into_iter()
    .chain(prio_jobs.into_iter())
    .collect();
    jobs.next().await;

    error!("a critical job aborted, exiting");
    Ok(())
}

#[derive(Default)]
struct SharedState {
    /// Addresses of the MangoAccounts belonging to the mango program.
    /// Needed to check health of them all when the cache updates.
    mango_accounts: HashSet<Pubkey>,

    /// Is the first snapshot done? Only start checking account health when it is.
    one_snapshot_done: bool,
}

fn start_chain_data_metrics(chain: Arc<RwLock<chain_data::ChainData>>, metrics: &metrics::Metrics) {
    let mut interval = mango_v4_client::delay_interval(std::time::Duration::from_secs(600));

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
