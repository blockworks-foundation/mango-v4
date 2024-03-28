use crate::configuration::Configuration;
use chrono::Utc;
use hdrhistogram::Histogram;
use solana_sdk::blake3::Hash;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use super::data::{AccountUpdateEvent, DataEvent, DataEventSource, SnapshotEvent};

pub struct LoggerProcessor {
    pub job: JoinHandle<()>,
}

impl LoggerProcessor {
    /// TODO FAS
    /// Enlever slot de la key, et comparer en mode "min slot" -> il faut un update avec upd.slot >= existing.slot pour match

    pub async fn init(
        data_sender_1: &tokio::sync::broadcast::Sender<DataEvent>,
        data_sender_2: &tokio::sync::broadcast::Sender<DataEvent>,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<LoggerProcessor> {
        let mut first = true;
        let mut got_1 = false;
        let mut got_2 = false;
        let mut data_1 = data_sender_1.subscribe();
        let mut data_2: tokio::sync::broadcast::Receiver<DataEvent> = data_sender_2.subscribe();

        let job = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(5 * 1000));
            let mut events = HashMap::<Pubkey, AccountUpdateEvent>::new();
            let mut grpc_late = Histogram::<u64>::new(3).unwrap();
            let mut ws_late = Histogram::<u64>::new(3).unwrap();

            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down logger processor...");
                    break;
                }
                tokio::select! {
                    _ = interval.tick() => {
                        if !first {
                            Self::print(&mut events, &mut ws_late, &mut grpc_late);
                            continue;
                        }

                        ws_late.clear();
                        grpc_late.clear();
                        events.clear();
                        first = !got_1 && !got_2;
                    },
                    Ok(msg) = data_1.recv() => { got_1 |= Self::handle(msg, &mut events, &mut ws_late, &mut grpc_late) },
                    Ok(msg) = data_2.recv() => { got_2 |= Self::handle(msg, &mut events, &mut ws_late, &mut grpc_late) },
                }
            }
        });

        let result = LoggerProcessor { job };

        Ok(result)
    }

    fn handle_account(
        upd: AccountUpdateEvent,
        pending_events: &mut HashMap<Pubkey, AccountUpdateEvent>,
        ws_late: &mut Histogram<u64>,
        grpc_late: &mut Histogram<u64>,
        is_snapshot: bool,
    ) {
        let key = upd.account;
        if let Some(existing) = pending_events.get(&key) {
            if existing.slot > upd.slot {
                // still lagging
                return;
            }

            let delay = (upd.received_at - existing.received_at)
                .num_nanoseconds()
                .unwrap();
            match existing.source {
                DataEventSource::Websocket => grpc_late.record(delay as u64).unwrap(),
                DataEventSource::Grpc => ws_late.record(delay as u64).unwrap(),
            }

            if is_snapshot {
                // only match existing,
                // but don't expect matching from the other source as there is probably nothing updated for the account
                pending_events.remove(&key);
                return;
            }

            if upd.slot == existing.slot {
                pending_events.remove(&key);
            } else {
                pending_events.insert(key, upd);
            }
        } else {
            if is_snapshot {
                return; // ignore
            }

            pending_events.insert(key, upd);
        }
    }

    fn print(
        events: &mut HashMap<Pubkey, AccountUpdateEvent>,
        ws_late: &mut Histogram<u64>,
        grpc_late: &mut Histogram<u64>,
    ) {
        let ws_late = format!(
            "{:?}",
            Duration::from_nanos(ws_late.value_at_quantile(0.99))
        );
        let grpc_late = format!(
            "{:?}",
            Duration::from_nanos(grpc_late.value_at_quantile(0.99))
        );
        let pending_ws_events_count = events
            .iter()
            .filter(|f| f.1.source == DataEventSource::Grpc)
            .count();
        let pending_grpc_events_count = events.len() - pending_ws_events_count;

        for x in events {
            tracing::debug!(
                "{} => {} {} (got from {})",
                x.0, x.1.slot, x.1.received_at, x.1.source
            )
        }

        info!(
            ws_lateness = %ws_late,
            grpc_lateness = %grpc_late,
            pending_ws_events_count = %pending_ws_events_count,
            pending_grpc_events_count = %pending_grpc_events_count,
        )
    }

    fn handle(
        msg: DataEvent,
        events: &mut HashMap<Pubkey, AccountUpdateEvent>,
        ws_late: &mut Histogram<u64>,
        grpc_late: &mut Histogram<u64>,
    ) -> bool {
        match msg {
            DataEvent::Other => false,
            DataEvent::Snapshot(upd) => {
                for acc in upd.accounts {
                    Self::handle_account(
                        AccountUpdateEvent {
                            received_at: upd.received_at,
                            account: acc,
                            source: upd.source,
                            slot: upd.slot,
                        },
                        events,
                        ws_late,
                        grpc_late,
                        true,
                    );
                }
                false
            }
            DataEvent::AccountUpdate(upd) => {
                Self::handle_account(upd, events, ws_late, grpc_late, false);
                true
            }
        }
    }
}
