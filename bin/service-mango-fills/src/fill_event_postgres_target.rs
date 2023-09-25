use crate::postgres_config::PostgresConfig;
use chrono::{TimeZone, Utc};
use log::*;
use mango_feeds_connector::metrics::{MetricType, MetricU64, Metrics};
use native_tls::{Certificate, Identity, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use postgres_query::Caching;
use service_mango_fills::*;
use std::{
    env, fs,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio_postgres::Client;

async fn postgres_connection(
    config: &PostgresConfig,
    metric_retries: MetricU64,
    metric_live: MetricU64,
    exit: Arc<AtomicBool>,
) -> anyhow::Result<async_channel::Receiver<Option<tokio_postgres::Client>>> {
    let (tx, rx) = async_channel::unbounded();

    // openssl pkcs12 -export -in client.cer -inkey client-key.cer -out client.pks
    // base64 -i ca.cer -o ca.cer.b64 && base64 -i client.pks -o client.pks.b64
    // fly secrets set PG_CA_CERT=- < ./ca.cer.b64 -a mango-fills
    // fly secrets set PG_CLIENT_KEY=- < ./client.pks.b64 -a mango-fills
    let tls = match &config.tls {
        Some(tls) => {
            use base64::{engine::general_purpose, Engine as _};
            let ca_cert = match &tls.ca_cert_path.chars().next().unwrap() {
                '$' => general_purpose::STANDARD
                    .decode(
                        env::var(&tls.ca_cert_path[1..])
                            .expect("reading client cert from env")
                            .into_bytes(),
                    )
                    .expect("decoding client cert"),
                _ => fs::read(&tls.ca_cert_path).expect("reading client cert from file"),
            };
            let client_key = match &tls.client_key_path.chars().next().unwrap() {
                '$' => general_purpose::STANDARD
                    .decode(
                        env::var(&tls.client_key_path[1..])
                            .expect("reading client key from env")
                            .into_bytes(),
                    )
                    .expect("decoding client key"),
                _ => fs::read(&tls.client_key_path).expect("reading client key from file"),
            };
            MakeTlsConnector::new(
                TlsConnector::builder()
                    .add_root_certificate(Certificate::from_pem(&ca_cert)?)
                    .identity(Identity::from_pkcs12(&client_key, "pass")?)
                    .danger_accept_invalid_certs(config.allow_invalid_certs)
                    .build()?,
            )
        }
        None => MakeTlsConnector::new(
            TlsConnector::builder()
                .danger_accept_invalid_certs(config.allow_invalid_certs)
                .build()?,
        ),
    };

    let config = config.clone();
    let connection_string = match &config.connection_string.chars().next().unwrap() {
        '$' => {
            env::var(&config.connection_string[1..]).expect("reading connection string from env")
        }
        _ => config.connection_string.clone(),
    };
    let mut initial = Some(tokio_postgres::connect(&connection_string, tls.clone()).await?);
    let mut metric_retries = metric_retries;
    let mut metric_live = metric_live;
    tokio::spawn(async move {
        loop {
            // don't acquire a new connection if we're shutting down
            if exit.load(Ordering::Relaxed) {
                warn!("shutting down fill_event_postgres_target...");
                break;
            }
            let (client, connection) = match initial.take() {
                Some(v) => v,
                None => {
                    let result = tokio_postgres::connect(&connection_string, tls.clone()).await;
                    match result {
                        Ok(v) => v,
                        Err(err) => {
                            warn!("could not connect to postgres: {:?}", err);
                            tokio::time::sleep(Duration::from_secs(
                                config.retry_connection_sleep_secs,
                            ))
                            .await;
                            continue;
                        }
                    }
                }
            };

            tx.send(Some(client)).await.expect("send success");
            metric_live.increment();

            let result = connection.await;

            metric_retries.increment();
            metric_live.decrement();

            tx.send(None).await.expect("send success");
            warn!("postgres connection error: {:?}", result);
            tokio::time::sleep(Duration::from_secs(config.retry_connection_sleep_secs)).await;
        }
    });

    Ok(rx)
}

async fn update_postgres_client<'a>(
    client: &'a mut Option<postgres_query::Caching<tokio_postgres::Client>>,
    rx: &async_channel::Receiver<Option<tokio_postgres::Client>>,
    config: &PostgresConfig,
) -> &'a postgres_query::Caching<tokio_postgres::Client> {
    // get the most recent client, waiting if there's a disconnect
    while !rx.is_empty() || client.is_none() {
        tokio::select! {
            client_raw_opt = rx.recv() => {
                *client = client_raw_opt.expect("not closed").map(postgres_query::Caching::new);
            },
            _ = tokio::time::sleep(Duration::from_secs(config.fatal_connection_timeout_secs)) => {
                error!("waited too long for new postgres client");
                std::process::exit(1);
            },
        }
    }
    client.as_ref().expect("must contain value")
}

async fn process_update(client: &Caching<Client>, update: &FillUpdate) -> anyhow::Result<()> {
    let market = &update.market_key;
    let seq_num = update.event.seq_num as i64;
    let fill_timestamp = Utc.timestamp_opt(update.event.timestamp as i64, 0).unwrap();
    let price = update.event.price;
    let quantity = update.event.quantity;
    let slot = update.slot as i64;
    let write_version = update.write_version as i64;

    if update.status == FillUpdateStatus::New {
        // insert new events
        let query = postgres_query::query!(
            "INSERT INTO transactions_v4.perp_fills_feed_events
            (market, seq_num, fill_timestamp, price,
            quantity, slot, write_version)
            VALUES
            ($market, $seq_num, $fill_timestamp, $price,
            $quantity, $slot, $write_version)
            ON CONFLICT (market, seq_num) DO NOTHING",
            market,
            seq_num,
            fill_timestamp,
            price,
            quantity,
            slot,
            write_version,
        );
        let _ = query.execute(&client).await?;
    } else {
        // delete revoked events
        let query = postgres_query::query!(
            "DELETE FROM transactions_v4.perp_fills_feed_events
            WHERE market=$market
            AND seq_num=$seq_num",
            market,
            seq_num,
        );
        let _ = query.execute(&client).await?;
    }

    Ok(())
}

pub async fn init(
    config: &PostgresConfig,
    metrics_sender: Metrics,
    exit: Arc<AtomicBool>,
) -> anyhow::Result<async_channel::Sender<FillUpdate>> {
    // The actual message may want to also contain a retry count, if it self-reinserts on failure?
    let (fill_update_queue_sender, fill_update_queue_receiver) =
        async_channel::bounded::<FillUpdate>(config.max_queue_size);

    let metric_con_retries = metrics_sender.register_u64(
        "fills_postgres_connection_retries".into(),
        MetricType::Counter,
    );
    let metric_con_live =
        metrics_sender.register_u64("fills_postgres_connections_alive".into(), MetricType::Gauge);

    // postgres fill update sending worker threads
    for _ in 0..config.connection_count {
        let postgres_account_writes = postgres_connection(
            config,
            metric_con_retries.clone(),
            metric_con_live.clone(),
            exit.clone(),
        )
        .await?;
        let fill_update_queue_receiver_c = fill_update_queue_receiver.clone();
        let config = config.clone();
        let mut metric_retries =
            metrics_sender.register_u64("fills_postgres_retries".into(), MetricType::Counter);

        tokio::spawn(async move {
            let mut client_opt = None;
            loop {
                // Retrieve up to batch_size updates
                let mut batch = Vec::new();
                batch.push(
                    fill_update_queue_receiver_c
                        .recv()
                        .await
                        .expect("sender must stay alive"),
                );
                while batch.len() < config.max_batch_size {
                    match fill_update_queue_receiver_c.try_recv() {
                        Ok(update) => batch.push(update),
                        Err(async_channel::TryRecvError::Empty) => break,
                        Err(async_channel::TryRecvError::Closed) => {
                            panic!("sender must stay alive")
                        }
                    };
                }

                info!(
                    "updates, batch {}, channel size {}",
                    batch.len(),
                    fill_update_queue_receiver_c.len(),
                );

                let mut error_count = 0;
                loop {
                    let client =
                        update_postgres_client(&mut client_opt, &postgres_account_writes, &config)
                            .await;
                    let mut results = futures::future::join_all(
                        batch.iter().map(|update| process_update(client, update)),
                    )
                    .await;
                    let mut iter = results.iter();
                    batch.retain(|_| iter.next().unwrap().is_err());
                    if !batch.is_empty() {
                        metric_retries.add(batch.len() as u64);
                        error_count += 1;
                        if error_count - 1 < config.retry_query_max_count {
                            results.retain(|r| r.is_err());
                            warn!("failed to process fill update, retrying: {:?}", results);
                            tokio::time::sleep(Duration::from_secs(config.retry_query_sleep_secs))
                                .await;
                            continue;
                        } else {
                            error!("failed to process account write, exiting");
                            std::process::exit(1);
                        }
                    };
                    break;
                }
            }
        });
    }

    Ok(fill_update_queue_sender)
}
