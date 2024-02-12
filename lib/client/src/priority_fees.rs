use futures::{SinkExt, StreamExt};
use jsonrpc_core::{MethodCall, Notification, Params, Version};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::*;

pub trait PriorityFeeProvider: Sync + Send {
    fn compute_unit_fee_microlamports(&self) -> u64;
}

pub struct FixedPriorityFeeProvider {
    pub compute_unit_fee_microlamports: u64,
}

impl FixedPriorityFeeProvider {
    pub fn new(fee_microlamports: u64) -> Self {
        Self {
            compute_unit_fee_microlamports: fee_microlamports,
        }
    }
}

impl PriorityFeeProvider for FixedPriorityFeeProvider {
    fn compute_unit_fee_microlamports(&self) -> u64 {
        self.compute_unit_fee_microlamports
    }
}

#[derive(Builder)]
pub struct EmaPriorityFeeProviderConfig {
    pub percentile: u8,

    #[builder(default = "0.2")]
    pub alpha: f64,

    pub fallback_prio: u64,

    #[builder(default = "Duration::from_secs(15)")]
    pub max_age: Duration,
}

impl EmaPriorityFeeProviderConfig {
    pub fn builder() -> EmaPriorityFeeProviderConfigBuilder {
        EmaPriorityFeeProviderConfigBuilder::default()
    }
}

#[derive(Default)]
struct CuPercentileEmaPriorityFeeProviderData {
    ema: f64,
    last_update: Option<Instant>,
}

pub struct CuPercentileEmaPriorityFeeProvider {
    data: RwLock<CuPercentileEmaPriorityFeeProviderData>,
    config: EmaPriorityFeeProviderConfig,
}

impl PriorityFeeProvider for CuPercentileEmaPriorityFeeProvider {
    fn compute_unit_fee_microlamports(&self) -> u64 {
        let data = self.data.read().unwrap();
        if let Some(last_update) = data.last_update {
            if Instant::now().duration_since(last_update) > self.config.max_age {
                return self.config.fallback_prio;
            }
        } else {
            return self.config.fallback_prio;
        }
        data.ema as u64
    }
}

impl CuPercentileEmaPriorityFeeProvider {
    pub fn run(
        config: EmaPriorityFeeProviderConfig,
        sender: &broadcast::Sender<BlockPrioFees>,
    ) -> (Arc<Self>, JoinHandle<()>) {
        let this = Arc::new(Self {
            data: Default::default(),
            config,
        });
        let handle = tokio::spawn({
            let this_c = this.clone();
            let rx = sender.subscribe();
            async move { Self::run_update_job(this_c, rx).await }
        });
        (this, handle)
    }

    async fn run_update_job(provider: Arc<Self>, mut rx: broadcast::Receiver<BlockPrioFees>) {
        let config = &provider.config;
        loop {
            let block_prios = rx.recv().await.unwrap();
            let prio = match block_prios.by_cu_percentile.get(&config.percentile) {
                Some(v) => *v as f64,
                None => {
                    error!("percentile not available: {}", config.percentile);
                    continue;
                }
            };

            let mut data = provider.data.write().unwrap();
            data.ema = data.ema * (1.0 - config.alpha) + config.alpha * prio;
            data.last_update = Some(Instant::now());
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct BlockPrioFees {
    pub slot: u64,
    // prio fee percentile in percent -> prio fee
    pub percentile: HashMap<u8, u64>,
    // cu percentile in percent -> median prio fee of the group
    pub by_cu_percentile: HashMap<u8, u64>,
}

#[derive(serde::Deserialize)]
struct BlockPrioritizationFeesNotificationContext {
    slot: u64,
}

#[derive(serde::Deserialize)]
struct BlockPrioritizationFeesNotificationValue {
    by_tx: Vec<u64>,
    by_tx_percentiles: Vec<f64>,
    by_cu: Vec<u64>,
    by_cu_percentiles: Vec<f64>,
}

#[derive(serde::Deserialize)]
struct BlockPrioritizationFeesNotificationParams {
    context: BlockPrioritizationFeesNotificationContext,
    value: BlockPrioritizationFeesNotificationValue,
}

fn as_block_prioritization_fees_notification(
    notification_str: &str,
) -> anyhow::Result<Option<BlockPrioFees>> {
    let notification: Notification = match serde_json::from_str(&notification_str) {
        Ok(v) => v,
        Err(_) => return Ok(None), // not a notification at all
    };
    if notification.method != "blockPrioritizationFeesNotification" {
        return Ok(None);
    }
    let map = match notification.params {
        Params::Map(m) => m,
        _ => anyhow::bail!("unexpected params, expected map"),
    };
    let result = map
        .get("result")
        .ok_or(anyhow::anyhow!("missing params.result"))?
        .clone();

    let mut data = BlockPrioFees::default();
    let v: BlockPrioritizationFeesNotificationParams = serde_json::from_value(result)?;
    data.slot = v.context.slot;
    for (percentile, prio) in v.value.by_tx_percentiles.iter().zip(v.value.by_tx.iter()) {
        let int_perc: u8 = ((percentile * 100.0) as u64).try_into()?;
        data.percentile.insert(int_perc, *prio);
    }
    for (percentile, prio) in v.value.by_cu_percentiles.iter().zip(v.value.by_cu.iter()) {
        let int_perc: u8 = ((percentile * 100.0) as u64).try_into()?;
        data.by_cu_percentile.insert(int_perc, *prio);
    }

    Ok(Some(data))
}

async fn connect_and_broadcast(
    url: &str,
    sender: &broadcast::Sender<BlockPrioFees>,
) -> anyhow::Result<()> {
    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Create a JSON-RPC request
    let call = MethodCall {
        jsonrpc: Some(Version::V2),
        method: "blockPrioritizationFeesSubscribe".to_string(),
        params: Params::None,
        id: jsonrpc_core::Id::Num(1),
    };

    let request = serde_json::to_string(&call).unwrap();
    write.send(Message::Text(request)).await?;

    loop {
        let timeout = tokio::time::sleep(Duration::from_secs(20));
        tokio::select! {
            message = read.next() => {
            match message {
                Some(Ok(Message::Text(text))) => {
                    if let Some(block_prio) = as_block_prioritization_fees_notification(&text)? {
                        // Failure might just mean there is no receiver right now
                        let _ = sender.send(block_prio);
                    }
                }
                Some(Ok(Message::Ping(..))) => {}
                Some(Ok(Message::Pong(..))) => {}
                Some(Ok(msg @ _)) => {
                    anyhow::bail!("received a non-text message: {:?}", msg);
                },
                Some(Err(e)) => {
                    anyhow::bail!("error receiving message: {}", e);
                }
                None => {
                    anyhow::bail!("websocket stream closed");
                }
            }
            },
            _ = timeout => {
                anyhow::bail!("timeout");
            }
        }
    }
}

async fn connect_and_broadcast_loop(url: &str, sender: broadcast::Sender<BlockPrioFees>) {
    loop {
        if let Err(err) = connect_and_broadcast(url, &sender).await {
            info!("recent block prio feed error, restarting: {err:?}");
        }
    }
}

pub fn run_broadcast_from_websocket_feed(
    url: String,
) -> (broadcast::Sender<BlockPrioFees>, JoinHandle<()>) {
    let (sender, _) = broadcast::channel(10);
    let sender_c = sender.clone();
    let handle = tokio::spawn(async move { connect_and_broadcast_loop(&url, sender_c).await });
    (sender, handle)
}
