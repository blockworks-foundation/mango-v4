use crate::configuration::Configuration;
use crate::processors::data::DataEvent;
use crate::processors::health::HealthEvent;
use log::warn;
use mango_v4::accounts_zerocopy::LoadZeroCopy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct PersisterProcessor {
    pub job: JoinHandle<()>,
}

impl PersisterProcessor {
    pub async fn init(
        data: async_channel::Receiver<HealthEvent>,
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<PersisterProcessor> {
        let job = tokio::spawn(async move {
            // TODO FAS
            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down persister processor...");
                    break;
                }

                data.recv().await;
            }
        });

        let result = PersisterProcessor { job };

        Ok(result)
    }
}
