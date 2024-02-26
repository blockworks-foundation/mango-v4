use crate::configuration::Configuration;
use crate::fail_or_retry;
use crate::processors::health::{HealthComponent, HealthEvent};
use crate::utils::postgres_connection;
use crate::utils::retry_counter::RetryCounter;
use anchor_lang::prelude::Pubkey;
use chrono::{Duration, Utc};
use fixed::types::I80F48;
use futures_util::pin_mut;
use postgres_types::{ToSql, Type};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::{Client, Transaction};
use tracing::{warn, error};

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
        let postgres_configuration = configuration.postgres.clone().unwrap_or_default();
        let persistence_configuration = configuration.persistence_configuration.clone();

        let job = tokio::spawn(async move {
            if !persistence_configuration.enabled {
                return;
            }

            let mut retry_counter = RetryCounter::new(postgres_configuration.max_retry_count);

            let mut connection = match fail_or_retry!(
                retry_counter,
                postgres_connection::connect(&postgres_configuration).await
            ) {
                Err(e) => {
                    tracing::error!("Failed to connect to postgres sql: {}", e);
                    return;
                }
                Ok(cnt) => cnt,
            };

            let mut previous =
                match fail_or_retry!(retry_counter, Self::load_previous(&connection).await) {
                    Ok(prv) => prv,
                    Err(e) => {
                        error!("loading of previous state failed: {}", e);
                        return;
                    }
                };

            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down persister processor...");
                    break;
                }

                let event = data.recv().await.unwrap();

                if let Err(e) = retry_counter.fail_or_ignore(
                    Self::persist(
                        &mut connection,
                        &mut previous,
                        event,
                        Duration::seconds(persistence_configuration.history_time_to_live_secs),
                    )
                    .await,
                ) {
                    error!("persistence failed: {}", e);
                    break;
                }
            }
        });

        let result = PersisterProcessor { job };

        Ok(result)
    }

    async fn load_previous(client: &Client) -> anyhow::Result<HashMap<Pubkey, PersistedData>> {
        let rows = client
            .query(
                "SELECT Pubkey, Timestamp, MaintenanceRatio, Maintenance, Initial, LiquidationEnd, IsBeingLiquidated FROM mango_monitoring.health_current",
                &[],
            )
            .await?;

        let mut result = HashMap::<Pubkey, PersistedData>::new();
        for row in rows {
            let key = Pubkey::from_str(row.get(0))?;
            let ts: chrono::NaiveDateTime = row.get(1);
            let ts_utc = chrono::DateTime::<Utc>::from_naive_utc_and_offset(ts, Utc);
            let mr: Option<f64> = row.get(2);
            let i: Option<f64> = row.get(3);
            let m: Option<f64> = row.get(4);
            let le: Option<f64> = row.get(5);
            let is_being_liquidated: Option<bool> = row.get(6);
            result.insert(
                key,
                PersistedData {
                    computed_at: ts_utc,
                    maintenance_ratio: mr.map(|x| I80F48::wrapping_from_num(x)), // TODO FAS What conversion should we use there ?
                    initial_health: i.map(|x| I80F48::wrapping_from_num(x)), // TODO FAS What conversion should we use there ?
                    maintenance_health: m.map(|x| I80F48::wrapping_from_num(x)), // TODO FAS What conversion should we use there ?
                    liquidation_end_health: le.map(|x| I80F48::wrapping_from_num(x)), // TODO FAS What conversion should we use there ?
                    is_being_liquidated: is_being_liquidated, // TODO FAS What conversion should we use there ?
                },
            );
        }

        Ok(result)
    }

    async fn persist(
        client: &mut Client,
        previous: &mut HashMap<Pubkey, PersistedData>,
        event: HealthEvent,
        ttl: Duration,
    ) -> anyhow::Result<()> {
        let mut updates = HashMap::new();

        for component in &event.components {
            if !Self::should_insert(&previous, event.computed_at, component.clone()) {
                continue;
            }

            let persisted_data = match &component.value {
                Some(value) => PersistedData {
                    computed_at: event.computed_at,
                    maintenance_ratio: Some(value.maintenance_ratio),
                    initial_health: Some(value.initial_health),
                    maintenance_health: Some(value.maintenance_health),
                    liquidation_end_health: Some(value.liquidation_end_health),
                    is_being_liquidated: Some(value.is_being_liquidated),
                },
                None => PersistedData {
                    computed_at: event.computed_at,
                    maintenance_ratio: None,
                    initial_health: None,
                    maintenance_health: None,
                    liquidation_end_health: None,
                    is_being_liquidated: None,
                },
            };

            updates.insert(component.account, persisted_data);
        }

        let tx = client.transaction().await?;
        Self::insert_history(&tx, &updates).await?;
        Self::delete_old_history(&tx, event.computed_at, ttl).await?;
        Self::update_current(&tx).await?;
        tx.commit().await?;

        for (k, v) in updates {
            previous.insert(k, v);
        }

        Ok(())
    }

    async fn insert_history<'tx>(
        client: &Transaction<'tx>,
        updates: &HashMap<Pubkey, PersistedData>,
    ) -> anyhow::Result<()> {
        let col_types = [
            Type::VARCHAR,
            Type::TIMESTAMP,
            Type::FLOAT8,
            Type::FLOAT8,
            Type::FLOAT8,
            Type::FLOAT8,
            Type::BOOL,
        ];
        let sink = client.copy_in("COPY mango_monitoring.health_history (Pubkey, Timestamp, MaintenanceRatio, Maintenance, Initial, LiquidationEnd, IsBeingLiquidated) FROM STDIN BINARY").await?;
        let writer = BinaryCopyInWriter::new(sink, &col_types);
        pin_mut!(writer);

        for (key, value) in updates {
            let key = key.to_string();
            let ts = value.computed_at.naive_utc();
            let mr = value.maintenance_ratio.map(|x| x.to_num::<f64>());
            let i = value.initial_health.map(|x| x.to_num::<f64>());
            let m = value.maintenance_health.map(|x| x.to_num::<f64>());
            let le = value.liquidation_end_health.map(|x| x.to_num::<f64>());
            let ibl = value.is_being_liquidated;

            let mut row: Vec<&'_ (dyn ToSql + Sync)> = Vec::new();
            row.push(&key);
            row.push(&ts);
            row.push(&mr);
            row.push(&i);
            row.push(&m);
            row.push(&le);
            row.push(&ibl);
            writer.as_mut().write(&row).await?;
        }

        writer.finish().await?;

        Ok(())
    }

    async fn update_current<'tx>(client: &Transaction<'tx>) -> anyhow::Result<()> {
        let query =
            postgres_query::query!("REFRESH MATERIALIZED VIEW mango_monitoring.health_current");
        query.execute(client).await.expect("Update failed");

        Ok(())
    }

    async fn delete_old_history<'tx>(
        client: &Transaction<'tx>,
        now: chrono::DateTime<Utc>,
        ttl: chrono::Duration,
    ) -> anyhow::Result<()> {
        let min_ts = (now - ttl).naive_utc();
        let query = postgres_query::query!(
            "DELETE FROM mango_monitoring.health_history WHERE timestamp < $min_ts",
            min_ts
        );
        if let Err(e) = query.execute(client).await {
            Err(e.into())
        } else {
            Ok(())
        }
    }

    fn should_insert(
        persisted_data: &HashMap<Pubkey, PersistedData>,
        computed_at: chrono::DateTime<Utc>,
        health_component: HealthComponent,
    ) -> bool {
        match persisted_data.get(&health_component.account) {
            None => true,
            Some(previous) => {
                let is_old = computed_at - previous.computed_at >= chrono::Duration::seconds(60);
                let between_none_and_some = previous.is_some() != health_component.value.is_some();

                if is_old || between_none_and_some {
                    true
                } else if previous.is_some() && health_component.value.is_some() {
                    let current_value = health_component.value.unwrap();
                    let changing_flag =
                        current_value.is_being_liquidated != previous.is_being_liquidated.unwrap();

                    let curr = current_value.maintenance_ratio;
                    let prev = previous.maintenance_ratio.unwrap();
                    let changing_side = (prev <= 0 && curr > 0) || (prev > 0 && curr <= 0);
                    let big_move = prev != 0 && (prev - curr).abs() / prev > 0.1;

                    changing_side || changing_flag || big_move
                } else {
                    false
                }
            }
        }
    }
}

struct PersistedData {
    pub computed_at: chrono::DateTime<Utc>,
    pub maintenance_ratio: Option<I80F48>,
    pub initial_health: Option<I80F48>,
    pub maintenance_health: Option<I80F48>,
    pub liquidation_end_health: Option<I80F48>,
    pub is_being_liquidated: Option<bool>,
}

impl PersistedData {
    pub fn is_some(&self) -> bool {
        self.maintenance_ratio.is_some()
            && self.initial_health.is_some()
            && self.maintenance_health.is_some()
            && self.liquidation_end_health.is_some()
            && self.is_being_liquidated.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_value(hr: f64, i: u64, m: u64, le: u64, ibl: bool) -> Option<HealthComponentValue> {
        Some(HealthComponentValue {
            maintenance_ratio: I80F48::wrapping_from_num(hr),
            initial_health: I80F48::from(i),
            maintenance_health: I80F48::from(m),
            liquidation_end_health: I80F48::from(le),
            is_being_liquidated: ibl,
        })
    }

    fn make_persisted(t_secs: i64, mr: f64) -> PersistedData {
        PersistedData {
            computed_at: chrono::Utc::now() - chrono::Duration::seconds(t_secs),
            maintenance_ratio: Some(I80F48::wrapping_from_num(mr)),
            initial_health: Some(I80F48::from(1000)),
            maintenance_health: Some(I80F48::from(1000)),
            liquidation_end_health: Some(I80F48::from(1)),
            is_being_liquidated: Some(false),
        }
    }

    #[test]
    fn should_persist_if_there_is_no_previous_point() {
        let previous = HashMap::new();

        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: Pubkey::new_unique(),
                value: make_value(123f64, 1000, 1000, 1, false)
            }
        ));

        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: Pubkey::new_unique(),
                value: make_value(0f64, 1000, 1000, 1, false)
            }
        ));

        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: Pubkey::new_unique(),
                value: None
            }
        ));
    }

    #[test]
    fn should_persist_if_previous_point_is_old() {
        let mut previous = HashMap::new();
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();
        previous.insert(pk1, make_persisted(120, 123f64));
        previous.insert(pk2, make_persisted(3, 123f64));

        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk1,
                value: make_value(124f64, 1000, 1000, 1, false)
            }
        ));

        assert!(!PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk2,
                value: make_value(124f64, 1000, 1000, 1, false)
            }
        ));
    }

    #[test]
    fn should_persist_when_change_is_interesting() {
        let mut previous = HashMap::new();
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();

        previous.insert(pk1, make_persisted(0, 123f64));

        previous.insert(pk2, make_persisted(0, 1f64 / 100f64));

        // small move, nop
        assert!(!PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk1,
                value: make_value(124f64, 1000, 1000, 1, false)
            }
        ));

        // big move, insert
        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk1,
                value: make_value(100f64, 1000, 1000, 1, false)
            }
        ));

        // small move, but cross 0, insert
        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk2,
                value: make_value(-1f64 / 1000f64, 1000, 1000, 1, false)
            }
        ));

        // small move, does not cross 0, nop
        assert!(!PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk2,
                value: make_value(1f64 / 100f64 + 1f64 / 1000f64, 1000, 1000, 1, false)
            }
        ));

        // no change except flag being liquidated change
        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk2,
                value: make_value(1f64 / 100f64, 1000, 1000, 1, true)
            }
        ));
    }
}
