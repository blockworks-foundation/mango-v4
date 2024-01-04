use anchor_lang::AccountDeserialize;
use fixed::types::I80F48;
use itertools::Itertools;
use log::*;
use mango_feeds_connector::metrics::MetricU64;
use mango_feeds_connector::{
    chain_data::{AccountData, ChainData, ChainDataMetrics, SlotData},
    metrics::{MetricType, Metrics},
    AccountWrite, SlotUpdate,
};
use mango_feeds_lib::{
    base_lots_to_ui, base_lots_to_ui_perp, price_lots_to_ui, price_lots_to_ui_perp, MarketConfig,
    OrderbookSide,
};
use mango_v4::accounts_zerocopy::{AccountReader, KeyedAccountReader};
use mango_v4::state::OracleConfigParams;
use mango_v4::state::{oracle_state_unchecked, OracleAccountInfos};
use mango_v4::{
    serum3_cpi::OrderBookStateHeader,
    state::{BookSide, OrderTreeType},
};
use serum_dex::critbit::Slab;
use service_mango_orderbook::{
    BookCheckpoint, BookUpdate, LevelCheckpoint, LevelUpdate, Order, OrderbookFilterMessage,
    OrderbookLevel,
};
use solana_sdk::account::AccountSharedData;
use solana_sdk::{
    account::{ReadableAccount, WritableAccount},
    clock::Epoch,
    pubkey::Pubkey,
};
use std::borrow::BorrowMut;
use std::{
    collections::{HashMap, HashSet},
    mem::size_of,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

struct KeyedSharedDataAccountReader {
    pub key: Pubkey,
    pub shared: AccountSharedData,
}

impl AccountReader for KeyedSharedDataAccountReader {
    fn owner(&self) -> &Pubkey {
        ReadableAccount::owner(&self.shared)
    }

    fn data(&self) -> &[u8] {
        ReadableAccount::data(&self.shared)
    }
}

impl KeyedAccountReader for KeyedSharedDataAccountReader {
    fn key(&self) -> &Pubkey {
        &self.key
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::ptr_arg)]
fn publish_changes(
    slot: u64,
    write_version: u64,
    mkt: &(Pubkey, MarketConfig),
    side: OrderbookSide,
    current_orders: &Vec<Order>,
    previous_orders: &Vec<Order>,
    maybe_other_orders: Option<&Vec<Order>>,
    orderbook_update_sender: &async_channel::Sender<OrderbookFilterMessage>,
    metric_book_updates: &mut MetricU64,
    metric_level_updates: &mut MetricU64,
) {
    let mut level_update: Vec<OrderbookLevel> = vec![];
    let mut book_additions: Vec<Order> = vec![];
    let mut book_removals: Vec<Order> = vec![];

    let current_bookside: Vec<OrderbookLevel> = current_orders
        .iter()
        .group_by(|order| order.price)
        .into_iter()
        .map(|(price, group)| [price, group.map(|o| o.quantity).sum()])
        .collect();

    let previous_bookside: Vec<OrderbookLevel> = previous_orders
        .iter()
        .group_by(|order| order.price)
        .into_iter()
        .map(|(price, group)| [price, group.map(|o| o.quantity).sum()])
        .collect();

    // push diff for levels that are no longer present
    if current_bookside.len() != previous_bookside.len() {
        debug!(
            "L {}",
            current_bookside.len() as i64 - previous_bookside.len() as i64
        )
    }

    for prev_order in previous_orders.iter() {
        let peer = current_orders.iter().find(|order| prev_order == *order);

        match peer {
            None => {
                debug!("R {:?}", prev_order);
                book_removals.push(prev_order.clone());
            }
            _ => continue,
        }
    }

    for previous_level in previous_bookside.iter() {
        let peer = current_bookside
            .iter()
            .find(|level| previous_level[0] == level[0]);

        match peer {
            None => {
                debug!("R {} {}", previous_level[0], previous_level[1]);
                level_update.push([previous_level[0], 0f64]);
            }
            _ => continue,
        }
    }

    // push diff where there's a new level or size has changed
    for current_level in &current_bookside {
        let peer = previous_bookside
            .iter()
            .find(|item| item[0] == current_level[0]);

        match peer {
            Some(previous_level) => {
                if previous_level[1] == current_level[1] {
                    continue;
                }
                debug!(
                    "C {} {} -> {}",
                    current_level[0], previous_level[1], current_level[1]
                );
                level_update.push(*current_level);
            }
            None => {
                debug!("A {} {}", current_level[0], current_level[1]);
                level_update.push(*current_level)
            }
        }
    }

    for current_order in current_orders {
        let peer = previous_orders.iter().find(|order| current_order == *order);

        match peer {
            Some(_) => {
                continue;
            }
            None => {
                debug!("A {:?}", current_order);
                book_additions.push(current_order.clone())
            }
        }
    }

    match maybe_other_orders {
        Some(other_orders) => {
            let (bids, asks) = match side {
                OrderbookSide::Bid => (current_orders, other_orders),
                OrderbookSide::Ask => (other_orders, current_orders),
            };
            orderbook_update_sender
                .try_send(OrderbookFilterMessage::BookCheckpoint(BookCheckpoint {
                    slot,
                    write_version,
                    bids: bids.clone(),
                    asks: asks.clone(),
                    market: mkt.0.to_string(),
                }))
                .unwrap();

            let bid_levels = bids
                .iter()
                .group_by(|order| order.price)
                .into_iter()
                .map(|(price, group)| [price, group.map(|o| o.quantity).sum()])
                .collect();

            let ask_levels = asks
                .iter()
                .group_by(|order| order.price)
                .into_iter()
                .map(|(price, group)| [price, group.map(|o| o.quantity).sum()])
                .collect();

            orderbook_update_sender
                .try_send(OrderbookFilterMessage::LevelCheckpoint(LevelCheckpoint {
                    slot,
                    write_version,
                    bids: bid_levels,
                    asks: ask_levels,
                    market: mkt.0.to_string(),
                }))
                .unwrap()
        }
        None => info!("other bookside not in cache"),
    }

    if !level_update.is_empty() {
        orderbook_update_sender
            .try_send(OrderbookFilterMessage::LevelUpdate(LevelUpdate {
                market: mkt.0.to_string(),
                side: side.clone(),
                update: level_update,
                slot,
                write_version,
            }))
            .unwrap(); // TODO: use anyhow to bubble up error
        metric_level_updates.increment();
    }

    if !book_additions.is_empty() && !book_removals.is_empty() {
        orderbook_update_sender
            .try_send(OrderbookFilterMessage::BookUpdate(BookUpdate {
                market: mkt.0.to_string(),
                side,
                additions: book_additions,
                removals: book_removals,
                slot,
                write_version,
            }))
            .unwrap();
        metric_book_updates.increment();
    }
}

pub async fn init(
    market_configs: Vec<(Pubkey, MarketConfig)>,
    serum_market_configs: Vec<(Pubkey, MarketConfig)>,
    metrics_sender: Metrics,
    exit: Arc<AtomicBool>,
) -> anyhow::Result<(
    async_channel::Sender<AccountWrite>,
    async_channel::Sender<SlotUpdate>,
    async_channel::Receiver<OrderbookFilterMessage>,
)> {
    let mut metric_book_events_new =
        metrics_sender.register_u64("orderbook_book_updates".into(), MetricType::Counter);
    let mut metric_level_events_new =
        metrics_sender.register_u64("orderbook_level_updates".into(), MetricType::Counter);

    // The actual message may want to also contain a retry count, if it self-reinserts on failure?
    let (account_write_queue_sender, account_write_queue_receiver) =
        async_channel::unbounded::<AccountWrite>();

    // Slot updates flowing from the outside into the single processing thread. From
    // there they'll flow into the postgres sending thread.
    let (slot_queue_sender, slot_queue_receiver) = async_channel::unbounded::<SlotUpdate>();

    // Book updates can be consumed by client connections, they contain L2 and L3 updates for all markets
    let (book_update_sender, book_update_receiver) =
        async_channel::unbounded::<OrderbookFilterMessage>();

    let mut chain_cache = ChainData::new();
    let mut chain_data_metrics = ChainDataMetrics::new(&metrics_sender);
    let mut bookside_cache: HashMap<String, Vec<Order>> = HashMap::new();
    let mut serum_bookside_cache: HashMap<String, Vec<Order>> = HashMap::new();
    let mut last_write_versions = HashMap::<String, (u64, u64)>::new();

    let mut relevant_pubkeys = [market_configs.clone(), serum_market_configs.clone()]
        .concat()
        .iter()
        .flat_map(|m| [m.1.bids, m.1.asks])
        .collect::<HashSet<Pubkey>>();

    relevant_pubkeys.extend(market_configs.iter().map(|(_, cfg)| cfg.oracle));

    info!("relevant_pubkeys {:?}", relevant_pubkeys);
    // update handling thread, reads both slots and account updates
    tokio::spawn(async move {
        loop {
            if exit.load(Ordering::Relaxed) {
                warn!("shutting down orderbook_filter...");
                break;
            }
            tokio::select! {
                Ok(account_write) = account_write_queue_receiver.recv() => {
                    if !relevant_pubkeys.contains(&account_write.pubkey) {
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
            }

            chain_data_metrics.report(&chain_cache);

            for mkt in market_configs.iter() {
                for side in 0..2 {
                    let mkt_pk = mkt.0;
                    let side_pk = if side == 0 { mkt.1.bids } else { mkt.1.asks };
                    let other_side_pk = if side == 0 { mkt.1.asks } else { mkt.1.bids };
                    let oracle_pk = mkt.1.oracle;
                    let last_side_write_version = last_write_versions
                        .get(&side_pk.to_string())
                        .unwrap_or(&(0, 0));
                    let last_oracle_write_version = last_write_versions
                        .get(&oracle_pk.to_string())
                        .unwrap_or(&(0, 0));

                    match (
                        chain_cache.account(&side_pk),
                        chain_cache.account(&oracle_pk),
                    ) {
                        (Ok(side_info), Ok(oracle_info)) => {
                            let side_pk_string = side_pk.to_string();
                            let oracle_pk_string = oracle_pk.to_string();

                            if !side_info
                                .is_newer_than(last_side_write_version.0, last_side_write_version.1)
                                && !oracle_info.is_newer_than(
                                    last_oracle_write_version.0,
                                    last_oracle_write_version.1,
                                )
                            {
                                // neither bookside nor oracle was updated
                                continue;
                            }
                            last_write_versions.insert(
                                side_pk_string.clone(),
                                (side_info.slot, side_info.write_version),
                            );
                            last_write_versions.insert(
                                oracle_pk_string.clone(),
                                (oracle_info.slot, oracle_info.write_version),
                            );

                            let keyed_account = KeyedSharedDataAccountReader {
                                key: oracle_pk,
                                shared: oracle_info.account.clone(),
                            };
                            let oracle_config = OracleConfigParams {
                                conf_filter: 100_000.0, // use a large value to never fail the confidence check
                                max_staleness_slots: None, // don't check oracle staleness to get an orderbook
                            };

                            if let Ok(unchecked_oracle_state) = oracle_state_unchecked(
                                &OracleAccountInfos::from_reader(&keyed_account),
                                mkt.1.base_decimals,
                            ) {
                                if unchecked_oracle_state
                                    .check_confidence_and_maybe_staleness(
                                        &oracle_config.to_oracle_config(),
                                        None, // force this to always return a price no matter how stale
                                    )
                                    .is_ok()
                                {
                                    let oracle_price = unchecked_oracle_state.price;
                                    let account = &side_info.account;
                                    let bookside: BookSide = BookSide::try_deserialize(
                                        solana_sdk::account::ReadableAccount::data(account)
                                            .borrow_mut(),
                                    )
                                    .unwrap();
                                    let side = match bookside.nodes.order_tree_type() {
                                        OrderTreeType::Bids => OrderbookSide::Bid,
                                        OrderTreeType::Asks => OrderbookSide::Ask,
                                    };
                                    let time_now = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs();
                                    let oracle_price_lots = (oracle_price
                                        * I80F48::from_num(mkt.1.base_lot_size)
                                        / I80F48::from_num(mkt.1.quote_lot_size))
                                    .to_num();
                                    let bookside: Vec<Order> = bookside
                                        .iter_valid(time_now, oracle_price_lots)
                                        .map(|item| Order {
                                            price: price_lots_to_ui_perp(
                                                item.price_lots,
                                                mkt.1.base_decimals,
                                                mkt.1.quote_decimals,
                                                mkt.1.base_lot_size,
                                                mkt.1.quote_lot_size,
                                            ),
                                            quantity: base_lots_to_ui_perp(
                                                item.node.quantity,
                                                mkt.1.base_decimals,
                                                mkt.1.base_lot_size,
                                            ),
                                            owner_pubkey: item.node.owner.to_string(),
                                        })
                                        .collect();

                                    let other_bookside =
                                        bookside_cache.get(&other_side_pk.to_string());

                                    match bookside_cache.get(&side_pk_string) {
                                        Some(old_bookside) => publish_changes(
                                            side_info.slot,
                                            side_info.write_version,
                                            mkt,
                                            side,
                                            &bookside,
                                            old_bookside,
                                            other_bookside,
                                            &book_update_sender,
                                            &mut metric_book_events_new,
                                            &mut metric_level_events_new,
                                        ),
                                        _ => info!(
                                            "bookside_cache could not find {}",
                                            side_pk_string
                                        ),
                                    }

                                    bookside_cache.insert(side_pk_string.clone(), bookside.clone());
                                }
                            }
                        }
                        (side, oracle) => debug!(
                            "chain_cache could not find for mkt={} side={} oracle={}",
                            mkt_pk,
                            side.is_err(),
                            oracle.is_err()
                        ),
                    }
                }
            }

            for mkt in serum_market_configs.iter() {
                for side in 0..2 {
                    let side_pk = if side == 0 { mkt.1.bids } else { mkt.1.asks };
                    let other_side_pk = if side == 0 { mkt.1.asks } else { mkt.1.bids };
                    let last_write_version = last_write_versions
                        .get(&side_pk.to_string())
                        .unwrap_or(&(0, 0));

                    match chain_cache.account(&side_pk) {
                        Ok(account_info) => {
                            let side_pk_string = side_pk.to_string();

                            let write_version = (account_info.slot, account_info.write_version);
                            // todo: should this be <= so we don't overwrite with old data received late?
                            if write_version <= *last_write_version {
                                continue;
                            }
                            last_write_versions.insert(side_pk_string.clone(), write_version);
                            debug!("W {}", mkt.1.name);
                            let account = &mut account_info.account.clone();
                            let data = account.data_as_mut_slice();
                            let len = data.len();
                            let inner = &mut data[5..len - 7];
                            let slab = Slab::new(&mut inner[size_of::<OrderBookStateHeader>()..]);

                            let bookside: Vec<Order> = slab
                                .iter(side == 0)
                                .map(|item| {
                                    let owner_bytes: [u8; 32] = bytemuck::cast(item.owner());
                                    Order {
                                        price: price_lots_to_ui(
                                            u64::from(item.price()) as i64,
                                            mkt.1.base_decimals,
                                            mkt.1.quote_decimals,
                                            mkt.1.base_lot_size,
                                            mkt.1.quote_lot_size,
                                        ),
                                        quantity: base_lots_to_ui(
                                            item.quantity() as i64,
                                            mkt.1.base_decimals,
                                            mkt.1.quote_decimals,
                                            mkt.1.base_lot_size,
                                            mkt.1.quote_lot_size,
                                        ),
                                        owner_pubkey: Pubkey::new_from_array(owner_bytes)
                                            .to_string(),
                                    }
                                })
                                .collect();

                            let other_bookside =
                                serum_bookside_cache.get(&other_side_pk.to_string());

                            match serum_bookside_cache.get(&side_pk_string) {
                                Some(old_bookside) => publish_changes(
                                    account_info.slot,
                                    account_info.write_version,
                                    mkt,
                                    if side == 0 {
                                        OrderbookSide::Bid
                                    } else {
                                        OrderbookSide::Ask
                                    },
                                    &bookside,
                                    old_bookside,
                                    other_bookside,
                                    &book_update_sender,
                                    &mut metric_book_events_new,
                                    &mut metric_level_events_new,
                                ),
                                _ => info!("bookside_cache could not find {}", side_pk_string),
                            }

                            serum_bookside_cache.insert(side_pk_string.clone(), bookside);
                        }
                        Err(_) => debug!("chain_cache could not find {}", side_pk),
                    }
                }
            }
        }
    });

    Ok((
        account_write_queue_sender,
        slot_queue_sender,
        book_update_receiver,
    ))
}
