use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anchor_client::Cluster;
use clap::Parser;
use mango_v4::state::{PerpMarketIndex, TokenIndex};
use mango_v4_client::{
    account_update_stream, chain_data, jupiter, keypair_from_cli, snapshot_source,
    websocket_source, Client, MangoClient, MangoClientError, MangoGroupContext,
    TransactionBuilderConfig,
};

use itertools::Itertools;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use tracing::*;

pub mod liquidate;
pub mod metrics;
pub mod rebalance;
pub mod telemetry;
pub mod token_swap_info;
pub mod trigger_tcs;
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

// Prefer "--rebalance false" over "--no-rebalance" because it works
// better with REBALANCE=false env values.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum BoolArg {
    True,
    False,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum JupiterVersionArg {
    Mock,
    V4,
    V6,
}

impl From<JupiterVersionArg> for jupiter::Version {
    fn from(a: JupiterVersionArg) -> Self {
        match a {
            JupiterVersionArg::Mock => jupiter::Version::Mock,
            JupiterVersionArg::V4 => jupiter::Version::V4,
            JupiterVersionArg::V6 => jupiter::Version::V6,
        }
    }
}

#[derive(Parser)]
#[clap()]
struct Cli {
    #[clap(short, long, env)]
    rpc_url: String,

    #[clap(long, env)]
    liqor_mango_account: Pubkey,

    #[clap(long, env)]
    liqor_owner: String,

    #[clap(long, env, default_value = "1000")]
    check_interval_ms: u64,

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

    /// if rebalancing is enabled
    ///
    /// typically only disabled for tests where swaps are unavailable
    #[clap(long, env, value_enum, default_value = "true")]
    rebalance: BoolArg,

    /// max slippage to request on swaps to rebalance spot tokens
    #[clap(long, env, default_value = "100")]
    rebalance_slippage_bps: u64,

    /// prioritize each transaction with this many microlamports/cu
    #[clap(long, env, default_value = "0")]
    prioritization_micro_lamports: u64,

    /// compute limit requested for liquidation instructions
    #[clap(long, env, default_value = "250000")]
    compute_limit_for_liquidation: u32,

    /// compute limit requested for tcs trigger instructions
    #[clap(long, env, default_value = "300000")]
    compute_limit_for_tcs: u32,

    /// control which version of jupiter to use
    #[clap(long, env, value_enum, default_value = "v6")]
    jupiter_version: JupiterVersionArg,

    /// report liquidator's existence and pubkey
    #[clap(long, env, value_enum, default_value = "true")]
    telemetry: BoolArg,
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

    let liqor_owner = Arc::new(keypair_from_cli(&cli.liqor_owner));

    let rpc_url = cli.rpc_url;
    let ws_url = rpc_url.replace("https", "wss");

    let rpc_timeout = Duration::from_secs(10);
    let cluster = Cluster::Custom(rpc_url.clone(), ws_url.clone());
    let commitment = CommitmentConfig::processed();
    let client = Client::new(
        cluster.clone(),
        commitment,
        liqor_owner.clone(),
        Some(rpc_timeout),
        TransactionBuilderConfig {
            prioritization_micro_lamports: (cli.prioritization_micro_lamports > 0)
                .then_some(cli.prioritization_micro_lamports),
        },
    );

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

    let serum_programs = group_context
        .serum3_markets
        .values()
        .map(|s3| s3.market.serum_program)
        .unique()
        .collect_vec();

    //
    // feed setup
    //
    // FUTURE: decouple feed setup and liquidator business logic
    // feed should send updates to a channel which liquidator can consume

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
            snapshot_interval: Duration::from_secs(cli.snapshot_interval_secs),
            min_slot: first_websocket_slot + 10,
        },
        mango_oracles,
        account_update_sender,
    );

    start_chain_data_metrics(chain_data.clone(), &metrics);

    let shared_state = Arc::new(RwLock::new(SharedState::default()));

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

    let token_swap_info_config = token_swap_info::Config {
        quote_index: 0,              // USDC
        quote_amount: 1_000_000_000, // TODO: config, $1000, should be >= tcs_config.max_trigger_quote_amount
        jupiter_version: cli.jupiter_version.into(),
    };

    let token_swap_info_updater = Arc::new(token_swap_info::TokenSwapInfoUpdater::new(
        mango_client.clone(),
        token_swap_info_config,
    ));

    let liq_config = liquidate::Config {
        min_health_ratio: cli.min_health_ratio,
        compute_limit_for_liq_ix: cli.compute_limit_for_liquidation,
        // TODO: config
        refresh_timeout: Duration::from_secs(30),
    };

    let tcs_config = trigger_tcs::Config {
        min_health_ratio: cli.min_health_ratio,
        max_trigger_quote_amount: 1_000_000_000, // TODO: config, $1000
        jupiter_version: cli.jupiter_version.into(),
        compute_limit_for_trigger: cli.compute_limit_for_tcs,
        // TODO: config
        refresh_timeout: Duration::from_secs(30),
    };

    let mut rebalance_interval = tokio::time::interval(Duration::from_secs(5));
    let rebalance_config = rebalance::Config {
        enabled: cli.rebalance == BoolArg::True,
        slippage_bps: cli.rebalance_slippage_bps,
        // TODO: config
        borrow_settle_excess: 1.05,
        refresh_timeout: Duration::from_secs(30),
        jupiter_version: cli.jupiter_version.into(),
    };

    let rebalancer = Arc::new(rebalance::Rebalancer {
        mango_client: mango_client.clone(),
        account_fetcher: account_fetcher.clone(),
        mango_account_address: cli.liqor_mango_account,
        config: rebalance_config,
    });

    let mut liquidation = Box::new(LiquidationState {
        mango_client: mango_client.clone(),
        account_fetcher,
        liquidation_config: liq_config,
        trigger_tcs_config: tcs_config,
        rebalancer: rebalancer.clone(),
        token_swap_info: token_swap_info_updater.clone(),
        liq_errors: ErrorTracking {
            skip_threshold: 5,
            skip_duration: Duration::from_secs(120),
            ..ErrorTracking::default()
        },
        tcs_collection_hard_errors: ErrorTracking {
            skip_threshold: 2,
            skip_duration: Duration::from_secs(120),
            ..ErrorTracking::default()
        },
        tcs_collection_partial_errors: ErrorTracking {
            skip_threshold: 2,
            skip_duration: Duration::from_secs(120),
            ..ErrorTracking::default()
        },
        tcs_execution_errors: ErrorTracking {
            skip_threshold: 2,
            skip_duration: Duration::from_secs(120),
            ..ErrorTracking::default()
        },
        persistent_error_report_interval: Duration::from_secs(300),
        persistent_error_min_duration: Duration::from_secs(300),
        last_persistent_error_report: Instant::now(),
    });

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

    // Could be refactored to only start the below jobs when the first snapshot is done.
    // But need to take care to abort if the above job aborts beforehand.

    let rebalance_job = tokio::spawn({
        let shared_state = shared_state.clone();
        async move {
            loop {
                rebalance_interval.tick().await;
                if !shared_state.read().unwrap().one_snapshot_done {
                    continue;
                }
                if let Err(err) = rebalancer.zero_all_non_quote().await {
                    error!("failed to rebalance liqor: {:?}", err);

                    // Workaround: We really need a sequence enforcer in the liquidator since we don't want to
                    // accidentally send a similar tx again when we incorrectly believe an earlier one got forked
                    // off. For now, hard sleep on error to avoid the most frequent error cases.
                    std::thread::sleep(Duration::from_secs(10));
                }
            }
        }
    });

    let liquidation_job = tokio::spawn({
        let mut interval = tokio::time::interval(Duration::from_millis(cli.check_interval_ms));
        let shared_state = shared_state.clone();
        async move {
            loop {
                interval.tick().await;

                let account_addresses = {
                    let state = shared_state.write().unwrap();
                    if !state.one_snapshot_done {
                        continue;
                    }
                    state.mango_accounts.iter().cloned().collect_vec()
                };

                liquidation.log_persistent_errors();

                let liquidated = liquidation
                    .maybe_liquidate_one_and_rebalance(account_addresses.iter())
                    .await
                    .unwrap();

                if !liquidated {
                    liquidation
                        .maybe_take_token_conditional_swap(account_addresses.iter())
                        .await
                        .unwrap();
                }
            }
        }
    });

    let token_swap_info_job = tokio::spawn({
        // TODO: configurable interval
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        let mut min_delay = tokio::time::interval(Duration::from_secs(1));
        let shared_state = shared_state.clone();
        async move {
            loop {
                min_delay.tick().await;
                if !shared_state.read().unwrap().one_snapshot_done {
                    continue;
                }

                interval.tick().await;
                let token_indexes = token_swap_info_updater
                    .mango_client()
                    .context
                    .token_indexes_by_name
                    .values()
                    .copied()
                    .collect_vec();
                for token_index in token_indexes {
                    min_delay.tick().await;
                    match token_swap_info_updater.update_one(token_index).await {
                        Ok(()) => {}
                        Err(err) => {
                            warn!(
                                "failed to update token swap info for token {token_index}: {err:?}",
                            );
                        }
                    }
                }
                token_swap_info_updater.log_all();
            }
        }
    });

    if cli.telemetry == BoolArg::True {
        tokio::spawn(telemetry::report_regularly(
            mango_client,
            cli.min_health_ratio,
        ));
    }

    use futures::StreamExt;
    let mut jobs: futures::stream::FuturesUnordered<_> = vec![
        data_job,
        rebalance_job,
        liquidation_job,
        token_swap_info_job,
    ]
    .into_iter()
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

#[derive(Clone)]
pub struct AccountErrorState {
    pub messages: Vec<String>,
    pub count: u64,
    pub last_at: Instant,
}

#[derive(Default)]
pub struct ErrorTracking {
    accounts: HashMap<Pubkey, AccountErrorState>,
    skip_threshold: u64,
    skip_duration: Duration,
}

impl ErrorTracking {
    pub fn had_too_many_errors(&self, pubkey: &Pubkey, now: Instant) -> Option<AccountErrorState> {
        if let Some(error_entry) = self.accounts.get(pubkey) {
            if error_entry.count >= self.skip_threshold
                && now.duration_since(error_entry.last_at) < self.skip_duration
            {
                Some(error_entry.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn record_error(&mut self, pubkey: &Pubkey, now: Instant, message: String) {
        let error_entry = self.accounts.entry(*pubkey).or_insert(AccountErrorState {
            messages: Vec::with_capacity(1),
            count: 0,
            last_at: now,
        });
        error_entry.count += 1;
        error_entry.last_at = now;
        if !error_entry.messages.contains(&message) {
            error_entry.messages.push(message);
        }
        if error_entry.messages.len() > 5 {
            error_entry.messages.remove(0);
        }
    }

    pub fn clear_errors(&mut self, pubkey: &Pubkey) {
        self.accounts.remove(pubkey);
    }

    #[instrument(skip_all, fields(%error_type))]
    #[allow(unused_variables)]
    pub fn log_persistent_errors(&self, error_type: &str, min_duration: Duration) {
        let now = Instant::now();
        for (pubkey, errors) in self.accounts.iter() {
            if now.duration_since(errors.last_at) < min_duration {
                continue;
            }
            info!(
                %pubkey,
                count = errors.count,
                messages = ?errors.messages,
                "has persistent errors",
            );
        }
    }
}

struct LiquidationState {
    mango_client: Arc<MangoClient>,
    account_fetcher: Arc<chain_data::AccountFetcher>,
    rebalancer: Arc<rebalance::Rebalancer>,
    token_swap_info: Arc<token_swap_info::TokenSwapInfoUpdater>,
    liquidation_config: liquidate::Config,
    trigger_tcs_config: trigger_tcs::Config,

    liq_errors: ErrorTracking,
    /// Errors that suggest we maybe should skip trying to collect tcs for that pubkey
    tcs_collection_hard_errors: ErrorTracking,
    /// Recording errors when some tcs have errors during collection but others don't
    tcs_collection_partial_errors: ErrorTracking,
    tcs_execution_errors: ErrorTracking,
    persistent_error_report_interval: Duration,
    last_persistent_error_report: Instant,
    persistent_error_min_duration: Duration,
}

impl LiquidationState {
    async fn maybe_liquidate_one_and_rebalance<'b>(
        &mut self,
        accounts_iter: impl Iterator<Item = &'b Pubkey>,
    ) -> anyhow::Result<bool> {
        use rand::seq::SliceRandom;

        let mut accounts = accounts_iter.collect::<Vec<&Pubkey>>();
        {
            let mut rng = rand::thread_rng();
            accounts.shuffle(&mut rng);
        }

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
            return Ok(false);
        }

        if let Err(err) = self.rebalancer.zero_all_non_quote().await {
            error!("failed to rebalance liqor: {:?}", err);
        }
        Ok(true)
    }

    async fn maybe_liquidate_and_log_error(&mut self, pubkey: &Pubkey) -> anyhow::Result<bool> {
        let now = Instant::now();
        let error_tracking = &mut self.liq_errors;

        // Skip a pubkey if there've been too many errors recently
        if let Some(error_entry) = error_tracking.had_too_many_errors(pubkey, now) {
            trace!(
                %pubkey,
                error_entry.count,
                "skip checking account for liquidation, had errors recently",
            );
            return Ok(false);
        }

        let result = liquidate::maybe_liquidate_account(
            &self.mango_client,
            &self.account_fetcher,
            pubkey,
            &self.liquidation_config,
        )
        .await;

        if let Err(err) = result.as_ref() {
            // Keep track of pubkeys that had errors
            error_tracking.record_error(pubkey, now, err.to_string());

            // Not all errors need to be raised to the user's attention.
            let mut is_error = true;

            // Simulation errors due to liqee precondition failures on the liquidation instructions
            // will commonly happen if our liquidator is late or if there are chain forks.
            match err.downcast_ref::<MangoClientError>() {
                Some(MangoClientError::SendTransactionPreflightFailure { logs, .. }) => {
                    if logs.iter().any(|line| {
                        line.contains("HealthMustBeNegative") || line.contains("IsNotBankrupt")
                    }) {
                        is_error = false;
                    }
                }
                _ => {}
            };
            if is_error {
                error!("liquidating account {}: {:?}", pubkey, err);
            } else {
                trace!("liquidating account {}: {:?}", pubkey, err);
            }
        } else {
            error_tracking.clear_errors(pubkey);
        }

        result
    }

    async fn maybe_take_token_conditional_swap<'b>(
        &mut self,
        accounts_iter: impl Iterator<Item = &'b Pubkey>,
    ) -> anyhow::Result<()> {
        let accounts = accounts_iter.collect::<Vec<&Pubkey>>();

        let now = Instant::now();
        let now_ts: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
            .try_into()?;

        // Find interesting (pubkey, tcsid, volume)
        let mut interesting_tcs = Vec::with_capacity(accounts.len());
        for pubkey in accounts.iter() {
            if let Some(error_entry) = self
                .tcs_collection_hard_errors
                .had_too_many_errors(pubkey, now)
            {
                trace!(
                    %pubkey,
                    error_entry.count,
                    "skip checking account for tcs, had errors recently",
                );
                continue;
            }

            match trigger_tcs::find_interesting_tcs_for_account(
                pubkey,
                &self.mango_client,
                &self.account_fetcher,
                &self.token_swap_info,
                now_ts,
            ) {
                Ok(v) => {
                    self.tcs_collection_hard_errors.clear_errors(pubkey);
                    if v.is_empty() {
                        self.tcs_collection_partial_errors.clear_errors(pubkey);
                        self.tcs_execution_errors.clear_errors(pubkey);
                    } else if v.iter().all(|it| it.is_ok()) {
                        self.tcs_collection_partial_errors.clear_errors(pubkey);
                    } else {
                        for it in v.iter() {
                            if let Err(e) = it {
                                self.tcs_collection_partial_errors.record_error(
                                    pubkey,
                                    now,
                                    e.to_string(),
                                );
                            }
                        }
                    }
                    interesting_tcs.extend(v.iter().filter_map(|it| it.as_ref().ok()));
                }
                Err(e) => {
                    self.tcs_collection_hard_errors
                        .record_error(pubkey, now, e.to_string());
                }
            }
        }
        if interesting_tcs.is_empty() {
            return Ok(());
        }

        let tcs_context = trigger_tcs::ExecutionContext {
            mango_client: self.mango_client.clone(),
            account_fetcher: self.account_fetcher.clone(),
            token_swap_info: self.token_swap_info.clone(),
            config: self.trigger_tcs_config.clone(),
        };
        let (txsigs, mut changed_pubkeys) = tcs_context
            .execute_tcs(&mut interesting_tcs, &mut self.tcs_execution_errors)
            .await?;
        changed_pubkeys.push(self.mango_client.mango_account_address);

        // Force a refresh of affected accounts
        let slot = self.account_fetcher.transaction_max_slot(&txsigs).await?;
        if let Err(e) = self
            .account_fetcher
            .refresh_accounts_via_rpc_until_slot(
                &changed_pubkeys,
                slot,
                self.liquidation_config.refresh_timeout,
            )
            .await
        {
            info!(slot, "could not refresh after tcs execution: {}", e);
        }

        if let Err(err) = self.rebalancer.zero_all_non_quote().await {
            error!("failed to rebalance liqor: {:?}", err);
        }
        Ok(())
    }

    fn log_persistent_errors(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_persistent_error_report)
            < self.persistent_error_report_interval
        {
            return;
        }
        self.last_persistent_error_report = now;

        let min_duration = self.persistent_error_min_duration;
        self.liq_errors
            .log_persistent_errors("liquidation", min_duration);
        self.tcs_execution_errors
            .log_persistent_errors("tcs execution", min_duration);
        self.tcs_collection_hard_errors
            .log_persistent_errors("tcs collection hard", min_duration);
        self.tcs_collection_partial_errors
            .log_persistent_errors("tcs collection partial", min_duration);
    }
}

fn start_chain_data_metrics(chain: Arc<RwLock<chain_data::ChainData>>, metrics: &metrics::Metrics) {
    let mut interval = tokio::time::interval(Duration::from_secs(600));

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
