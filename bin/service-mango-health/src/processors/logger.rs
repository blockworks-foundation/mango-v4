use crate::configuration::Configuration;
use crate::processors::health::HealthEvent;
use log::warn;
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

                        if component.health_ratio.is_some() {
                            println!(
                                "PUB {:?} {} -> {}%",
                                msg.computed_at,
                                component.account,
                                component.health_ratio.unwrap()
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
