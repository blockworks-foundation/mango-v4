use crate::configuration::Configuration;
use crate::processors::health::{HealthComponent, HealthEvent};
use anchor_lang::prelude::Pubkey;
use chrono::{Duration, DurationRound, Utc};
use futures_util::pin_mut;
use postgres_types::{ToSql, Type};
use services_mango_lib::fail_or_retry;
use services_mango_lib::postgres_configuration::PostgresConfiguration;
use services_mango_lib::postgres_connection;
use services_mango_lib::retry_counter::RetryCounter;
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::{Client, Transaction};
use tracing::{error, warn};

pub struct PersisterProcessor {
    pub job: JoinHandle<()>,
}

impl PersisterProcessor {
    pub async fn init(
        data_sender: &tokio::sync::broadcast::Sender<HealthEvent>,
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<Option<PersisterProcessor>> {
        let postgres_configuration = configuration.postgres.clone().unwrap_or_default();
        let persistence_configuration = configuration.persistence_configuration.clone();
        let time_to_live = Duration::seconds(persistence_configuration.history_time_to_live_secs);
        let periodicity = Duration::seconds(persistence_configuration.persist_max_periodicity_secs);
        let max_snapshot_count = persistence_configuration.snapshot_queue_length;
        let max_failure_duration =
            Duration::seconds(persistence_configuration.max_failure_duration_secs);

        if !persistence_configuration.enabled {
            return Ok(None);
        }

        let mut data = data_sender.subscribe();
        let mut unpersisted_snapshots = VecDeque::new();

        let job = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(1000));
            let mut retry_counter = RetryCounter::new(persistence_configuration.max_retry_count);
            let mut last_successful_persistence = chrono::Utc::now();

            let mut connection = match fail_or_retry!(
                retry_counter,
                postgres_connection::connect(&postgres_configuration).await
            ) {
                Err(e) => {
                    tracing::error!("Failed to connect to postgres sql: {}", e);
                    return;
                }
                Ok(cnt) => cnt.0,
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

                tokio::select! {
                    _ = interval.tick() => {
                    },
                    Ok(event) = data.recv() => {
                        Self::store_snapshot(
                            &previous,
                            &mut unpersisted_snapshots,
                            &event,
                            periodicity,
                            max_snapshot_count,
                        );

                        if let Err(e) = retry_counter.fail_or_ignore(
                            Self::persist_all_snapshots_and_update_state(
                                &mut connection,
                                &mut previous,
                                &mut unpersisted_snapshots,
                                time_to_live,
                            )
                            .await,
                        ) {
                            error!("persistence failed (for {}): {}", chrono::Utc::now() - last_successful_persistence, e);

                            match Self::try_to_reconnect(&postgres_configuration).await {
                                Ok(client) => {
                                    connection = client;
                                }
                                Err(e) => {
                                    if chrono::Utc::now() - last_successful_persistence
                                        > max_failure_duration
                                    {
                                        error!("failed to reconnect (after multiple retries): {}", e);
                                        break; // Shutdown processor
                                    }
                                }
                            };
                        }

                        if unpersisted_snapshots.is_empty() {
                            last_successful_persistence = chrono::Utc::now();
                        }
                    }
                }
            }
        });

        let result = PersisterProcessor { job };

        Ok(Some(result))
    }

    fn build_persisted_data(
        computed_at: chrono::DateTime<Utc>,
        component: &HealthComponent,
    ) -> PersistedData {
        match &component.value {
            Some(value) => PersistedData {
                computed_at: computed_at,
                maintenance_ratio: Some(value.maintenance_ratio),
                initial_health: Some(value.initial_health),
                maintenance_health: Some(value.maintenance_health),
                liquidation_end_health: Some(value.liquidation_end_health),
                is_being_liquidated: Some(value.is_being_liquidated),
            },
            None => PersistedData {
                computed_at: computed_at,
                maintenance_ratio: None,
                initial_health: None,
                maintenance_health: None,
                liquidation_end_health: None,
                is_being_liquidated: None,
            },
        }
    }

    fn store_snapshot(
        previous: &HashMap<Pubkey, PersistedData>,
        snapshots: &mut VecDeque<PersisterSnapshot>,
        event: &HealthEvent,
        periodicity: chrono::Duration,
        max_snapshot_count: usize,
    ) {
        let bucket = event
            .computed_at
            .duration_round(periodicity)
            .unwrap_or(chrono::DateTime::<Utc>::MIN_UTC);
        let mut previous_snapshot = &PersisterSnapshot {
            bucket,
            value: HashMap::new(),
        };
        if !snapshots.is_empty() {
            previous_snapshot = &snapshots[snapshots.len() - 1];
        }

        let updates = event
            .components
            .iter()
            .filter_map(|component| {
                let persisted_data = Self::build_persisted_data(event.computed_at, &component);
                let should_insert_new_point = Self::should_insert(
                    &previous,
                    &component.account,
                    &persisted_data,
                    periodicity,
                );
                let should_update_exising_point =
                    previous_snapshot.value.contains_key(&component.account);

                (should_insert_new_point || should_update_exising_point)
                    .then(|| (component.account, persisted_data))
            })
            .collect();

        if let Some(existing_snapshot_for_bucket) = (*snapshots)
            .iter_mut()
            .find(|s| s.bucket == bucket)
            .as_mut()
        {
            for (k, v) in updates {
                existing_snapshot_for_bucket.value.insert(k, v);
            }
            return;
        }

        if snapshots.len() >= max_snapshot_count {
            snapshots.pop_front();
        }

        let snapshot = PersisterSnapshot {
            bucket,
            value: updates,
        };

        snapshots.push_back(snapshot);
    }

    async fn persist_all_snapshots_and_update_state(
        client: &mut Client,
        previous: &mut HashMap<Pubkey, PersistedData>,
        snapshots: &mut VecDeque<PersisterSnapshot>,
        ttl: Duration,
    ) -> anyhow::Result<()> {
        loop {
            if snapshots.is_empty() {
                break;
            }

            let snapshot = &snapshots[0];

            if snapshot.value.len() == 0 {
                snapshots.pop_front();
                continue;
            }

            Self::persist_snapshot(client, &snapshot.value, ttl).await?;

            let snapshot = snapshots.pop_front().unwrap();
            for (k, v) in snapshot.value {
                previous.insert(k, v);
            }
        }

        Ok(())
    }

    async fn try_to_reconnect(
        postgres_configuration: &PostgresConfiguration,
    ) -> anyhow::Result<Client> {
        let client = postgres_connection::connect(&postgres_configuration)
            .await?
            .0;

        Ok(client)
    }

    async fn load_previous(client: &Client) -> anyhow::Result<HashMap<Pubkey, PersistedData>> {
        let rows = client
            .query(
                "SELECT Pubkey, Timestamp, MaintenanceRatio, Initial, Maintenance, LiquidationEnd, IsBeingLiquidated FROM mango_monitoring.health_current",
                &[],
            )
            .await?;

        let mut result = HashMap::<Pubkey, PersistedData>::new();
        for row in rows {
            let key = Pubkey::from_str(row.get(0))?;
            result.insert(
                key,
                PersistedData {
                    computed_at: row.get(1),
                    maintenance_ratio: row.get(2),
                    initial_health: row.get(3),
                    maintenance_health: row.get(4),
                    liquidation_end_health: row.get(5),
                    is_being_liquidated: row.get(6),
                },
            );
        }

        Ok(result)
    }

    async fn persist_snapshot(
        client: &mut Client,
        updates: &HashMap<Pubkey, PersistedData>,
        ttl: chrono::Duration,
    ) -> anyhow::Result<()> {
        let tx = client.transaction().await?;
        Self::insert_history(&tx, &updates).await?;
        Self::delete_old_history(&tx, chrono::Utc::now(), ttl).await?;
        Self::update_current(&tx).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn insert_history<'tx>(
        client: &Transaction<'tx>,
        updates: &HashMap<Pubkey, PersistedData>,
    ) -> anyhow::Result<()> {
        let col_types = [
            Type::VARCHAR,
            Type::TIMESTAMPTZ,
            Type::FLOAT8,
            Type::FLOAT8,
            Type::FLOAT8,
            Type::FLOAT8,
            Type::BOOL,
        ];
        let sink = client.copy_in("COPY mango_monitoring.health_history (Pubkey, Timestamp, MaintenanceRatio, Initial, Maintenance, LiquidationEnd, IsBeingLiquidated) FROM STDIN BINARY").await?;
        let writer = BinaryCopyInWriter::new(sink, &col_types);
        pin_mut!(writer);

        for (key, value) in updates {
            let key = key.to_string();
            let row: Vec<&'_ (dyn ToSql + Sync)> = vec![
                &key,
                &value.computed_at,
                &value.maintenance_ratio,
                &value.initial_health,
                &value.maintenance_health,
                &value.liquidation_end_health,
                &value.is_being_liquidated,
            ];
            writer.as_mut().write(&row).await?;
        }

        writer.finish().await?;
        Ok(())
    }

    async fn update_current<'tx>(client: &Transaction<'tx>) -> anyhow::Result<()> {
        let query =
            postgres_query::query!("REFRESH MATERIALIZED VIEW mango_monitoring.health_current");
        query.execute(client).await?;
        Ok(())
    }

    async fn delete_old_history<'tx>(
        client: &Transaction<'tx>,
        now: chrono::DateTime<Utc>,
        ttl: chrono::Duration,
    ) -> anyhow::Result<()> {
        let min_ts = now - ttl;
        let query = postgres_query::query!(
            "DELETE FROM mango_monitoring.health_history WHERE timestamp < $min_ts",
            min_ts
        );
        query.execute(client).await?;
        Ok(())
    }

    fn should_insert(
        persisted_data: &HashMap<Pubkey, PersistedData>,
        health_component_key: &Pubkey,
        health_component: &PersistedData,
        periodicity: Duration,
    ) -> bool {
        match persisted_data.get(health_component_key) {
            None => true,
            Some(previous) => {
                let is_old = health_component.computed_at - previous.computed_at >= periodicity;
                let between_none_and_some = previous.is_some() != health_component.is_some();

                if is_old || between_none_and_some {
                    true
                } else if previous.is_some() && health_component.is_some() {
                    let changing_flag = health_component.is_being_liquidated.unwrap()
                        != previous.is_being_liquidated.unwrap();

                    let curr = health_component.maintenance_ratio.unwrap();
                    let prev = previous.maintenance_ratio.unwrap();
                    let changing_side = (prev <= 0.0 && curr > 0.0) || (prev > 0.0 && curr <= 0.0);
                    let big_move = prev != 0.0 && (prev - curr).abs() / prev > 0.1;

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
    pub maintenance_ratio: Option<f64>,
    pub initial_health: Option<f64>,
    pub maintenance_health: Option<f64>,
    pub liquidation_end_health: Option<f64>,
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

struct PersisterSnapshot {
    pub value: HashMap<Pubkey, PersistedData>,
    pub bucket: chrono::DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processors::health::HealthComponentValue;
    use chrono::SubsecRound;

    fn make_value(hr: f64, i: u64, m: u64, le: u64, ibl: bool) -> Option<HealthComponentValue> {
        Some(HealthComponentValue {
            maintenance_ratio: hr,
            initial_health: i as f64,
            maintenance_health: m as f64,
            liquidation_end_health: le as f64,
            is_being_liquidated: ibl,
        })
    }

    fn make_persisted_empty(t_secs: i64) -> PersistedData {
        PersistedData {
            computed_at: chrono::Utc::now() - chrono::Duration::seconds(t_secs),
            maintenance_ratio: None,
            initial_health: None,
            maintenance_health: None,
            liquidation_end_health: None,
            is_being_liquidated: None,
        }
    }

    fn make_persisted(t_secs: i64, mr: f64) -> PersistedData {
        PersistedData {
            computed_at: chrono::Utc::now() - chrono::Duration::seconds(t_secs),
            maintenance_ratio: Some(mr),
            initial_health: Some(1000f64),
            maintenance_health: Some(1000f64),
            liquidation_end_health: Some(1f64),
            is_being_liquidated: Some(false),
        }
    }

    fn make_persisted_with_liquidated_flag(t_secs: i64, mr: f64) -> PersistedData {
        PersistedData {
            computed_at: chrono::Utc::now() - chrono::Duration::seconds(t_secs),
            maintenance_ratio: Some(mr),
            initial_health: Some(1000f64),
            maintenance_health: Some(1000f64),
            liquidation_end_health: Some(1f64),
            is_being_liquidated: Some(true),
        }
    }

    #[test]
    fn should_persist_if_there_is_no_previous_point() {
        let previous = HashMap::new();

        assert!(PersisterProcessor::should_insert(
            &previous,
            &Pubkey::new_unique(),
            &make_persisted(0, 123f64),
            chrono::Duration::seconds(60)
        ));

        assert!(PersisterProcessor::should_insert(
            &previous,
            &Pubkey::new_unique(),
            &make_persisted(0, 0f64),
            chrono::Duration::seconds(60)
        ));

        assert!(PersisterProcessor::should_insert(
            &previous,
            &Pubkey::new_unique(),
            &make_persisted_empty(0),
            chrono::Duration::seconds(60)
        ));
    }

    #[test]
    fn should_persist_if_previous_point_is_old() {
        let mut previous = HashMap::new();
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();
        previous.insert(pk1, make_persisted(120, 123.0));
        previous.insert(pk2, make_persisted(3, 123.0));

        assert!(PersisterProcessor::should_insert(
            &previous,
            &pk1,
            &make_persisted(0, 124.0),
            chrono::Duration::seconds(60)
        ));

        assert!(!PersisterProcessor::should_insert(
            &previous,
            &pk2,
            &make_persisted(0, 124.0),
            chrono::Duration::seconds(60)
        ));
    }

    #[test]
    fn should_persist_when_change_is_interesting() {
        let mut previous = HashMap::new();
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();

        previous.insert(pk1, make_persisted(0, 123f64));

        previous.insert(pk2, make_persisted(0, 0.01));

        // small move, nop
        assert!(!PersisterProcessor::should_insert(
            &previous,
            &pk1,
            &make_persisted(0, 124.0),
            chrono::Duration::seconds(60)
        ));

        // big move, insert
        assert!(PersisterProcessor::should_insert(
            &previous,
            &pk1,
            &make_persisted(0, 100.0),
            chrono::Duration::seconds(60)
        ));

        // small move, but cross 0, insert
        assert!(PersisterProcessor::should_insert(
            &previous,
            &pk2,
            &make_persisted(0, -0.001),
            chrono::Duration::seconds(60)
        ));

        // small move, does not cross 0, nop
        assert!(!PersisterProcessor::should_insert(
            &previous,
            &pk2,
            &make_persisted(0, 0.0099),
            chrono::Duration::seconds(60)
        ));

        // no change except flag being liquidated change
        assert!(PersisterProcessor::should_insert(
            &previous,
            &pk2,
            &make_persisted_with_liquidated_flag(0, 0.01),
            chrono::Duration::seconds(60)
        ));
    }

    #[test]
    fn should_correctly_convert_event_into_data() {
        let computed_at = chrono::Utc::now();
        let component = HealthComponent {
            account: Pubkey::new_unique(),
            value: Some(HealthComponentValue {
                maintenance_ratio: 123.0,
                initial_health: 1000.0,
                maintenance_health: 2000.0,
                liquidation_end_health: 3000.0,
                is_being_liquidated: false,
            }),
        };

        let converted = PersisterProcessor::build_persisted_data(computed_at, &component);

        assert_eq!(converted.computed_at, computed_at);
        assert_eq!(converted.maintenance_ratio.unwrap(), 123.0);
        assert_eq!(converted.initial_health.unwrap(), 1000.0);
        assert_eq!(converted.maintenance_health.unwrap(), 2000.0);
        assert_eq!(converted.liquidation_end_health.unwrap(), 3000.0);
        assert_eq!(converted.is_being_liquidated.unwrap(), false);
    }

    #[test]
    fn should_store_or_replace_snapshot() {
        let pk = Pubkey::new_unique();
        let previous = HashMap::new();
        let mut snapshots = VecDeque::new();
        let event1 = HealthEvent {
            computed_at: chrono::Utc::now().trunc_subsecs(0) - chrono::Duration::seconds(300),
            components: vec![HealthComponent {
                account: pk,
                value: make_value(50.25f64, 2, 3, 4, false),
            }],
        };
        let event2 = HealthEvent {
            computed_at: chrono::Utc::now().trunc_subsecs(0) - chrono::Duration::seconds(290),
            components: vec![HealthComponent {
                account: pk,
                value: make_value(502.5f64, 20, 30, 40, false),
            }],
        };
        let event3 = HealthEvent {
            computed_at: chrono::Utc::now().trunc_subsecs(0) - chrono::Duration::seconds(200),
            components: vec![HealthComponent {
                account: pk,
                value: make_value(5025.0f64, 200, 300, 400, false),
            }],
        };
        let event4 = HealthEvent {
            computed_at: chrono::Utc::now().trunc_subsecs(0) - chrono::Duration::seconds(100),
            components: vec![HealthComponent {
                account: pk,
                value: make_value(50250.0f64, 2000, 3000, 4000, false),
            }],
        };

        PersisterProcessor::store_snapshot(
            &previous,
            &mut snapshots,
            &event1,
            Duration::seconds(60),
            2,
        );
        assert_eq!(snapshots.len(), 1);
        assert_eq!(
            snapshots[0]
                .value
                .iter()
                .next()
                .unwrap()
                .1
                .maintenance_health
                .unwrap(),
            3.0
        );

        PersisterProcessor::store_snapshot(
            &previous,
            &mut snapshots,
            &event2,
            Duration::seconds(60),
            2,
        );
        assert_eq!(snapshots.len(), 1);
        assert_eq!(
            snapshots[0]
                .value
                .iter()
                .next()
                .unwrap()
                .1
                .maintenance_health
                .unwrap(),
            30.0
        );

        PersisterProcessor::store_snapshot(
            &previous,
            &mut snapshots,
            &event3,
            Duration::seconds(60),
            2,
        );
        assert_eq!(snapshots.len(), 2);
        assert_eq!(
            snapshots[0]
                .value
                .iter()
                .next()
                .unwrap()
                .1
                .maintenance_health
                .unwrap(),
            30.0
        );
        assert_eq!(
            snapshots[1]
                .value
                .iter()
                .next()
                .unwrap()
                .1
                .maintenance_health
                .unwrap(),
            300.0
        );

        PersisterProcessor::store_snapshot(
            &previous,
            &mut snapshots,
            &event4,
            Duration::seconds(60),
            2,
        );
        assert_eq!(snapshots.len(), 2);
        assert_eq!(
            snapshots[0]
                .value
                .iter()
                .next()
                .unwrap()
                .1
                .maintenance_health
                .unwrap(),
            300.0
        );
        assert_eq!(
            snapshots[1]
                .value
                .iter()
                .next()
                .unwrap()
                .1
                .maintenance_health
                .unwrap(),
            3000.0
        );
    }
}
