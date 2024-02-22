use hdrhistogram::Histogram;
use std::time::Duration;
use {
    std::collections::HashMap,
    std::sync::{atomic, Arc, Mutex, RwLock},
    tokio::time,
    tracing::*,
};

#[derive(Debug)]
enum Value {
    U64(Arc<atomic::AtomicU64>),
    I64(Arc<atomic::AtomicI64>),
    String(Arc<Mutex<String>>),
    Latency(Arc<Mutex<Histogram<u64>>>),
use prometheus::{Encoder, IntCounter, IntGauge, Registry};
use warp::Filter;

lazy_static::lazy_static! {
    pub static ref METRICS_REGISTRY: Registry = Registry::new_custom(Some("liquidator".to_string()), None).unwrap();
    pub static ref METRIC_TOTAL_LIQUIDATION_TRANSACTIONS: IntCounter =
        IntCounter::new("total_liquidation_transactions", "Total liquidation transactions").unwrap();
    pub static ref METRIC_TOTAL_TOKEN_LIQ_WITH_TOKEN_TRANSACTIONS: IntCounter =
        IntCounter::new("total_token_liq_with_token_transactions", "Total token_liq_with_token transactions").unwrap();
    pub static ref METRIC_TOTAL_TOKEN_LIQ_BANKRUPTCY_TRANSACTIONS: IntCounter =
        IntCounter::new("total_token_liq_bankruptcy_transactions", "Total token_liq_bankruptcy transactions").unwrap();
    pub static ref METRIC_TOTAL_PERP_LIQ_BASE_OR_POS_PNL_TRANSACTIONS: IntCounter =
        IntCounter::new("total_perp_liq_base_or_positive_pnl_transactions", "Total perp_liq_base_or_positive_pnl transactions").unwrap();
    pub static ref METRIC_TOTAL_PERP_LIQ_NEG_PNL_OR_BANKRUPTCY_TRANSACTIONS: IntCounter =
        IntCounter::new("total_perp_liq_base_or_positive_pnl_transactions", "Total perp_liq_negative_pnl_or_bankruptcy transactions").unwrap();
    pub static ref METRIC_CHAIN_DATA_SLOTS: IntGauge =
        IntGauge::new("chain_data_slots_count", "chain_data_slots_count").unwrap();
    pub static ref METRIC_CHAIN_DATA_ACCOUNTS: IntGauge =
        IntGauge::new("chain_data_accounts_count", "chain_data_accounts_count").unwrap();
    pub static ref METRIC_CHAIN_DATA_ACCOUNT_WRITE: IntGauge =
        IntGauge::new("chain_data_account_write_count", "chain_data_account_write_count").unwrap();
    pub static ref METRIC_ACCOUNT_UPDATE_QUEUE_LEN: IntGauge = 
        IntGauge::new("account_update_queue_len", "account_update_queue_len").unwrap();
    pub static ref METRIC_MANGO_ACCOUNTS: IntGauge = 
        IntGauge::new("mango_accounts", "mango_accounts").unwrap();
}

pub async fn serve_metrics() {
    METRICS_REGISTRY
        .register(Box::new(METRIC_TOTAL_LIQUIDATION_TRANSACTIONS.clone()))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(
            METRIC_TOTAL_TOKEN_LIQ_WITH_TOKEN_TRANSACTIONS.clone(),
        ))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(
            METRIC_TOTAL_TOKEN_LIQ_BANKRUPTCY_TRANSACTIONS.clone(),
        ))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(
            METRIC_TOTAL_PERP_LIQ_BASE_OR_POS_PNL_TRANSACTIONS.clone(),
        ))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(
            METRIC_TOTAL_PERP_LIQ_NEG_PNL_OR_BANKRUPTCY_TRANSACTIONS.clone(),
        ))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(METRIC_CHAIN_DATA_SLOTS.clone()))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(METRIC_CHAIN_DATA_ACCOUNTS.clone()))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(METRIC_CHAIN_DATA_ACCOUNT_WRITE.clone()))
        .unwrap();

    let metrics_route = warp::path!("metrics").map(|| {
        let mut buffer = Vec::<u8>::new();
        let encoder = prometheus::TextEncoder::new();
        encoder
            .encode(&METRICS_REGISTRY.gather(), &mut buffer)
            .unwrap();

    pub fn set(&mut self, value: u64) {
        self.value.store(value, atomic::Ordering::Release);
    }

    pub fn set_max(&mut self, value: u64) {
        self.value.fetch_max(value, atomic::Ordering::AcqRel);
    }

    pub fn add(&mut self, value: u64) {
        self.value.fetch_add(value, atomic::Ordering::AcqRel);
    }

    pub fn increment(&mut self) {
        self.value.fetch_add(1, atomic::Ordering::AcqRel);
    }

    pub fn decrement(&mut self) {
        self.value.fetch_sub(1, atomic::Ordering::AcqRel);
    }
}

#[derive(Clone)]
pub struct MetricLatency {
    value: Arc<Mutex<Histogram<u64>>>,
}
impl MetricLatency {
    pub fn push(&mut self, duration: std::time::Duration) {
        let mut guard = self.value.lock().unwrap();
        let ns: u64 = duration.as_nanos().try_into().unwrap();
        guard.record(ns).expect("latency error");
    }
}

#[derive(Clone)]
pub struct MetricI64 {
    value: Arc<atomic::AtomicI64>,
}
impl MetricI64 {
    pub fn set(&mut self, value: i64) {
        self.value.store(value, atomic::Ordering::Release);
    }

    pub fn increment(&mut self) {
        self.value.fetch_add(1, atomic::Ordering::AcqRel);
    }

    pub fn decrement(&mut self) {
        self.value.fetch_sub(1, atomic::Ordering::AcqRel);
    }
}

#[derive(Clone)]
pub struct MetricString {
    value: Arc<Mutex<String>>,
}

impl MetricString {
    pub fn set(&self, value: String) {
        *self.value.lock().unwrap() = value;
    }
}

#[derive(Clone)]
pub struct Metrics {
    registry: Arc<RwLock<HashMap<String, Value>>>,
}

impl Metrics {
    pub fn register_u64(&self, name: String) -> MetricU64 {
        let mut registry = self.registry.write().unwrap();
        let value = registry
            .entry(name)
            .or_insert_with(|| Value::U64(Arc::new(atomic::AtomicU64::new(0))));
        MetricU64 {
            value: match value {
                Value::U64(v) => v.clone(),
                _ => panic!("bad metric type"),
            },
        }
    }

    pub fn register_i64(&self, name: String) -> MetricI64 {
        let mut registry = self.registry.write().unwrap();
        let value = registry
            .entry(name)
            .or_insert_with(|| Value::I64(Arc::new(atomic::AtomicI64::new(0))));
        MetricI64 {
            value: match value {
                Value::I64(v) => v.clone(),
                _ => panic!("bad metric type"),
            },
        }
    }

    pub fn register_latency(&self, name: String) -> MetricLatency {
        let mut registry = self.registry.write().unwrap();
        let value = registry.entry(name).or_insert_with(|| {
            Value::Latency(Arc::new(Mutex::new(Histogram::<u64>::new(3).unwrap())))
        });
        MetricLatency {
            value: match value {
                Value::Latency(v) => v.clone(),
                _ => panic!("bad metric type"),
            },
        }
    }

    pub fn register_string(&self, name: String) -> MetricString {
        let mut registry = self.registry.write().unwrap();
        let value = registry
            .entry(name)
            .or_insert_with(|| Value::String(Arc::new(Mutex::new(String::new()))));
        MetricString {
            value: match value {
                Value::String(v) => v.clone(),
                _ => panic!("bad metric type"),
            },
        }
    }
}

pub fn start() -> Metrics {
    let mut write_interval = mango_v4_client::delay_interval(time::Duration::from_secs(60));

    let registry = Arc::new(RwLock::new(HashMap::<String, Value>::new()));
    let registry_c = Arc::clone(&registry);

    tokio::spawn(async move {
        let mut previous_values = HashMap::<String, PrevValue>::new();
        loop {
            write_interval.tick().await;

            // Nested locking! Safe because the only other user locks registry for writing and doesn't
            // acquire any interior locks.
            let metrics = registry_c.read().unwrap();
            for (name, value) in metrics.iter() {
                let previous_value = previous_values.get_mut(name);
                match value {
                    Value::U64(v) => {
                        let new_value = v.load(atomic::Ordering::Acquire);
                        let previous_value = if let Some(PrevValue::U64(v)) = previous_value {
                            let prev = *v;
                            *v = new_value;
                            prev
                        } else {
                            previous_values.insert(name.clone(), PrevValue::U64(new_value));
                            0
                        };
                        let diff = new_value.wrapping_sub(previous_value) as i64;
                        info!("metric: {}: {} ({:+})", name, new_value, diff);
                    }
                    Value::I64(v) => {
                        let new_value = v.load(atomic::Ordering::Acquire);
                        let previous_value = if let Some(PrevValue::I64(v)) = previous_value {
                            let prev = *v;
                            *v = new_value;
                            prev
                        } else {
                            previous_values.insert(name.clone(), PrevValue::I64(new_value));
                            0
                        };
                        let diff = new_value - previous_value;
                        info!("metric: {}: {} ({:+})", name, new_value, diff);
                    }
                    Value::String(v) => {
                        let new_value = v.lock().unwrap();
                        let previous_value = if let Some(PrevValue::String(v)) = previous_value {
                            let mut prev = new_value.clone();
                            std::mem::swap(&mut prev, v);
                            prev
                        } else {
                            previous_values
                                .insert(name.clone(), PrevValue::String(new_value.clone()));
                            "".into()
                        };
                        if *new_value == previous_value {
                            info!("metric: {}: {} (unchanged)", name, &*new_value);
                        } else {
                            info!(
                                "metric: {}: {} (before: {})",
                                name, &*new_value, previous_value
                            );
                        }
                    }
                    Value::Latency(v) => {
                        let hist = v.lock().unwrap();

                        info!(
                            "metric: {}: 99'th percentile: {:?}, 99,9'th percentile: {:?}",
                            name,
                            Duration::from_nanos(hist.value_at_quantile(0.99)),
                            Duration::from_nanos(hist.value_at_quantile(0.999))
                        );
                    }
                }
            }
        }
        String::from_utf8(buffer.clone()).unwrap()
    });
    println!("Metrics server starting on port 9091");
    warp::serve(metrics_route).run(([0, 0, 0, 0], 9091)).await;
}
