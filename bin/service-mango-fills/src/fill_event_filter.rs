use log::*;
use mango_feeds_connector::{
    chain_data::{AccountData, ChainData, ChainDataMetrics, SlotData},
    metrics::{MetricType, Metrics},
    AccountWrite, SlotUpdate,
};
use mango_feeds_lib::serum::SerumEventQueueHeader;
use mango_feeds_lib::MarketConfig;
use solana_sdk::{
    account::{ReadableAccount, WritableAccount},
    clock::Epoch,
    pubkey::Pubkey,
};
use std::{
    borrow::BorrowMut,
    cmp::max,
    collections::{HashMap, HashSet},
    iter::FromIterator,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::metrics::MetricU64;
use anchor_lang::AccountDeserialize;
use mango_v4::state::{
    AnyEvent, EventQueue, EventQueueHeader, EventType, FillEvent as PerpFillEvent,
    OutEvent as PerpOutEvent, QueueHeader, MAX_NUM_EVENTS,
};
use service_mango_fills::*;

// couldn't compile the correct struct size / math on m1, fixed sizes resolve this issue
type EventQueueEvents = [AnyEvent; MAX_NUM_EVENTS as usize];

#[allow(clippy::too_many_arguments)]
fn publish_changes_perp(
    slot: u64,
    write_version: u64,
    mkt: &(Pubkey, MarketConfig),
    header: &EventQueueHeader,
    events: &EventQueueEvents,
    prev_seq_num: u64,
    prev_head: usize,
    prev_events: &EventQueueEvents,
    fill_update_sender: &async_channel::Sender<FillEventFilterMessage>,
    metric_events_new: &mut MetricU64,
    metric_events_change: &mut MetricU64,
    metric_events_drop: &mut MetricU64,
    metric_head_update: &mut MetricU64,
) {
    // seq_num = N means that events (N-QUEUE_LEN) until N-1 are available
    let start_seq_num = max(prev_seq_num, header.seq_num).saturating_sub(MAX_NUM_EVENTS as u64);
    let mut checkpoint = Vec::new();
    let mkt_pk_string = mkt.0.to_string();
    let evq_pk_string = mkt.1.event_queue.to_string();
    for seq_num in start_seq_num..header.seq_num {
        let idx = (seq_num % MAX_NUM_EVENTS as u64) as usize;

        // there are three possible cases:
        // 1) the event is past the old seq num, hence guaranteed new event
        // 2) the event is not matching the old event queue
        // 3) all other events are matching the old event queue
        // the order of these checks is important so they are exhaustive
        if seq_num >= prev_seq_num {
            debug!(
                "found new event {} idx {} type {} slot {} write_version {}",
                mkt_pk_string, idx, events[idx].event_type as u32, slot, write_version
            );

            metric_events_new.increment();

            // new fills are published and recorded in checkpoint
            if events[idx].event_type == EventType::Fill as u8 {
                let fill: PerpFillEvent = bytemuck::cast(events[idx]);
                let fill = FillEvent::new_from_perp(fill, &mkt.1);

                fill_update_sender
                    .try_send(FillEventFilterMessage::Update(FillUpdate {
                        slot,
                        write_version,
                        event: fill.clone(),
                        status: FillUpdateStatus::New,
                        market_key: mkt_pk_string.clone(),
                        market_name: mkt.1.name.clone(),
                    }))
                    .unwrap(); // TODO: use anyhow to bubble up error
                checkpoint.push(fill);
            }
        } else if prev_events[idx].event_type != events[idx].event_type
            || prev_events[idx].padding != events[idx].padding
        {
            debug!(
                "found changed event {} idx {} seq_num {} header seq num {} old seq num {}",
                mkt_pk_string, idx, seq_num, header.seq_num, prev_seq_num
            );

            metric_events_change.increment();

            // first revoke old event if a fill
            if prev_events[idx].event_type == EventType::Fill as u8 {
                let fill: PerpFillEvent = bytemuck::cast(prev_events[idx]);
                let fill = FillEvent::new_from_perp(fill, &mkt.1);
                fill_update_sender
                    .try_send(FillEventFilterMessage::Update(FillUpdate {
                        slot,
                        write_version,
                        event: fill,
                        status: FillUpdateStatus::Revoke,
                        market_key: mkt_pk_string.clone(),
                        market_name: mkt.1.name.clone(),
                    }))
                    .unwrap(); // TODO: use anyhow to bubble up error
            }

            // then publish new if its a fill and record in checkpoint
            if events[idx].event_type == EventType::Fill as u8 {
                let fill: PerpFillEvent = bytemuck::cast(events[idx]);
                let fill = FillEvent::new_from_perp(fill, &mkt.1);
                fill_update_sender
                    .try_send(FillEventFilterMessage::Update(FillUpdate {
                        slot,
                        write_version,
                        event: fill.clone(),
                        status: FillUpdateStatus::New,
                        market_key: mkt_pk_string.clone(),
                        market_name: mkt.1.name.clone(),
                    }))
                    .unwrap(); // TODO: use anyhow to bubble up error
                checkpoint.push(fill);
            }
        } else {
            // every already published event is recorded in checkpoint if a fill
            if events[idx].event_type == EventType::Fill as u8 {
                let fill: PerpFillEvent = bytemuck::cast(events[idx]);
                let fill = FillEvent::new_from_perp(fill, &mkt.1);
                checkpoint.push(fill);
            }
        }
    }

    // in case queue size shrunk due to a fork we need revoke all previous fills
    for seq_num in header.seq_num..prev_seq_num {
        let idx = (seq_num % MAX_NUM_EVENTS as u64) as usize;
        debug!(
            "found dropped event {} idx {} seq_num {} header seq num {} old seq num {} slot {} write_version {}",
            mkt_pk_string, idx, seq_num, header.seq_num, prev_seq_num, slot, write_version
        );

        metric_events_drop.increment();

        if prev_events[idx].event_type == EventType::Fill as u8 {
            let fill: PerpFillEvent = bytemuck::cast(prev_events[idx]);
            let fill = FillEvent::new_from_perp(fill, &mkt.1);
            fill_update_sender
                .try_send(FillEventFilterMessage::Update(FillUpdate {
                    slot,
                    event: fill,
                    write_version,
                    status: FillUpdateStatus::Revoke,
                    market_key: mkt_pk_string.clone(),
                    market_name: mkt.1.name.clone(),
                }))
                .unwrap(); // TODO: use anyhow to bubble up error
        }
    }

    let head = header.head();

    let head_seq_num = if events[head - 1].event_type == EventType::Fill as u8 {
        let event: PerpFillEvent = bytemuck::cast(events[head - 1]);
        event.seq_num + 1
    } else if events[head - 1].event_type == EventType::Out as u8 {
        let event: PerpOutEvent = bytemuck::cast(events[head - 1]);
        event.seq_num + 1
    } else {
        0
    };

    let prev_head_seq_num = if prev_events[prev_head - 1].event_type == EventType::Fill as u8 {
        let event: PerpFillEvent = bytemuck::cast(prev_events[prev_head - 1]);
        event.seq_num + 1
    } else if prev_events[prev_head - 1].event_type == EventType::Out as u8 {
        let event: PerpOutEvent = bytemuck::cast(prev_events[prev_head - 1]);
        event.seq_num + 1
    } else {
        0
    };

    // publish a head update event if the head changed (events were consumed)
    if head != prev_head {
        metric_head_update.increment();

        fill_update_sender
            .try_send(FillEventFilterMessage::HeadUpdate(HeadUpdate {
                head,
                prev_head,
                head_seq_num,
                prev_head_seq_num,
                status: FillUpdateStatus::New,
                market_key: mkt_pk_string.clone(),
                market_name: mkt.1.name.clone(),
                slot,
                write_version,
            }))
            .unwrap(); // TODO: use anyhow to bubble up error
    }

    fill_update_sender
        .try_send(FillEventFilterMessage::Checkpoint(FillCheckpoint {
            slot,
            write_version,
            events: checkpoint,
            market: mkt_pk_string,
            queue: evq_pk_string,
        }))
        .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn publish_changes_serum(
    _slot: u64,
    _write_version: u64,
    _mkt: &(Pubkey, MarketConfig),
    _header: &SerumEventQueueHeader,
    _events: &[serum_dex::state::Event],
    _prev_seq_num: u64,
    _prev_events: &[serum_dex::state::Event],
    _fill_update_sender: &async_channel::Sender<FillEventFilterMessage>,
    _metric_events_new: &mut MetricU64,
    _metric_events_change: &mut MetricU64,
    _metric_events_drop: &mut MetricU64,
) {
    // // seq_num = N means that events (N-QUEUE_LEN) until N-1 are available
    // let start_seq_num = max(prev_seq_num, header.seq_num)
    //     .checked_sub(MAX_NUM_EVENTS as u64)
    //     .unwrap_or(0);
    // let mut checkpoint = Vec::new();
    // let mkt_pk_string = mkt.0.to_string();
    // let evq_pk_string = mkt.1.event_queue.to_string();
    // let header_seq_num = header.seq_num;
    // debug!("start seq {} header seq {}", start_seq_num, header_seq_num);

    // // Timestamp for spot events is time scraped
    // let timestamp = SystemTime::now()
    //     .duration_since(SystemTime::UNIX_EPOCH)
    //     .unwrap()
    //     .as_secs();
    // for seq_num in start_seq_num..header_seq_num {
    //     let idx = (seq_num % MAX_NUM_EVENTS as u64) as usize;
    //     let event_view = events[idx].as_view().unwrap();
    //     let old_event_view = prev_events[idx].as_view().unwrap();

    //     match event_view {
    //         SpotEvent::Fill { .. } => {
    //             // there are three possible cases:
    //             // 1) the event is past the old seq num, hence guaranteed new event
    //             // 2) the event is not matching the old event queue
    //             // 3) all other events are matching the old event queue
    //             // the order of these checks is important so they are exhaustive
    //             let fill = FillEvent::new_from_spot(event_view, timestamp, seq_num, &mkt.1);
    //             if seq_num >= prev_seq_num {
    //                 debug!("found new serum fill {} idx {}", mkt_pk_string, idx,);

    //                 metric_events_new.increment();
    //                 fill_update_sender
    //                     .try_send(FillEventFilterMessage::Update(FillUpdate {
    //                         slot,
    //                         write_version,
    //                         event: fill.clone(),
    //                         status: FillUpdateStatus::New,
    //                         market_key: mkt_pk_string.clone(),
    //                         market_name: mkt.1.name.clone(),
    //                     }))
    //                     .unwrap(); // TODO: use anyhow to bubble up error
    //                 checkpoint.push(fill);
    //                 continue;
    //             }

    //             match old_event_view {
    //                 SpotEvent::Fill {
    //                     client_order_id, ..
    //                 } => {
    //                     let client_order_id = match client_order_id {
    //                         Some(id) => id.into(),
    //                         None => 0u64,
    //                     };
    //                     if client_order_id != fill.client_order_id {
    //                         debug!(
    //                             "found changed id event {} idx {} seq_num {} header seq num {} old seq num {}",
    //                             mkt_pk_string, idx, seq_num, header_seq_num, prev_seq_num
    //                         );

    //                         metric_events_change.increment();

    //                         let old_fill = FillEvent::new_from_spot(
    //                             old_event_view,
    //                             timestamp,
    //                             seq_num,
    //                             &mkt.1,
    //                         );
    //                         // first revoke old event
    //                         fill_update_sender
    //                             .try_send(FillEventFilterMessage::Update(FillUpdate {
    //                                 slot,
    //                                 write_version,
    //                                 event: old_fill,
    //                                 status: FillUpdateStatus::Revoke,
    //                                 market_key: mkt_pk_string.clone(),
    //                                 market_name: mkt.1.name.clone(),
    //                             }))
    //                             .unwrap(); // TODO: use anyhow to bubble up error

    //                         // then publish new
    //                         fill_update_sender
    //                             .try_send(FillEventFilterMessage::Update(FillUpdate {
    //                                 slot,
    //                                 write_version,
    //                                 event: fill.clone(),
    //                                 status: FillUpdateStatus::New,
    //                                 market_key: mkt_pk_string.clone(),
    //                                 market_name: mkt.1.name.clone(),
    //                             }))
    //                             .unwrap(); // TODO: use anyhow to bubble up error
    //                     }

    //                     // record new event in checkpoint
    //                     checkpoint.push(fill);
    //                 }
    //                 SpotEvent::Out { .. } => {
    //                     debug!(
    //                         "found changed type event {} idx {} seq_num {} header seq num {} old seq num {}",
    //                         mkt_pk_string, idx, seq_num, header_seq_num, prev_seq_num
    //                     );

    //                     metric_events_change.increment();

    //                     // publish new fill and record in checkpoint
    //                     fill_update_sender
    //                         .try_send(FillEventFilterMessage::Update(FillUpdate {
    //                             slot,
    //                             write_version,
    //                             event: fill.clone(),
    //                             status: FillUpdateStatus::New,
    //                             market_key: mkt_pk_string.clone(),
    //                             market_name: mkt.1.name.clone(),
    //                         }))
    //                         .unwrap(); // TODO: use anyhow to bubble up error
    //                     checkpoint.push(fill);
    //                 }
    //             }
    //         }
    //         _ => continue,
    //     }
    // }

    // // in case queue size shrunk due to a fork we need revoke all previous fills
    // for seq_num in header_seq_num..prev_seq_num {
    //     let idx = (seq_num % MAX_NUM_EVENTS as u64) as usize;
    //     let old_event_view = prev_events[idx].as_view().unwrap();
    //     debug!(
    //         "found dropped event {} idx {} seq_num {} header seq num {} old seq num {}",
    //         mkt_pk_string, idx, seq_num, header_seq_num, prev_seq_num
    //     );

    //     metric_events_drop.increment();

    //     match old_event_view {
    //         SpotEvent::Fill { .. } => {
    //             let old_fill = FillEvent::new_from_spot(old_event_view, timestamp, seq_num, &mkt.1);
    //             fill_update_sender
    //                 .try_send(FillEventFilterMessage::Update(FillUpdate {
    //                     slot,
    //                     event: old_fill,
    //                     write_version,
    //                     status: FillUpdateStatus::Revoke,
    //                     market_key: mkt_pk_string.clone(),
    //                     market_name: mkt.1.name.clone(),
    //                 }))
    //                 .unwrap(); // TODO: use anyhow to bubble up error
    //         }
    //         SpotEvent::Out { .. } => continue,
    //     }
    // }

    // fill_update_sender
    //     .try_send(FillEventFilterMessage::Checkpoint(FillCheckpoint {
    //         slot,
    //         write_version,
    //         events: checkpoint,
    //         market: mkt_pk_string,
    //         queue: evq_pk_string,
    //     }))
    //     .unwrap()
}

pub async fn init(
    perp_market_configs: Vec<(Pubkey, MarketConfig)>,
    spot_market_configs: Vec<(Pubkey, MarketConfig)>,
    metrics_sender: Metrics,
    exit: Arc<AtomicBool>,
) -> anyhow::Result<(
    async_channel::Sender<AccountWrite>,
    async_channel::Sender<SlotUpdate>,
    async_channel::Receiver<FillEventFilterMessage>,
)> {
    let metrics_sender = metrics_sender;

    let mut metric_events_new =
        metrics_sender.register_u64("fills_feed_events_new".into(), MetricType::Counter);
    let mut metric_events_new_serum =
        metrics_sender.register_u64("fills_feed_events_new_serum".into(), MetricType::Counter);
    let mut metric_events_change =
        metrics_sender.register_u64("fills_feed_events_change".into(), MetricType::Counter);
    let mut metric_events_change_serum =
        metrics_sender.register_u64("fills_feed_events_change_serum".into(), MetricType::Counter);
    let mut metrics_events_drop =
        metrics_sender.register_u64("fills_feed_events_drop".into(), MetricType::Counter);
    let mut metrics_events_drop_serum =
        metrics_sender.register_u64("fills_feed_events_drop_serum".into(), MetricType::Counter);
    let mut metrics_head_update =
        metrics_sender.register_u64("fills_feed_head_update".into(), MetricType::Counter);

    // The actual message may want to also contain a retry count, if it self-reinserts on failure?
    let (account_write_queue_sender, account_write_queue_receiver) =
        async_channel::unbounded::<AccountWrite>();

    // Slot updates flowing from the outside into the single processing thread. From
    // there they'll flow into the postgres sending thread.
    let (slot_queue_sender, slot_queue_receiver) = async_channel::unbounded::<SlotUpdate>();

    // Fill updates can be consumed by client connections, they contain all fills for all markets
    let (fill_update_sender, fill_update_receiver) =
        async_channel::unbounded::<FillEventFilterMessage>();

    let account_write_queue_receiver_c = account_write_queue_receiver;

    let mut chain_cache = ChainData::new();
    let mut chain_data_metrics = ChainDataMetrics::new(&metrics_sender);
    let mut perp_events_cache: HashMap<String, EventQueueEvents> = HashMap::new();
    let mut serum_events_cache: HashMap<String, Vec<serum_dex::state::Event>> = HashMap::new();
    let mut seq_num_cache = HashMap::new();
    let mut head_cache = HashMap::new();
    let mut last_evq_versions = HashMap::<String, (u64, u64)>::new();

    let all_market_configs = [perp_market_configs.clone(), spot_market_configs.clone()].concat();
    let perp_queue_pks: Vec<Pubkey> = perp_market_configs
        .iter()
        .map(|x| x.1.event_queue)
        .collect();
    let spot_queue_pks: Vec<Pubkey> = spot_market_configs
        .iter()
        .map(|x| x.1.event_queue)
        .collect();
    let all_queue_pks: HashSet<Pubkey> =
        HashSet::from_iter([perp_queue_pks, spot_queue_pks].concat());

    // update handling thread, reads both sloths and account updates
    tokio::spawn(async move {
        loop {
            if exit.load(Ordering::Relaxed) {
                warn!("shutting down fill_event_filter...");
                break;
            }
            tokio::select! {
                Ok(account_write) = account_write_queue_receiver_c.recv() => {
                    if !all_queue_pks.contains(&account_write.pubkey) {
                        continue;
                    }

                    chain_cache.update_account(
                        account_write.pubkey,
                        AccountData {
                            slot: account_write.slot,
                            write_version: account_write.write_version,
                            account: WritableAccount::create(
                                account_write.lamports,
                                account_write.data.clone(),
                                account_write.owner,
                                account_write.executable,
                                account_write.rent_epoch as Epoch,
                            ),
                        },
                    );
                }
                Ok(slot_update) = slot_queue_receiver.recv() => {
                    chain_cache.update_slot(SlotData {
                        slot: slot_update.slot,
                        parent: slot_update.parent,
                        status: slot_update.status,
                        chain: 0,
                    });

                }
                Err(e) = slot_queue_receiver.recv() => {
                    warn!("slot update channel err {:?}", e);
                }
                Err(e) = account_write_queue_receiver_c.recv() => {
                    warn!("write update channel err {:?}", e);
                }
            }

            chain_data_metrics.report(&chain_cache);

            for mkt in all_market_configs.iter() {
                let evq_pk = mkt.1.event_queue;
                let evq_pk_string = evq_pk.to_string();
                let last_evq_version = last_evq_versions
                    .get(&mkt.1.event_queue.to_string())
                    .unwrap_or(&(0, 0));

                match chain_cache.account(&evq_pk) {
                    Ok(account_info) => {
                        // only process if the account state changed
                        let evq_version = (account_info.slot, account_info.write_version);
                        if evq_version == *last_evq_version {
                            continue;
                        }
                        if evq_version.0 < last_evq_version.0 {
                            debug!("evq version slot was old");
                            continue;
                        }
                        if evq_version.0 == last_evq_version.0 && evq_version.1 < last_evq_version.1
                        {
                            info!("evq version slot was same and write version was old");
                            continue;
                        }
                        last_evq_versions.insert(evq_pk_string.clone(), evq_version);

                        let account = &account_info.account;
                        let is_perp = mango_v4::check_id(account.owner());
                        if is_perp {
                            let event_queue =
                                EventQueue::try_deserialize(account.data().borrow_mut()).unwrap();

                            match (
                                seq_num_cache.get(&evq_pk_string),
                                head_cache.get(&evq_pk_string),
                            ) {
                                (Some(prev_seq_num), Some(prev_head)) => match perp_events_cache
                                    .get(&evq_pk_string)
                                {
                                    Some(prev_events) => publish_changes_perp(
                                        account_info.slot,
                                        account_info.write_version,
                                        mkt,
                                        &event_queue.header,
                                        &event_queue.buf,
                                        *prev_seq_num,
                                        *prev_head,
                                        prev_events,
                                        &fill_update_sender,
                                        &mut metric_events_new,
                                        &mut metric_events_change,
                                        &mut metrics_events_drop,
                                        &mut metrics_head_update,
                                    ),
                                    _ => {
                                        info!("perp_events_cache could not find {}", evq_pk_string)
                                    }
                                },
                                _ => info!("seq_num/head cache could not find {}", evq_pk_string),
                            }

                            seq_num_cache.insert(evq_pk_string.clone(), event_queue.header.seq_num);
                            head_cache.insert(evq_pk_string.clone(), event_queue.header.head());
                            perp_events_cache.insert(evq_pk_string.clone(), event_queue.buf);
                        } else {
                            let inner_data = &account.data()[5..&account.data().len() - 7];
                            let header_span = std::mem::size_of::<SerumEventQueueHeader>();
                            let header: SerumEventQueueHeader =
                                *bytemuck::from_bytes(&inner_data[..header_span]);
                            let seq_num = header.seq_num;
                            let count = header.count;
                            let rest = &inner_data[header_span..];
                            let slop = rest.len() % std::mem::size_of::<serum_dex::state::Event>();
                            let new_len = rest.len() - slop;
                            let events = &rest[..new_len];
                            debug!("evq {} header_span {} header_seq_num {} header_count {} inner_len {} events_len {} sizeof Event {}", evq_pk_string, header_span, seq_num, count, inner_data.len(), events.len(), std::mem::size_of::<serum_dex::state::Event>());
                            let events: &[serum_dex::state::Event] = bytemuck::cast_slice(events);

                            match seq_num_cache.get(&evq_pk_string) {
                                Some(prev_seq_num) => {
                                    match serum_events_cache.get(&evq_pk_string) {
                                        Some(prev_events) => publish_changes_serum(
                                            account_info.slot,
                                            account_info.write_version,
                                            mkt,
                                            &header,
                                            events,
                                            *prev_seq_num,
                                            prev_events,
                                            &fill_update_sender,
                                            &mut metric_events_new_serum,
                                            &mut metric_events_change_serum,
                                            &mut metrics_events_drop_serum,
                                        ),
                                        _ => {
                                            debug!(
                                                "serum_events_cache could not find {}",
                                                evq_pk_string
                                            )
                                        }
                                    }
                                }
                                _ => debug!("seq_num_cache could not find {}", evq_pk_string),
                            }

                            seq_num_cache.insert(evq_pk_string.clone(), seq_num);
                            head_cache.insert(evq_pk_string.clone(), header.head as usize);
                            serum_events_cache.insert(evq_pk_string.clone(), events.to_vec());
                        }
                    }
                    Err(_) => debug!("chain_cache could not find {}", mkt.1.event_queue),
                }
            }
        }
    });

    Ok((
        account_write_queue_sender,
        slot_queue_sender,
        fill_update_receiver,
    ))
}
