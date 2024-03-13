use crate::cli_args::Cli;
use crate::metrics::Metrics;
use crate::token_swap_info::TokenSwapInfoUpdater;
use crate::{trigger_tcs, LiqErrorType, SharedState, TxTrigger};
use anchor_lang::prelude::Pubkey;
use anyhow::Context;
use itertools::Itertools;
use mango_v4_client::error_tracking::ErrorTracking;
use mango_v4_client::{chain_data, AsyncChannelSendUnlessFull, MangoClient};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{error, info, trace};

pub fn spawn_tcs_job(
    cli: &Cli,
    shared_state: &Arc<RwLock<SharedState>>,
    tx_trigger_sender: async_channel::Sender<TxTrigger>,
    mut tcs: Box<TcsState>,
    metrics: &Metrics,
) -> JoinHandle<()> {
    tokio::spawn({
        let mut interval =
            mango_v4_client::delay_interval(Duration::from_millis(cli.tcs_check_interval_ms));
        let mut tcs_start_time = None;
        let mut metric_tcs_start_end = metrics.register_latency("tcs_start_end".into());
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

                tcs.errors.write().unwrap().update();

                tcs_start_time = Some(tcs_start_time.unwrap_or(Instant::now()));

                let interesting_tcs = tcs
                    .find_candidate(account_addresses.iter())
                    .await
                    .unwrap_or_else(|err| {
                        error!("error during find_candidate: {err}");
                        Vec::new()
                    });

                if !interesting_tcs.is_empty() {
                    let mut state_writer = shared_state.write().unwrap();
                    let mut n = 0;

                    for i in interesting_tcs {
                        if state_writer.interesting_tcs.insert(i) {
                            n += 1;
                        }
                    }

                    tx_trigger_sender
                        .send_unless_full(TxTrigger::TokenConditionalSwap(n))
                        .unwrap();
                }

                let current_time = Instant::now();
                metric_tcs_start_end.push(current_time - tcs_start_time.unwrap());
                tcs_start_time = None;
            }
        }
    })
}

#[derive(Clone)]
pub struct TcsState {
    pub mango_client: Arc<MangoClient>,
    pub account_fetcher: Arc<chain_data::AccountFetcher>,
    pub token_swap_info: Arc<TokenSwapInfoUpdater>,
    pub trigger_tcs_config: trigger_tcs::Config,

    pub errors: Arc<RwLock<ErrorTracking<Pubkey, LiqErrorType>>>,
}

impl TcsState {
    async fn find_candidate(
        &mut self,
        accounts_iter: impl Iterator<Item = &Pubkey>,
    ) -> anyhow::Result<Vec<(Pubkey, u64, u64)>> {
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
            if let Some(error_entry) = self.errors.read().unwrap().had_too_many_errors(
                LiqErrorType::TcsCollectionHard,
                pubkey,
                now,
            ) {
                trace!(
                    %pubkey,
                    error_entry.count,
                    "skip checking account for tcs, had errors recently",
                );
                continue;
            }

            let candidates = tcs_context.find_interesting_tcs_for_account(pubkey);
            let mut error_guard = self.errors.write().unwrap();

            match candidates {
                Ok(v) => {
                    error_guard.clear(LiqErrorType::TcsCollectionHard, pubkey);
                    if v.is_empty() {
                        error_guard.clear(LiqErrorType::TcsCollectionPartial, pubkey);
                        error_guard.clear(LiqErrorType::TcsExecution, pubkey);
                    } else if v.iter().all(|it| it.is_ok()) {
                        error_guard.clear(LiqErrorType::TcsCollectionPartial, pubkey);
                    } else {
                        for it in v.iter() {
                            if let Err(e) = it {
                                error_guard.record(
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
                    error_guard.record(LiqErrorType::TcsCollectionHard, pubkey, e.to_string());
                }
            }
        }

        return Ok(interesting_tcs);
    }

    pub async fn maybe_take_token_conditional_swap(
        &mut self,
        mut interesting_tcs: Vec<(Pubkey, u64, u64)>,
    ) -> anyhow::Result<bool> {
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

        if interesting_tcs.is_empty() {
            return Ok(false);
        }

        let (txsigs, mut changed_pubkeys) = tcs_context
            .execute_tcs(&mut interesting_tcs, self.errors.clone())
            .await?;
        for pubkey in changed_pubkeys.iter() {
            self.errors
                .write()
                .unwrap()
                .clear(LiqErrorType::TcsExecution, pubkey);
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
                self.trigger_tcs_config.refresh_timeout,
            )
            .await
        {
            info!(slot, "could not refresh after tcs execution: {}", e);
        }

        Ok(true)
    }
}
