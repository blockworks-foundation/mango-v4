use crate::configuration::Configuration;
use crate::processors::health::HealthEvent;
use log::warn;
use mango_v4::accounts_zerocopy::LoadZeroCopy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct CompositeChannelProcessor<T: Clone + Send> {
    pub job: JoinHandle<()>,

    available_receivers: usize,
    receivers: Vec<async_channel::Receiver<T>>,
}

impl<T: Clone + Send + 'static> CompositeChannelProcessor<T> {
    pub fn get_receiver(&mut self) -> anyhow::Result<async_channel::Receiver<T>> {
        if self.available_receivers == 0 {
            anyhow::bail!("no more receiver available")
        } else {
            self.available_receivers -= 1;
            Ok(self.receivers.pop().unwrap())
        }
    }

    pub async fn init(
        data: async_channel::Receiver<T>,
        count: usize,
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<CompositeChannelProcessor<T>> {
        let mut senders = Vec::new();
        let mut receivers = Vec::new();

        for i in 0..count {
            let (sender, receiver) = async_channel::bounded::<T>(1000);
            senders.push(sender);
            receivers.push(receiver);
        }

        let job = tokio::spawn(async move {
            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down composite channel...");
                    break;
                }
                tokio::select! {
                    Ok(event) = data.recv() => {
                        for sender in &senders {
                            let res = sender.try_send(event.clone());
                                if res.is_err() {
                                    warn!("cannot send update {:?}", res.unwrap_err());
                                    break;
                                }
                        }
                    },
                    Err(e) = data.recv() => {
                        warn!("data update channel err {:?}", e);
                        break;
                    },
                }
            }

            warn!("shutting down composite processor...");
        });

        let result = CompositeChannelProcessor::<T> {
            job,
            available_receivers: count,
            receivers,
        };

        Ok(result)
    }
}
