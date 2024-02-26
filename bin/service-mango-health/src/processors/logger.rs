use crate::configuration::Configuration;
use crate::processors::health::{HealthComponentValue, HealthEvent};
use tracing::warn;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct LoggerProcessor {
    pub job: JoinHandle<()>,
}

impl LoggerProcessor {
    pub async fn init(
        data_sender: &tokio::sync::broadcast::Sender<HealthEvent>,
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<LoggerProcessor> {
        let mut data = data_sender.subscribe();
        let filter: HashSet<Pubkey> = configuration
            .logging_configuration
            .log_health_for_accounts
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|s| Pubkey::from_str(s).unwrap())
            .collect();
        let enable_logging = configuration.logging_configuration.log_health_to_stdout;

        let job = tokio::spawn(async move {
            if !enable_logging {
                return;
            }

            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down logger processor...");
                    break;
                }
                if let Ok(msg) = data.recv().await {
                    for component in msg.components {
                        if !filter.is_empty() && !filter.contains(&component.account) {
                            continue;
                        }

                        if component.value.is_some() {
                            let value: HealthComponentValue = component.value.unwrap();

                            println!(
                                "PUB {:?} {} -> {}% (init: {}, maint: {}, liquidationEnd: {}, beingLiquidated: {})",
                                msg.computed_at,
                                component.account,
                                value.maintenance_ratio,
                                value.initial_health,
                                value.maintenance_health,
                                value.liquidation_end_health,
                                value.is_being_liquidated as u8,
                            );
                        } else {
                            println!("PUB {:?} {} -> ERROR%", msg.computed_at, component.account);
                        }
                    }
                }
            }
        });

        let result = LoggerProcessor { job };

        Ok(result)
    }
}
