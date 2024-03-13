use crate::cli_args::Cli;
use crate::metrics::Metrics;
use crate::unwrappable_oracle_error::UnwrappableOracleError;
use crate::{liquidate, LiqErrorType, SharedState};
use anchor_lang::prelude::Pubkey;
use itertools::Itertools;
use mango_v4::state::TokenIndex;
use mango_v4_client::error_tracking::ErrorTracking;
use mango_v4_client::{chain_data, MangoClient, MangoClientError};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{error, trace, warn};

#[derive(Clone)]
pub struct LiquidationState {
    pub mango_client: Arc<MangoClient>,
    pub account_fetcher: Arc<chain_data::AccountFetcher>,
    pub liquidation_config: liquidate::Config,

    pub errors: Arc<RwLock<ErrorTracking<Pubkey, LiqErrorType>>>,
    pub oracle_errors: Arc<RwLock<ErrorTracking<TokenIndex, LiqErrorType>>>,
}

impl LiquidationState {
    async fn find_candidates(
        &mut self,
        accounts_iter: impl Iterator<Item = &Pubkey>,
        action: impl Fn(Pubkey) -> anyhow::Result<()>,
    ) -> anyhow::Result<u64> {
        let mut found_counter = 0u64;
        use rand::seq::SliceRandom;

        let mut accounts = accounts_iter.collect::<Vec<&Pubkey>>();
        {
            let mut rng = rand::thread_rng();
            accounts.shuffle(&mut rng);
        }

        for pubkey in accounts {
            if self.should_skip_execution(pubkey) {
                continue;
            }

            let result =
                liquidate::can_liquidate_account(&self.mango_client, &self.account_fetcher, pubkey)
                    .await;

            self.log_or_ignore_error(&result, pubkey);

            if result.unwrap_or(false) {
                action(*pubkey)?;
                found_counter = found_counter + 1;
            }
        }

        Ok(found_counter)
    }

    fn should_skip_execution(&mut self, pubkey: &Pubkey) -> bool {
        let now = Instant::now();
        let error_tracking = &mut self.errors;

        // Skip a pubkey if there've been too many errors recently
        if let Some(error_entry) =
            error_tracking
                .read()
                .unwrap()
                .had_too_many_errors(LiqErrorType::Liq, pubkey, now)
        {
            trace!(
                %pubkey,
                error_entry.count,
                "skip checking account for liquidation, had errors recently",
            );
            return true;
        }

        false
    }

    fn log_or_ignore_error<T>(&mut self, result: &anyhow::Result<T>, pubkey: &Pubkey) {
        let error_tracking = &mut self.errors;

        if let Err(err) = result.as_ref() {
            if let Some((ti, ti_name)) = err.try_unwrap_oracle_error() {
                if self
                    .oracle_errors
                    .read()
                    .unwrap()
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
                    .write()
                    .unwrap()
                    .record(LiqErrorType::Liq, &ti, err.to_string());
                return;
            }

            // Keep track of pubkeys that had errors
            error_tracking
                .write()
                .unwrap()
                .record(LiqErrorType::Liq, pubkey, err.to_string());

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
            error_tracking
                .write()
                .unwrap()
                .clear(LiqErrorType::Liq, pubkey);
        }
    }

    pub async fn maybe_liquidate_and_log_error(&mut self, pubkey: &Pubkey) -> anyhow::Result<bool> {
        if self.should_skip_execution(pubkey) {
            return Ok(false);
        }

        let result = liquidate::maybe_liquidate_account(
            &self.mango_client,
            &self.account_fetcher,
            pubkey,
            &self.liquidation_config,
        )
        .await;

        self.log_or_ignore_error(&result, pubkey);
        return result;
    }
}

pub fn spawn_liquidation_job(
    cli: &Cli,
    shared_state: &Arc<RwLock<SharedState>>,
    tx_trigger_sender: async_channel::Sender<()>,
    mut liquidation: Box<LiquidationState>,
    metrics: &Metrics,
) -> JoinHandle<()> {
    tokio::spawn({
        let mut interval =
            mango_v4_client::delay_interval(Duration::from_millis(cli.check_interval_ms));
        let mut metric_liquidation_check = metrics.register_latency("liquidation_check".into());
        let mut metric_liquidation_start_end =
            metrics.register_latency("liquidation_start_end".into());

        let mut liquidation_start_time = None;

        let shared_state = shared_state.clone();
        async move {
            loop {
                interval.tick().await;

                let account_addresses = {
                    let mut state = shared_state.write().unwrap();
                    if !state.one_snapshot_done {
                        // discard first latency info as it will skew data too much
                        state.oldest_chain_event_reception_time = None;
                        continue;
                    }
                    if state.oldest_chain_event_reception_time.is_none()
                        && liquidation_start_time.is_none()
                    {
                        // no new update, skip computing
                        continue;
                    }

                    state.mango_accounts.iter().cloned().collect_vec()
                };

                liquidation.errors.write().unwrap().update();
                liquidation.oracle_errors.write().unwrap().update();

                if liquidation_start_time.is_none() {
                    liquidation_start_time = Some(Instant::now());
                }

                let found_candidates = liquidation
                    .find_candidates(account_addresses.iter(), |p| {
                        if shared_state
                            .write()
                            .unwrap()
                            .liquidation_candidates_accounts
                            .insert(p)
                        {
                            tx_trigger_sender.try_send(())?;
                        }

                        Ok(())
                    })
                    .await
                    .unwrap();

                if found_candidates > 0 {
                    tracing::debug!("found {} candidates for liquidation", found_candidates);
                }

                let mut state = shared_state.write().unwrap();
                let reception_time = state.oldest_chain_event_reception_time.unwrap();
                let current_time = Instant::now();

                state.oldest_chain_event_reception_time = None;

                metric_liquidation_check.push(current_time - reception_time);
                metric_liquidation_start_end.push(current_time - liquidation_start_time.unwrap());
                liquidation_start_time = None;
            }
        }
    })
}
