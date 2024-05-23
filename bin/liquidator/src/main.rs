use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anchor_client::Cluster;
use clap::Parser;
use futures_util::StreamExt;
use mango_v4::state::{PerpMarketIndex, TokenIndex};
use mango_v4_client::{
    account_update_stream, chain_data, error_tracking::ErrorTracking, keypair_from_cli,
    snapshot_source, websocket_source, Client, MangoClient, MangoGroupContext,
    TransactionBuilderConfig,
};

use crate::cli_args::{BoolArg, Cli, CliDotenv};
use crate::liquidation_state::LiquidationState;
use crate::rebalance::Rebalancer;
use crate::tcs_state::TcsState;
use crate::token_swap_info::TokenSwapInfoUpdater;
use itertools::Itertools;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use tokio::task::JoinHandle;
use tracing::*;

pub mod cli_args;
pub mod liquidate;
mod liquidation_state;
pub mod metrics;
pub mod rebalance;
mod tcs_state;
pub mod telemetry;
pub mod token_swap_info;
pub mod trigger_tcs;
mod tx_sender;
mod unwrappable_oracle_error;
pub mod util;

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
    mango_v4_client::print_git_version();

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
    let rpc_url = cli.rpc_url.clone();
    let ws_url = rpc_url.replace("https", "wss");
    let rpc_timeout = Duration::from_secs(10);
    let cluster = Cluster::Custom(rpc_url.clone(), ws_url.clone());
    let commitment = CommitmentConfig::processed();
    let client = Client::builder()
        .cluster(cluster.clone())
        .commitment(commitment)
        .fee_payer(Some(liqor_owner.clone()))
        .timeout(rpc_timeout)
        .jupiter_timeout(Duration::from_secs(cli.jupiter_timeout_secs))
        .jupiter_v6_url(cli.jupiter_v6_url.clone())
        .jupiter_token(cli.jupiter_token.clone())
        .sanctum_url(cli.sanctum_url.clone())
        .sanctum_timeout(Duration::from_secs(cli.sanctum_timeout_secs))
        .transaction_builder_config(
            TransactionBuilderConfig::builder()
                .priority_fee_provider(prio_provider)
                // Liquidation and tcs triggers set their own budgets, this is a default for other tx
                .compute_budget_per_instruction(Some(250_000))
                .build()
                .unwrap(),
        )
        .override_send_transaction_urls(cli.override_send_transaction_url.clone())
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
        .flat_map(|value| {
            [
                value.oracle,
                value.fallback_context.key,
                value.fallback_context.quote_key,
            ]
        })
        .chain(group_context.perp_markets.values().map(|p| p.oracle))
        .unique()
        .filter(|&k| k != Pubkey::default())
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
    let snapshot_job = snapshot_source::start(
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
        only_allowed_tokens: cli_args::cli_to_hashset::<TokenIndex>(cli.only_allow_tokens.clone()),
        forbidden_tokens: cli_args::cli_to_hashset::<TokenIndex>(cli.forbidden_tokens.clone()),
        only_allowed_perp_markets: cli_args::cli_to_hashset::<PerpMarketIndex>(
            cli.liquidation_only_allow_perp_markets.clone(),
        ),
        forbidden_perp_markets: cli_args::cli_to_hashset::<PerpMarketIndex>(
            cli.liquidation_forbidden_perp_markets.clone(),
        ),
    };

    let tcs_config = trigger_tcs::Config {
        refresh_timeout: Duration::from_secs(cli.tcs_refresh_timeout_secs),
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

    let (rebalance_trigger_sender, rebalance_trigger_receiver) = async_channel::bounded::<()>(1);
    let (tx_tcs_trigger_sender, tx_tcs_trigger_receiver) = async_channel::unbounded::<()>();
    let (tx_liq_trigger_sender, tx_liq_trigger_receiver) = async_channel::unbounded::<()>();

    if cli.rebalance_using_limit_order == BoolArg::True && !signer_is_owner {
        warn!("Can't withdraw dust to liqor account if delegate and using limit orders for rebalancing");
    }

    let rebalance_config = rebalance::Config {
        enabled: cli.rebalance == BoolArg::True,
        slippage_bps: cli.rebalance_slippage_bps,
        borrow_settle_excess: (1f64 + cli.rebalance_borrow_settle_excess).max(1f64),
        refresh_timeout: Duration::from_secs(cli.rebalance_refresh_timeout_secs),
        jupiter_version: cli.jupiter_version.into(),
        skip_tokens: cli.rebalance_skip_tokens.clone().unwrap_or(Vec::new()),
        alternate_jupiter_route_tokens: cli
            .rebalance_alternate_jupiter_route_tokens
            .clone()
            .unwrap_or_default(),
        alternate_sanctum_route_tokens: cli
            .rebalance_alternate_sanctum_route_tokens
            .clone()
            .unwrap_or_default(),
        allow_withdraws: cli.rebalance_using_limit_order == BoolArg::False || signer_is_owner,
        use_sanctum: cli.sanctum_enabled == BoolArg::True,
        use_limit_order: cli.rebalance_using_limit_order == BoolArg::True,
        limit_order_distance_from_oracle_price_bps: cli
            .rebalance_limit_order_distance_from_oracle_price_bps,
    };
    rebalance_config.validate(&mango_client.context);

    let mut rebalancer = rebalance::Rebalancer {
        mango_client: mango_client.clone(),
        account_fetcher: account_fetcher.clone(),
        mango_account_address: cli.liqor_mango_account,
        config: rebalance_config,
        sanctum_supported_mints: HashSet::<Pubkey>::new(),
    };

    let live_rpc_client = mango_client.client.new_rpc_async();
    rebalancer.init(&live_rpc_client).await;
    let rebalancer = Arc::new(rebalancer);

    let liquidation = Box::new(LiquidationState {
        mango_client: mango_client.clone(),
        account_fetcher: account_fetcher.clone(),
        liquidation_config: liq_config,
        errors: Arc::new(RwLock::new(
            ErrorTracking::builder()
                .skip_threshold(2)
                .skip_threshold_for_type(LiqErrorType::Liq, 5)
                .skip_duration(Duration::from_secs(120))
                .build()?,
        )),
        oracle_errors: Arc::new(RwLock::new(
            ErrorTracking::builder()
                .skip_threshold(1)
                .skip_duration(Duration::from_secs(
                    cli.skip_oracle_error_in_logs_duration_secs,
                ))
                .build()?,
        )),
    });

    let tcs = Box::new(TcsState {
        mango_client: mango_client.clone(),
        account_fetcher,
        trigger_tcs_config: tcs_config,
        token_swap_info: token_swap_info_updater.clone(),
        errors: Arc::new(RwLock::new(
            ErrorTracking::builder()
                .skip_threshold(2)
                .skip_threshold_for_type(LiqErrorType::Liq, 5)
                .skip_duration(Duration::from_secs(120))
                .build()?,
        )),
    });

    info!("main loop");

    // Job to update chain_data and notify the liquidation job when a new check is needed.
    let data_job = tokio::spawn({
        use account_update_stream::Message;

        let shared_state = shared_state.clone();

        let mut metric_account_update_queue_len =
            metrics.register_u64("account_update_queue_length".into());
        let mut metric_chain_update_latency =
            metrics.register_latency("in-memory chain update".into());
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
                let current_time = Instant::now();
                metric_account_update_queue_len.set(account_update_receiver.len() as u64);

                message.update_chain_data(&mut chain_data.write().unwrap());

                match message {
                    Message::Account(account_write) => {
                        let mut state = shared_state.write().unwrap();
                        let reception_time = account_write.reception_time;
                        state.oldest_chain_event_reception_time = Some(
                            state
                                .oldest_chain_event_reception_time
                                .unwrap_or(reception_time),
                        );

                        metric_chain_update_latency.push(current_time - reception_time);

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
                        let mut reception_time = None;

                        // Track all mango account pubkeys
                        for update in snapshot.iter() {
                            reception_time = Some(
                                update
                                    .reception_time
                                    .min(reception_time.unwrap_or(update.reception_time)),
                            );
                            state.oldest_chain_event_reception_time = Some(
                                state
                                    .oldest_chain_event_reception_time
                                    .unwrap_or(update.reception_time),
                            );

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

                        if reception_time.is_some() {
                            metric_chain_update_latency
                                .push(current_time - reception_time.unwrap());
                        }
                        metric_mango_accounts.set(state.mango_accounts.len() as u64);

                        state.one_snapshot_done = true;
                    }
                    _ => {}
                }
            }
        }
    });

    let mut optional_jobs = vec![];

    // Could be refactored to only start the below jobs when the first snapshot is done.
    // But need to take care to abort if the above job aborts beforehand.
    if cli.rebalance == BoolArg::True {
        let rebalance_job =
            spawn_rebalance_job(shared_state.clone(), rebalance_trigger_receiver, rebalancer);
        optional_jobs.push(rebalance_job);
    }

    if cli.liquidation_enabled == BoolArg::True {
        let liquidation_job = liquidation_state::spawn_liquidation_job(
            &cli,
            &shared_state,
            tx_liq_trigger_sender.clone(),
            liquidation.clone(),
            &metrics,
        );
        optional_jobs.push(liquidation_job);
    }

    if cli.take_tcs == BoolArg::True {
        let tcs_job = tcs_state::spawn_tcs_job(
            &cli,
            &shared_state,
            tx_tcs_trigger_sender.clone(),
            tcs.clone(),
            &metrics,
        );
        optional_jobs.push(tcs_job);
    }

    if cli.liquidation_enabled == BoolArg::True || cli.take_tcs == BoolArg::True {
        let mut tx_sender_jobs = tx_sender::spawn_tx_senders_job(
            cli.max_parallel_operations,
            cli.liquidation_enabled == BoolArg::True,
            tx_liq_trigger_receiver,
            tx_tcs_trigger_receiver,
            tx_tcs_trigger_sender,
            rebalance_trigger_sender,
            shared_state.clone(),
            liquidation,
            tcs,
        );
        optional_jobs.append(&mut tx_sender_jobs);
    }

    if cli.telemetry == BoolArg::True {
        optional_jobs.push(spawn_telemetry_job(&cli, mango_client.clone()));
    }

    let token_swap_info_job =
        spawn_token_swap_refresh_job(&cli, shared_state, token_swap_info_updater);
    let check_changes_for_abort_job = spawn_context_change_watchdog_job(mango_client.clone());

    let mut jobs: futures::stream::FuturesUnordered<_> = vec![
        snapshot_job,
        data_job,
        token_swap_info_job,
        check_changes_for_abort_job,
    ]
    .into_iter()
    .chain(optional_jobs)
    .chain(prio_jobs.into_iter())
    .collect();
    jobs.next().await;

    error!("a critical job aborted, exiting");
    Ok(())
}

fn spawn_token_swap_refresh_job(
    cli: &Cli,
    shared_state: Arc<RwLock<SharedState>>,
    token_swap_info_updater: Arc<TokenSwapInfoUpdater>,
) -> JoinHandle<()> {
    tokio::spawn({
        let mut interval = mango_v4_client::delay_interval(Duration::from_secs(
            cli.token_swap_refresh_interval_secs,
        ));
        let mut startup_wait = mango_v4_client::delay_interval(Duration::from_secs(1));
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
    })
}

fn spawn_context_change_watchdog_job(mango_client: Arc<MangoClient>) -> JoinHandle<()> {
    tokio::spawn(MangoClient::loop_check_for_context_changes_and_abort(
        mango_client,
        Duration::from_secs(300),
    ))
}

fn spawn_telemetry_job(cli: &Cli, mango_client: Arc<MangoClient>) -> JoinHandle<()> {
    tokio::spawn(telemetry::report_regularly(
        mango_client,
        cli.min_health_ratio,
    ))
}

fn spawn_rebalance_job(
    shared_state: Arc<RwLock<SharedState>>,
    rebalance_trigger_receiver: async_channel::Receiver<()>,
    rebalancer: Arc<Rebalancer>,
) -> JoinHandle<()> {
    let mut rebalance_interval = tokio::time::interval(Duration::from_secs(30));

    tokio::spawn({
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

                    // TODO FAS Are there other scenario where this sleep is useful ?
                    // Workaround: We really need a sequence enforcer in the liquidator since we don't want to
                    // accidentally send a similar tx again when we incorrectly believe an earlier one got forked
                    // off. For now, hard sleep on error to avoid the most frequent error cases.
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
    })
}

#[derive(Default)]
pub struct SharedState {
    /// Addresses of the MangoAccounts belonging to the mango program.
    /// Needed to check health of them all when the cache updates.
    mango_accounts: HashSet<Pubkey>,

    /// Is the first snapshot done? Only start checking account health when it is.
    one_snapshot_done: bool,

    /// Oldest chain event not processed yet
    oldest_chain_event_reception_time: Option<Instant>,

    /// Liquidation candidates (locally identified as liquidatable)
    liquidation_candidates_accounts: indexmap::set::IndexSet<Pubkey>,

    /// Interesting TCS that should be triggered
    interesting_tcs: indexmap::set::IndexSet<(Pubkey, u64, u64)>,

    /// Liquidation currently being processed by a worker
    processing_liquidation: HashSet<Pubkey>,

    // TCS currently being processed by a worker
    processing_tcs: HashSet<(Pubkey, u64, u64)>,
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
