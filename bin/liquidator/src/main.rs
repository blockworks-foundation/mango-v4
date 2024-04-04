use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anchor_client::Cluster;
use anyhow::Context;
use clap::Parser;
use mango_v4::state::{PerpMarketIndex, TokenIndex};
use mango_v4_client::AsyncChannelSendUnlessFull;
use mango_v4_client::{
    account_update_stream, chain_data, error_tracking::ErrorTracking, keypair_from_cli,
    snapshot_source, websocket_source, Client, MangoClient, MangoClientError, MangoGroupContext,
    TransactionBuilderConfig,
};

use itertools::Itertools;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use tracing::*;

pub mod cli_args;
pub mod liquidate;
pub mod metrics;
pub mod rebalance;
pub mod telemetry;
pub mod token_swap_info;
pub mod trigger_tcs;
mod unwrappable_oracle_error;
pub mod util;

use crate::unwrappable_oracle_error::UnwrappableOracleError;
use crate::util::{is_mango_account, is_mint_info, is_perp_market};

// jemalloc seems to be better at keeping the memory footprint reasonable over
// longer periods of time
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

pub fn encode_address(addr: &Pubkey) -> String {
    bs58::encode(&addr.to_bytes()).into_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mango_v4_client::tracing_subscriber_init();

    let args: Vec<std::ffi::OsString> = if let Ok(cli_dotenv) = CliDotenv::try_parse() {
        dotenv::from_path(cli_dotenv.dotenv)?;
        std::env::args_os()
            .take(1)
            .chain(cli_dotenv.remaining_args.into_iter())
            .collect()
    } else {
        dotenv::dotenv().ok();
        std::env::args_os().collect()
    };
    let cli = Cli::parse_from(args);

    //
    // Priority fee setup
    //
    let (prio_provider, prio_jobs) = cli
        .prioritization_fee_cli
        .make_prio_provider(cli.lite_rpc_url.clone())?;

    //
    // Client setup
    //
    let liqor_owner = Arc::new(keypair_from_cli(&cli.liqor_owner));
    let rpc_url = cli.rpc_url;
    let ws_url = rpc_url.replace("https", "wss");
    let rpc_timeout = Duration::from_secs(10);
    let cluster = Cluster::Custom(rpc_url.clone(), ws_url.clone());
    let commitment = CommitmentConfig::processed();
    let client = Client::builder()
        .cluster(cluster.clone())
        .commitment(commitment)
        .fee_payer(Some(liqor_owner.clone()))
        .timeout(rpc_timeout)
        .jupiter_v6_url(cli.jupiter_v6_url)
        .jupiter_token(cli.jupiter_token)
        .transaction_builder_config(
            TransactionBuilderConfig::builder()
                .priority_fee_provider(prio_provider)
                // Liquidation and tcs triggers set their own budgets, this is a default for other tx
                .compute_budget_per_instruction(Some(250_000))
                .build()
                .unwrap(),
        )
        .override_send_transaction_urls(cli.override_send_transaction_url)
        .build()
        .unwrap();

    // The representation of current on-chain account data
    let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
    // Reading accounts from chain_data
    let account_fetcher = Arc::new(chain_data::AccountFetcher {
        chain_data: chain_data.clone(),
        rpc: client.new_rpc_async(),
    });

    let mango_account = account_fetcher
        .fetch_fresh_mango_account(&cli.liqor_mango_account)
        .await?;
    let mango_group = mango_account.fixed.group;

    let signer_is_owner = mango_account.fixed.owner == liqor_owner.pubkey();
    if cli.rebalance == BoolArg::True && !signer_is_owner {
        warn!("rebalancing on delegated accounts will be unable to free token positions reliably, withdraw dust manually");
    }

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
        quote_index: 0, // USDC
        quote_amount: (cli.jupiter_swap_info_amount * 1e6) as u64,
        jupiter_version: cli.jupiter_version.into(),
    };

    let token_swap_info_updater = Arc::new(token_swap_info::TokenSwapInfoUpdater::new(
        mango_client.clone(),
        token_swap_info_config,
    ));

    let liq_config = liquidate::Config {
        min_health_ratio: cli.min_health_ratio,
        compute_limit_for_liq_ix: cli.compute_limit_for_liquidation,
        max_cu_per_transaction: 1_000_000,
        refresh_timeout: Duration::from_secs(cli.liquidation_refresh_timeout_secs as u64),
        only_allowed_tokens: cli_args::cli_to_hashset::<TokenIndex>(cli.only_allow_tokens),
        forbidden_tokens: cli_args::cli_to_hashset::<TokenIndex>(cli.forbidden_tokens),
        only_allowed_perp_markets: cli_args::cli_to_hashset::<PerpMarketIndex>(
            cli.liquidation_only_allow_perp_markets,
        ),
        forbidden_perp_markets: cli_args::cli_to_hashset::<PerpMarketIndex>(
            cli.liquidation_forbidden_perp_markets,
        ),
    };

    let tcs_config = trigger_tcs::Config {
        min_health_ratio: cli.min_health_ratio,
        max_trigger_quote_amount: (cli.tcs_max_trigger_amount * 1e6) as u64,
        compute_limit_for_trigger: cli.compute_limit_for_tcs,
        profit_fraction: cli.tcs_profit_fraction,
        collateral_token_index: 0, // USDC

        jupiter_version: cli.jupiter_version.into(),
        jupiter_slippage_bps: cli.rebalance_slippage_bps,

        mode: cli.tcs_mode.into(),
        min_buy_fraction: cli.tcs_min_buy_fraction,

        only_allowed_tokens: liq_config.only_allowed_tokens.clone(),
        forbidden_tokens: liq_config.forbidden_tokens.clone(),
    };

    let mut rebalance_interval = tokio::time::interval(Duration::from_secs(30));
    let (rebalance_trigger_sender, rebalance_trigger_receiver) = async_channel::bounded::<()>(1);
    let rebalance_config = rebalance::Config {
        enabled: cli.rebalance == BoolArg::True,
        slippage_bps: cli.rebalance_slippage_bps,
        borrow_settle_excess: (1f64 + cli.rebalance_borrow_settle_excess).max(1f64),
        refresh_timeout: Duration::from_secs(cli.rebalance_refresh_timeout_secs),
        jupiter_version: cli.jupiter_version.into(),
        skip_tokens: cli.rebalance_skip_tokens.unwrap_or(Vec::new()),
        allow_withdraws: signer_is_owner,
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
        token_swap_info: token_swap_info_updater.clone(),
        errors: ErrorTracking::builder()
            .skip_threshold(2)
            .skip_threshold_for_type(LiqErrorType::Liq, 5)
            .skip_duration(Duration::from_secs(120))
            .build()?,
        oracle_errors: ErrorTracking::builder()
            .skip_threshold(1)
            .skip_duration(Duration::from_secs(
                cli.skip_oracle_error_in_logs_duration_secs,
            ))
            .build()?,
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
                tokio::select! {
                    _ = rebalance_interval.tick() => {}
                    _ = rebalance_trigger_receiver.recv() => {}
                }
                if !shared_state.read().unwrap().one_snapshot_done {
                    continue;
                }
                if let Err(err) = rebalancer.zero_all_non_quote().await {
                    error!("failed to rebalance liqor: {:?}", err);

                    // Workaround: We really need a sequence enforcer in the liquidator since we don't want to
                    // accidentally send a similar tx again when we incorrectly believe an earlier one got forked
                    // off. For now, hard sleep on error to avoid the most frequent error cases.
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
    });

    let liquidation_job = tokio::spawn({
        let mut interval =
            mango_v4_client::delay_interval(Duration::from_millis(cli.check_interval_ms));
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

                liquidation.errors.update();
                liquidation.oracle_errors.update();

                let liquidated = liquidation
                    .maybe_liquidate_one(account_addresses.iter())
                    .await;

                let mut took_tcs = false;
                if !liquidated && cli.take_tcs == BoolArg::True {
                    took_tcs = liquidation
                        .maybe_take_token_conditional_swap(account_addresses.iter())
                        .await
                        .unwrap_or_else(|err| {
                            error!("error during maybe_take_token_conditional_swap: {err}");
                            false
                        })
                }

                if liquidated || took_tcs {
                    rebalance_trigger_sender.send_unless_full(()).unwrap();
                }
            }
        }
    });

    let token_swap_info_job = tokio::spawn({
        let mut interval = mango_v4_client::delay_interval(Duration::from_secs(
            cli.token_swap_refresh_interval_secs,
        ));
        let mut startup_wait = mango_v4_client::delay_interval(Duration::from_secs(1));
        let shared_state = shared_state.clone();
        async move {
            loop {
                if !shared_state.read().unwrap().one_snapshot_done {
                    startup_wait.tick().await;
                    continue;
                }

                interval.tick().await;
                let token_indexes = token_swap_info_updater
                    .mango_client()
                    .context
                    .tokens
                    .keys()
                    .copied()
                    .collect_vec();
                let mut min_delay = mango_v4_client::delay_interval(Duration::from_secs(1));
                for token_index in token_indexes {
                    min_delay.tick().await;
                    token_swap_info_updater.update_one(token_index).await;
                }
                token_swap_info_updater.log_all();
            }
        }
    });

    let check_changes_for_abort_job =
        tokio::spawn(MangoClient::loop_check_for_context_changes_and_abort(
            mango_client.clone(),
            Duration::from_secs(300),
        ));

    if cli.telemetry == BoolArg::True {
        tokio::spawn(telemetry::report_regularly(
            mango_client,
            cli.min_health_ratio,
        ));
    }

    use cli_args::{BoolArg, Cli, CliDotenv};
    use futures::StreamExt;
    let mut jobs: futures::stream::FuturesUnordered<_> = vec![
        data_job,
        rebalance_job,
        liquidation_job,
        token_swap_info_job,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiqErrorType {
    Liq,
    /// Errors that suggest we maybe should skip trying to collect tcs for that pubkey
    TcsCollectionHard,
    /// Recording errors when some tcs have errors during collection but others don't
    TcsCollectionPartial,
    TcsExecution,
}

impl std::fmt::Display for LiqErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Liq => write!(f, "liq"),
            Self::TcsCollectionHard => write!(f, "tcs-collection-hard"),
            Self::TcsCollectionPartial => write!(f, "tcs-collection-partial"),
            Self::TcsExecution => write!(f, "tcs-execution"),
        }
    }
}

struct LiquidationState {
    mango_client: Arc<MangoClient>,
    account_fetcher: Arc<chain_data::AccountFetcher>,
    token_swap_info: Arc<token_swap_info::TokenSwapInfoUpdater>,
    liquidation_config: liquidate::Config,
    trigger_tcs_config: trigger_tcs::Config,

    errors: ErrorTracking<Pubkey, LiqErrorType>,
    oracle_errors: ErrorTracking<TokenIndex, LiqErrorType>,
}

impl LiquidationState {
    async fn maybe_liquidate_one<'b>(
        &mut self,
        accounts_iter: impl Iterator<Item = &'b Pubkey>,
    ) -> bool {
        use rand::seq::SliceRandom;

        let mut accounts = accounts_iter.collect::<Vec<&Pubkey>>();
        {
            let mut rng = rand::thread_rng();
            accounts.shuffle(&mut rng);
        }

        for pubkey in accounts {
            if self
                .maybe_liquidate_and_log_error(pubkey)
                .await
                .unwrap_or(false)
            {
                return true;
            }
        }

        false
    }

    async fn maybe_liquidate_and_log_error(&mut self, pubkey: &Pubkey) -> anyhow::Result<bool> {
        let now = Instant::now();
        let error_tracking = &mut self.errors;

        // Skip a pubkey if there've been too many errors recently
        if let Some(error_entry) =
            error_tracking.had_too_many_errors(LiqErrorType::Liq, pubkey, now)
        {
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
            if let Some((ti, ti_name)) = err.try_unwrap_oracle_error() {
                if self
                    .oracle_errors
                    .had_too_many_errors(LiqErrorType::Liq, &ti, Instant::now())
                    .is_none()
                {
                    warn!(
                        "{:?} recording oracle error for token {} {}",
                        chrono::offset::Utc::now(),
                        ti_name,
                        ti
                    );
                }

                self.oracle_errors
                    .record(LiqErrorType::Liq, &ti, err.to_string());
                return result;
            }

            // Keep track of pubkeys that had errors
            error_tracking.record(LiqErrorType::Liq, pubkey, err.to_string());

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
            error_tracking.clear(LiqErrorType::Liq, pubkey);
        }

        result
    }

    async fn maybe_take_token_conditional_swap(
        &mut self,
        accounts_iter: impl Iterator<Item = &Pubkey>,
    ) -> anyhow::Result<bool> {
        let accounts = accounts_iter.collect::<Vec<&Pubkey>>();

        let now = Instant::now();
        let now_ts: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let tcs_context = trigger_tcs::Context {
            mango_client: self.mango_client.clone(),
            account_fetcher: self.account_fetcher.clone(),
            token_swap_info: self.token_swap_info.clone(),
            config: self.trigger_tcs_config.clone(),
            jupiter_quote_cache: Arc::new(trigger_tcs::JupiterQuoteCache::default()),
            now_ts,
        };

        // Find interesting (pubkey, tcsid, volume)
        let mut interesting_tcs = Vec::with_capacity(accounts.len());
        for pubkey in accounts.iter() {
            if let Some(error_entry) =
                self.errors
                    .had_too_many_errors(LiqErrorType::TcsCollectionHard, pubkey, now)
            {
                trace!(
                    %pubkey,
                    error_entry.count,
                    "skip checking account for tcs, had errors recently",
                );
                continue;
            }

            match tcs_context.find_interesting_tcs_for_account(pubkey) {
                Ok(v) => {
                    self.errors.clear(LiqErrorType::TcsCollectionHard, pubkey);
                    if v.is_empty() {
                        self.errors
                            .clear(LiqErrorType::TcsCollectionPartial, pubkey);
                        self.errors.clear(LiqErrorType::TcsExecution, pubkey);
                    } else if v.iter().all(|it| it.is_ok()) {
                        self.errors
                            .clear(LiqErrorType::TcsCollectionPartial, pubkey);
                    } else {
                        for it in v.iter() {
                            if let Err(e) = it {
                                self.errors.record(
                                    LiqErrorType::TcsCollectionPartial,
                                    pubkey,
                                    e.to_string(),
                                );
                            }
                        }
                    }
                    interesting_tcs.extend(v.iter().filter_map(|it| it.as_ref().ok()));
                }
                Err(e) => {
                    self.errors
                        .record(LiqErrorType::TcsCollectionHard, pubkey, e.to_string());
                }
            }
        }
        if interesting_tcs.is_empty() {
            return Ok(false);
        }

        let (txsigs, mut changed_pubkeys) = tcs_context
            .execute_tcs(&mut interesting_tcs, &mut self.errors)
            .await?;
        for pubkey in changed_pubkeys.iter() {
            self.errors.clear(LiqErrorType::TcsExecution, pubkey);
        }
        if txsigs.is_empty() {
            return Ok(false);
        }
        changed_pubkeys.push(self.mango_client.mango_account_address);

        // Force a refresh of affected accounts
        let slot = self
            .account_fetcher
            .transaction_max_slot(&txsigs)
            .await
            .context("transaction_max_slot")?;
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

        Ok(true)
    }
}

fn start_chain_data_metrics(chain: Arc<RwLock<chain_data::ChainData>>, metrics: &metrics::Metrics) {
    let mut interval = mango_v4_client::delay_interval(Duration::from_secs(600));

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
