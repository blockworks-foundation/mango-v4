use crate::configuration::Configuration;
use crate::processors::data::DataEvent;
use crate::processors::health::HealthEvent;
use log::warn;
use mango_v4::accounts_zerocopy::LoadZeroCopy;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct PublisherProcessor {
    pub job: JoinHandle<()>,
}

impl PublisherProcessor {
    pub async fn init(
        data: async_channel::Receiver<HealthEvent>,
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<PublisherProcessor> {
        let job = tokio::spawn(async move {
            // TODO FAS
            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down publisher processor...");
                    break;
                }
                if let Ok(msg) = data.recv().await {
                    for component in msg.components {
                        if component.account
                            == Pubkey::from_str("6DjRccWB5Ydj6rspczf3YGGHt5ESEmLL4jS6GYCe2ZQL")
                                .unwrap()
                        {
                            println!(
                                "PUB {:?} {} -> {}%",
                                msg.computed_at, component.account, component.health_ratio
                            );
                        }
                    }
                }
            }
        });

        let result = PublisherProcessor { job };

        Ok(result)
    }
}
