use crate::configuration::Configuration;
use crate::processors::health::{HealthComponentValue, HealthEvent};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

pub struct LoggerProcessor {
    pub job: JoinHandle<()>,
}

impl LoggerProcessor {
    pub async fn init(
        data_sender: &tokio::sync::broadcast::Sender<HealthEvent>,
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<Option<LoggerProcessor>> {
        let enable_logging = configuration.logging_configuration.log_health_to_stdout;
        if !enable_logging {
            return Ok(None);
        }

        let mut data = data_sender.subscribe();
        let filter: HashSet<Pubkey> = configuration
            .logging_configuration
            .log_health_for_accounts
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|s| Pubkey::from_str(s).unwrap())
            .collect();

        let job = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(1000));
            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down logger processor...");
                    break;
                }
                tokio::select! {
                    _ = interval.tick() => {
                    },
                    Ok(msg) = data.recv() => {
                        for component in msg.components {
                            if !filter.is_empty() && !filter.contains(&component.account) {
                                continue;
                            }

                            if component.value.is_some() {
                                let value: HealthComponentValue = component.value.unwrap();

                                info!(
                                    computed_at = %msg.computed_at,
                                    account = %component.account,
                                    maintenance_ratio = %value.maintenance_ratio,
                                    initial_health = %value.initial_health,
                                    maintenance_health = %value.maintenance_health,
                                    liquidation_end_health = %value.liquidation_end_health,
                                    is_being_liquidated = %value.is_being_liquidated,
                                )
                            } else {
                                info!(
                                computed_at = %msg.computed_at,
                                account = %component.account,
                                error = "Missing health data"
                                )
                            }
                        }
                    },
                }
            }
        });

        let result = LoggerProcessor { job };

        Ok(Some(result))
    }
}
