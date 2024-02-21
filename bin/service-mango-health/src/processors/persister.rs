use crate::configuration::Configuration;
use crate::processors::health::HealthEvent;
use log::warn;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct PersisterProcessor {
    pub job: JoinHandle<()>,
}

impl PersisterProcessor {
    pub async fn init(
        data_sender: &tokio::sync::broadcast::Sender<HealthEvent>,
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<PersisterProcessor> {
        let mut data = data_sender.subscribe();

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
