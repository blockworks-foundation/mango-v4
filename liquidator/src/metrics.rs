use {
    log::*,
    std::collections::HashMap,
    std::sync::{atomic, Arc, Mutex, RwLock},
    tokio::time,
};

#[derive(Debug)]
enum Value {
    U64(Arc<atomic::AtomicU64>),
    I64(Arc<atomic::AtomicI64>),
    String(Arc<Mutex<String>>),
}

#[derive(Debug)]
enum PrevValue {
    U64(u64),
    I64(i64),
    String(String),
}

#[derive(Clone)]
pub struct MetricU64 {
    value: Arc<atomic::AtomicU64>,
}
impl MetricU64 {
    pub fn value(&self) -> u64 {
        self.value.load(atomic::Ordering::Acquire)
    }

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
            .or_insert(Value::U64(Arc::new(atomic::AtomicU64::new(0))));
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
            .or_insert(Value::I64(Arc::new(atomic::AtomicI64::new(0))));
        MetricI64 {
            value: match value {
                Value::I64(v) => v.clone(),
                _ => panic!("bad metric type"),
            },
        }
    }

    pub fn register_string(&self, name: String) -> MetricString {
        let mut registry = self.registry.write().unwrap();
        let value = registry
            .entry(name)
            .or_insert(Value::String(Arc::new(Mutex::new(String::new()))));
        MetricString {
            value: match value {
                Value::String(v) => v.clone(),
                _ => panic!("bad metric type"),
            },
        }
    }
}

pub fn start() -> Metrics {
    let mut write_interval = time::interval(time::Duration::from_secs(60));

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
                }
            }
        }
    });

    Metrics { registry }
}
