use jsonrpc_core::futures::StreamExt;
use jsonrpc_core_client::transports::ws;

use mango_feeds_connector::metrics::Metrics;
use solana_sdk::pubkey::Pubkey;

use anyhow::Context;
use async_channel::{RecvError, Sender};
use mango_feeds_connector::{
    EntityFilter, FeedFilterType, FeedWrite, FilterConfig, GrpcSourceConfig, Memcmp,
    SnapshotSourceConfig, SourceConfig,
};
use solana_rpc::rpc_pubsub::RpcSolPubSubClient;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_stream::StreamMap;
use tracing::*;

use crate::account_update_stream::{AccountUpdate, ChainSlotUpdate, Message};
use crate::AnyhowWrap;

pub struct Config {
    pub rpc_ws_url: String,
    pub rpc_http_url: String,
    pub serum_programs: Vec<Pubkey>,
    pub open_orders_authority: Pubkey,
    pub grpc_sources: Vec<GrpcSourceConfig>,
}

async fn feed_data(
    config: &Config,
    mango_oracles: Vec<Pubkey>,
    sender: async_channel::Sender<Message>,
    metrics: &Metrics,
    exit: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let connect = ws::try_connect::<RpcSolPubSubClient>(&config.rpc_ws_url).map_err_anyhow()?;
    let client = connect.await.map_err_anyhow()?;

    let source_config = SourceConfig {
        dedup_queue_size: 5000,
        grpc_sources: config.grpc_sources.clone(),
        snapshot: SnapshotSourceConfig {
            rpc_http_url: config.rpc_http_url.clone(),
        },
        rpc_ws_url: config.rpc_ws_url.clone(),
    };

    let serum3_oo_custom_filters = vec![
        FeedFilterType::DataSize(3228), // open orders size
        // "serum" + u64 that is Initialized (1) + OpenOrders (4)
        FeedFilterType::Memcmp(Memcmp {
            // new_base58_encoded() does not work with old RPC nodes
            offset: 0,
            bytes: [0x73, 0x65, 0x72, 0x75, 0x6d, 5, 0, 0, 0, 0, 0, 0, 0].to_vec(),
        }),
        FeedFilterType::Memcmp(Memcmp {
            offset: 45,
            bytes: config.open_orders_authority.to_bytes().to_vec(),
        }),
    ];

    let mut all_jobs = vec![];

    let mango_filters = FilterConfig {
        entity_filter: EntityFilter::FilterByProgramId(mango_v4::id()),
    };
    let (mango_sub_sender, mango_sub_receiver) = async_channel::unbounded();
    let (mango_sub_slot_sender, mango_sub_slot_receiver) = async_channel::unbounded();
    let mango_sub_job = mango_feeds_connector::grpc_plugin_source::process_events(
        source_config.clone(),
        mango_filters,
        mango_sub_sender,
        mango_sub_slot_sender,
        metrics.clone(),
        exit.clone(),
    );
    all_jobs.push(tokio::spawn(mango_sub_job));

    let mango_oracles_filters = FilterConfig {
        entity_filter: EntityFilter::FilterByAccountIds(mango_oracles),
    };
    let (mango_oracle_sender, mango_oracle_receiver) = async_channel::unbounded();
    let (mango_oracle_slot_sender, mango_oracle_slot_receiver) = async_channel::unbounded();
    let mango_oracle_job = mango_feeds_connector::grpc_plugin_source::process_events(
        source_config.clone(),
        mango_oracles_filters,
        mango_oracle_sender,
        mango_oracle_slot_sender,
        metrics.clone(),
        exit.clone(),
    );
    all_jobs.push(tokio::spawn(mango_oracle_job));

    let mut serum3_oo_sub_map = StreamMap::new();
    let mut serum3_oo_slot_sub_map = StreamMap::new();
    for serum_program in config.serum_programs.iter() {
        let (serum3_oo_sender, serum3_oo_receiver) = async_channel::unbounded();
        let (serum3_oo_slot_sender, serum3_oo_slot_receiver) = async_channel::unbounded();
        let filters = FilterConfig {
            entity_filter: EntityFilter::FilterByProgramIdAndCustomCriteria(
                *serum_program,
                serum3_oo_custom_filters.clone(),
            ),
        };

        let serum3_job = mango_feeds_connector::grpc_plugin_source::process_events(
            source_config.clone(),
            filters,
            serum3_oo_sender,
            serum3_oo_slot_sender,
            metrics.clone(),
            exit.clone(),
        );

        all_jobs.push(tokio::spawn(serum3_job));
        serum3_oo_sub_map.insert(*serum_program, serum3_oo_receiver);
        serum3_oo_slot_sub_map.insert(*serum_program, serum3_oo_slot_receiver);
    }

    // Make sure the serum3_oo_sub_map does not exit when there's no serum_programs
    let _unused_serum_sender;
    if config.serum_programs.is_empty() {
        let (sender, receiver) = async_channel::unbounded::<FeedWrite>();
        _unused_serum_sender = sender;
        serum3_oo_sub_map.insert(Pubkey::default(), receiver);
    }

    let mut slot_sub = client.slots_updates_subscribe().map_err_anyhow()?;

    let mut jobs: futures::stream::FuturesUnordered<_> = all_jobs.into_iter().collect();

    loop {
        tokio::select! {
            _ = jobs.next() => {},
            _ = mango_sub_slot_receiver.recv() => {},
            _ = mango_oracle_slot_receiver.recv() => {},
            _ = serum3_oo_slot_sub_map.next() => {},
            message = mango_sub_receiver.recv() => if !handle_message("mango", message, &sender).await { return Ok(()); },
            message = mango_oracle_receiver.recv() => if !handle_message("oracles", message, &sender).await { return Ok(()); },
            message = serum3_oo_sub_map.next() => {
                if let Some((_, data)) = message {
                    handle_feed_write(&sender, data).await;
                } else {
                    warn!("serum stream closed");
                    return Ok(());
                }
            },
            message = slot_sub.next() => {
                if let Some(data) = message {
                    sender.send(Message::Slot(ChainSlotUpdate{
                        slot_update: data.map_err_anyhow()?,
                        reception_time: Instant::now()
                    })).await.expect("sending must succeed");
                } else {
                    warn!("slot update stream closed");
                    return Ok(());
                }
            },
            _ = tokio::time::sleep(Duration::from_secs(60)) => {
                warn!("grpc timeout");
                return Ok(())
            }
        }
    }
}

async fn handle_message(
    name: &str,
    message: Result<FeedWrite, RecvError>,
    sender: &Sender<Message>,
) -> bool {
    if let Ok(data) = message {
        handle_feed_write(sender, data).await;
        true
    } else {
        warn!("{} stream closed", name);
        false
    }
}

async fn handle_feed_write(sender: &Sender<Message>, data: FeedWrite) {
    match data {
        FeedWrite::Account(account) => sender
            .send(Message::Account(AccountUpdate::from_feed(account)))
            .await
            .expect("sending must succeed"),
        FeedWrite::Snapshot(mut snapshot) => sender
            .send(Message::Snapshot(
                snapshot
                    .accounts
                    .drain(0..)
                    .map(|a| AccountUpdate::from_feed(a))
                    .collect(),
                crate::account_update_stream::SnapshotType::Partial,
            ))
            .await
            .expect("sending must succeed"),
    }
}

pub fn start(
    config: Config,
    mango_oracles: Vec<Pubkey>,
    sender: async_channel::Sender<Message>,
    metrics: Metrics,
    exit: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        // if the grpc disconnects, we get no data in a while etc, reconnect and try again
        loop {
            info!("connecting to solana grpc streams");
            let out = feed_data(
                &config,
                mango_oracles.clone(),
                sender.clone(),
                &metrics,
                exit.clone(),
            );
            let result = out.await;
            if let Err(err) = result {
                warn!("grpc stream error: {err}");
            }
        }
    });
}

pub async fn get_next_create_bank_slot(
    receiver: async_channel::Receiver<Message>,
    timeout: Duration,
) -> anyhow::Result<u64> {
    let start = std::time::Instant::now();
    loop {
        let elapsed = start.elapsed();
        if elapsed > timeout {
            anyhow::bail!(
                "did not receive a slot from the grpc connection in {}s",
                timeout.as_secs()
            );
        }
        let remaining_timeout = timeout - elapsed;

        let msg = match tokio::time::timeout(remaining_timeout, receiver.recv()).await {
            // timeout
            Err(_) => continue,
            // channel close
            Ok(Err(err)) => {
                return Err(err).context("while waiting for first slot from grpc connection");
            }
            // success
            Ok(Ok(msg)) => msg,
        };

        match msg {
            Message::Slot(slot_update) => {
                if let solana_client::rpc_response::SlotUpdate::CreatedBank { slot, .. } =
                    *slot_update.slot_update
                {
                    return Ok(slot);
                }
            }
            _ => {}
        }
    }
}
