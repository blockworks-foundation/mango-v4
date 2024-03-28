use crate::configuration::Configuration;
use crate::processors::data::DataEvent::{AccountUpdate, Other, Snapshot};
use async_channel::Receiver;
use chrono::Utc;
use itertools::Itertools;
use mango_v4_client::account_update_stream::Message;
use mango_v4_client::{account_update_stream, grpc_source, websocket_source, MangoGroupContext};
use services_mango_lib::fail_or_retry;
use services_mango_lib::retry_counter::RetryCounter;
use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::warn;

pub struct DataProcessor {
    pub channel: tokio::sync::broadcast::Sender<DataEvent>,
    pub job: JoinHandle<()>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataEventSource {
    Websocket,
    Grpc,
}

impl Display for DataEventSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug)]
pub enum DataEvent {
    Other,
    Snapshot(SnapshotEvent),
    AccountUpdate(AccountUpdateEvent),
}

#[derive(Clone, Debug)]
pub struct SnapshotEvent {
    pub received_at: chrono::DateTime<Utc>,
    pub accounts: Vec<Pubkey>,
    pub source: DataEventSource,
    pub slot: u64,
}

#[derive(Clone, Debug)]
pub struct AccountUpdateEvent {
    pub received_at: chrono::DateTime<Utc>,
    pub account: Pubkey,
    pub source: DataEventSource,
    pub slot: u64,
}

impl DataProcessor {
    pub async fn init(
        configuration: &Configuration,
        source: DataEventSource,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<DataProcessor> {
        let mut retry_counter = RetryCounter::new(2);
        let mango_stream = fail_or_retry!(
            retry_counter,
            Self::init_mango_source(configuration, source, exit.clone()).await
        )?;
        let (sender, _) = tokio::sync::broadcast::channel(8192);
        let sender_clone = sender.clone();

        let job = tokio::spawn(async move {
            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down data processor...");
                    break;
                }
                tokio::select! {
                    Ok(msg) = mango_stream.recv() => {
                        let received_at = Utc::now();
                        if sender_clone.receiver_count() == 0 {
                            continue;
                        }

                        let event = Self::parse_message(msg, source, received_at);

                        if event.is_none() {
                            continue;
                        }

                        let res = sender_clone.send(event.unwrap());
                        if res.is_err() {
                            break;
                        }
                    },
                    else => {
                        warn!("mango update channel err");
                        break;
                    }
                }
            }
        });

        let result = DataProcessor {
            channel: sender,
            job,
        };

        Ok(result)
    }

    fn new_rpc_async(configuration: &Configuration) -> RpcClientAsync {
        let commitment = CommitmentConfig::processed();
        RpcClientAsync::new_with_timeout_and_commitment(
            configuration.source_configuration.rpc_http_url.clone(),
            Duration::from_secs(60),
            commitment,
        )
    }

    fn parse_message(
        message: Message,
        source: DataEventSource,
        received_at: chrono::DateTime<Utc>,
    ) -> Option<DataEvent> {
        match message {
            Message::Account(account_write) => {
                return Some(AccountUpdate(AccountUpdateEvent {
                    account: account_write.pubkey,
                    received_at,
                    source,
                    slot: account_write.slot,
                }));
            }
            Message::Snapshot(snapshot, _) => {
                let slot = snapshot[0].slot;
                let mut result = Vec::new();
                for update in snapshot.iter() {
                    result.push(update.pubkey);
                    assert!(slot == update.slot);
                }

                return Some(Snapshot(SnapshotEvent {
                    accounts: result,
                    received_at,
                    source: source,
                    slot: slot,
                }));
            }
            _ => {}
        };

        return Some(Other);
    }

    async fn init_mango_source(
        configuration: &Configuration,
        source: DataEventSource,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<Receiver<Message>> {
        //
        // Client setup
        //
        let rpc_async = Self::new_rpc_async(configuration);

        let mango_group = Pubkey::from_str(&configuration.mango_group)?;
        let group_context = MangoGroupContext::new_from_rpc(&rpc_async, mango_group).await?;

        let mango_oracles = group_context
            .tokens
            .values()
            .map(|value| value.oracle)
            .chain(group_context.perp_markets.values().map(|p| p.oracle))
            .unique()
            .collect::<Vec<Pubkey>>();

        let serum_programs = group_context
            .serum3_markets
            .values()
            .map(|s3| s3.serum_program)
            .unique()
            .collect_vec();

        let (account_update_sender, account_update_receiver) =
            async_channel::unbounded::<account_update_stream::Message>();

        if source == DataEventSource::Grpc {
            let metrics_config = mango_feeds_connector::MetricsConfig {
                output_stdout: false,
                output_http: false,
            };
            let metrics = mango_feeds_connector::metrics::start(
                metrics_config,
                "benchmark-data-update".to_string(),
            );
            let sources = configuration.source_configuration.grpc_sources.clone();

            grpc_source::start(
                grpc_source::Config {
                    rpc_http_url: configuration.source_configuration.rpc_http_url.clone(),
                    rpc_ws_url: configuration.source_configuration.rpc_ws_url.clone(),
                    serum_programs,
                    open_orders_authority: mango_group,
                    grpc_sources: sources,
                },
                mango_oracles,
                account_update_sender,
                metrics,
                exit,
            );
        } else {
            websocket_source::start(
                websocket_source::Config {
                    rpc_http_url: configuration.source_configuration.rpc_http_url.clone(),
                    rpc_ws_url: configuration.source_configuration.rpc_ws_url.clone(),
                    serum_programs,
                    open_orders_authority: mango_group,
                },
                mango_oracles,
                account_update_sender,
            );
        }

        Ok(account_update_receiver)
    }
}
