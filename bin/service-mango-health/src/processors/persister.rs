use crate::configuration::Configuration;
use crate::fail_or_retry;
use crate::processors::health::{HealthComponent, HealthEvent};
use crate::utils::postgres_connection;
use crate::utils::retry_counter::RetryCounter;
use anchor_lang::prelude::Pubkey;
use chrono::Utc;
use fixed::types::I80F48;
use log::warn;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_postgres::{Client, Transaction};

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
        let mut previous = HashMap::<Pubkey, PersistedData>::new();
        let is_enabled = configuration.postgres.is_some();
        let postgres_configuration = configuration.postgres.clone().unwrap_or_default();

        let job = tokio::spawn(async move {
            if !is_enabled {
                return;
            }

            let mut retry_counter = RetryCounter::new(postgres_configuration.max_retry_count);

            let mut connection = match fail_or_retry!(
                retry_counter,
                postgres_connection::connect(&postgres_configuration).await
            ) {
                Err(e) => {
                    log::error!("Failed to connect to postgres sql: {}", e);
                    return;
                }
                Ok(cnt) => cnt,
            };

            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down persister processor...");
                    break;
                }

                if previous.is_empty() {
                    let previous_res =
                        fail_or_retry!(retry_counter, Self::load_previous(&connection).await);
                    match previous_res {
                        Ok(prv) => {
                            previous = prv;
                        }
                        Err(e) => {
                            log::error!("loading of previous state failed: {}", e);
                            break;
                        }
                    }
                }

                let event = data.recv().await.unwrap();

                if let Err(e) = retry_counter
                    .fail_or_ignore(Self::persist(&mut connection, &mut previous, event).await)
                {
                    log::error!("persistence failed: {}", e);
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
                "SELECT Pubkey, Timestamp, healthRatio FROM mango_monitoring.health_current",
                &[],
            )
            .await?;

        let mut result = HashMap::<Pubkey, PersistedData>::new();
        for row in rows {
            let key = Pubkey::from_str(row.get(0))?;
            let ts: chrono::NaiveDateTime = row.get(1);
            let ts_utc = chrono::DateTime::<Utc>::from_naive_utc_and_offset(ts, Utc);
            let hr: Option<f64> = row.get(2);
            result.insert(
                key,
                PersistedData {
                    computed_at: ts_utc,
                    health_ratio: hr.map(|x| I80F48::wrapping_from_num(x)), // TODO FAS What conversion should we use there ?
                },
            );
        }

        Ok(result)
    }

    async fn persist(
        client: &mut Client,
        previous: &mut HashMap<Pubkey, PersistedData>,
        event: HealthEvent,
    ) -> anyhow::Result<()> {
        let mut updates = HashMap::new();

        for component in &event.components {
            if !Self::should_insert(&previous, event.computed_at, component.clone()) {
                continue;
            }

            updates.insert(
                component.account,
                PersistedData {
                    computed_at: event.computed_at,
                    health_ratio: component.health_ratio,
                },
            );
        }

        let tx = client.transaction().await?;
        Self::insert_history(&tx, &updates).await?;
        Self::update_current(&tx, &updates, &previous).await?; // <- updated & present in previous
        Self::insert_current(&tx, &updates, &previous).await?; // <- updated & not present in previous
        Self::delete_old_history(&tx, event.computed_at).await?;
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
        for (key, value) in updates {
            let key = key.to_string();
            let ts = value.computed_at.naive_utc();
            let hr = value.health_ratio.map(|x| x.to_num::<f64>());
            let query = postgres_query::query!("INSERT INTO mango_monitoring.health_history (Pubkey, Timestamp, HealthRatio) VALUES ($key, $ts, $hr)", key, ts, hr);
            query.execute(client).await.expect("Insertion failed");
        }

        Ok(())
    }

    async fn update_current<'tx>(
        client: &Transaction<'tx>,
        updates: &HashMap<Pubkey, PersistedData>,
        previous: &HashMap<Pubkey, PersistedData>,
    ) -> anyhow::Result<()> {
        let to_update = updates
            .into_iter()
            .filter(|(key, _)| previous.contains_key(&key));

        for (key, value) in to_update {
            let key = key.to_string();
            let ts = value.computed_at.naive_utc();
            let hr = value.health_ratio.map(|x| x.to_num::<f64>());
            let query = postgres_query::query!("UPDATE mango_monitoring.health_current SET Timestamp=$ts, HealthRatio=$hr WHERE Pubkey = $key", key, ts, hr);
            query.execute(client).await.expect("Update failed");
        }

        Ok(())
    }

    async fn insert_current<'tx>(
        client: &Transaction<'tx>,
        updates: &HashMap<Pubkey, PersistedData>,
        previous: &HashMap<Pubkey, PersistedData>,
    ) -> anyhow::Result<()> {
        let to_insert = updates
            .iter()
            .filter(|(key, _)| !previous.contains_key(key));

        for (key, value) in to_insert {
            let key = key.to_string();
            let ts = value.computed_at.naive_utc();
            let hr = value.health_ratio.map(|x| x.to_num::<f64>());
            let query = postgres_query::query!("INSERT INTO mango_monitoring.health_current (Pubkey, Timestamp, HealthRatio) VALUES ($key, $ts, $hr)", key, ts, hr);
            query.execute(client).await.expect("Insertion failed");
        }

        Ok(())
    }

    async fn delete_old_history<'tx>(
        client: &Transaction<'tx>,
        now: chrono::DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let min_ts = (now - chrono::Duration::days(31)).naive_utc();
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
                let between_none_and_some =
                    previous.health_ratio.is_some() != health_component.health_ratio.is_some();

                if is_old || between_none_and_some {
                    true
                } else if previous.health_ratio.is_some() && health_component.health_ratio.is_some()
                {
                    let prev = previous.health_ratio.unwrap();
                    let curr = health_component.health_ratio.unwrap();
                    let changing_side = (prev <= 0 && curr > 0) || (prev > 0 && curr <= 0);
                    let big_move = prev != 0 && (prev - curr).abs() / prev > 0.1;

                    changing_side || big_move
                } else {
                    false
                }
            }
        }
    }
}

struct PersistedData {
    pub computed_at: chrono::DateTime<Utc>,
    pub health_ratio: Option<I80F48>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_persist_if_there_is_no_previous_point() {
        let previous = HashMap::new();

        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: Pubkey::new_unique(),
                health_ratio: Some(I80F48::from(123))
            }
        ));

        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: Pubkey::new_unique(),
                health_ratio: Some(I80F48::ZERO)
            }
        ));

        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: Pubkey::new_unique(),
                health_ratio: None
            }
        ));
    }

    #[test]
    fn should_persist_if_previous_point_is_old() {
        let mut previous = HashMap::new();
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();
        previous.insert(
            pk1,
            PersistedData {
                computed_at: chrono::Utc::now() - chrono::Duration::seconds(120),
                health_ratio: Some(I80F48::from(123)),
            },
        );
        previous.insert(
            pk2,
            PersistedData {
                computed_at: chrono::Utc::now() - chrono::Duration::seconds(3),
                health_ratio: Some(I80F48::from(123)),
            },
        );

        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk1,
                health_ratio: Some(I80F48::from(124))
            }
        ));

        assert!(!PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk2,
                health_ratio: Some(I80F48::from(124))
            }
        ));
    }

    #[test]
    fn should_persist_when_change_is_interesting() {
        let mut previous = HashMap::new();
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();

        previous.insert(
            pk1,
            PersistedData {
                computed_at: chrono::Utc::now() - chrono::Duration::seconds(0),
                health_ratio: Some(I80F48::from(123)),
            },
        );

        previous.insert(
            pk2,
            PersistedData {
                computed_at: chrono::Utc::now() - chrono::Duration::seconds(0),
                health_ratio: Some(I80F48::from(1) / I80F48::from(100)),
            },
        );

        // small move, nop
        assert!(!PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk1,
                health_ratio: Some(I80F48::from(124))
            }
        ));

        // big move, insert
        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk1,
                health_ratio: Some(I80F48::from(100))
            }
        ));

        // small move, but cross 0, insert
        assert!(PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk2,
                health_ratio: Some(I80F48::from(-1) / I80F48::from(1000))
            }
        ));

        // small move, does not cross 0, nop
        assert!(!PersisterProcessor::should_insert(
            &previous,
            chrono::Utc::now(),
            HealthComponent {
                account: pk2,
                health_ratio: Some(
                    I80F48::from(1) / I80F48::from(100) + I80F48::from(1) / I80F48::from(1000)
                )
            }
        ));
    }
}
