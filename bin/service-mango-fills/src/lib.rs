use std::convert::{identity, TryFrom};

use anchor_lang::prelude::Pubkey;
use bytemuck::cast_slice;
use chrono::{TimeZone, Utc};
use mango_feeds_lib::{base_lots_to_ui_perp, price_lots_to_ui_perp, MarketConfig, OrderbookSide};
use mango_v4::state::{FillEvent as PerpFillEvent, Side};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use serum_dex::state::EventView as SpotEvent;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FillUpdateStatus {
    New,
    Revoke,
}

impl Serialize for FillUpdateStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            FillUpdateStatus::New => {
                serializer.serialize_unit_variant("FillUpdateStatus", 0, "new")
            }
            FillUpdateStatus::Revoke => {
                serializer.serialize_unit_variant("FillUpdateStatus", 1, "revoke")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FillEventType {
    Spot,
    Perp,
}

impl Serialize for FillEventType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            FillEventType::Spot => serializer.serialize_unit_variant("FillEventType", 0, "spot"),
            FillEventType::Perp => serializer.serialize_unit_variant("FillEventType", 1, "perp"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FillEvent {
    pub event_type: FillEventType,
    pub maker: String,
    pub taker: String,
    pub taker_side: OrderbookSide,
    pub timestamp: u64, // make all strings
    pub seq_num: u64,
    pub maker_client_order_id: u64,
    pub taker_client_order_id: u64,
    pub maker_fee: f32,
    pub taker_fee: f32,
    pub price: f64,
    pub quantity: f64,
}

impl Serialize for FillEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("FillEvent", 12)?;
        state.serialize_field("eventType", &self.event_type)?;
        state.serialize_field("maker", &self.maker)?;
        state.serialize_field("taker", &self.taker)?;
        state.serialize_field("takerSide", &self.taker_side)?;
        state.serialize_field(
            "timestamp",
            &Utc.timestamp_opt(self.timestamp as i64, 0)
                .unwrap()
                .to_rfc3339(),
        )?;
        state.serialize_field("seqNum", &self.seq_num)?;
        state.serialize_field("makerClientOrderId", &self.maker_client_order_id)?;
        state.serialize_field("takerClientOrderId", &self.taker_client_order_id)?; // make string
        state.serialize_field("makerFee", &self.maker_fee)?;
        state.serialize_field("takerFee", &self.taker_fee)?;
        state.serialize_field("price", &self.price)?;
        state.serialize_field("quantity", &self.quantity)?;
        state.end()
    }
}

impl FillEvent {
    pub fn new_from_perp(event: PerpFillEvent, config: &MarketConfig) -> Self {
        let taker_side = match event.taker_side() {
            Side::Ask => OrderbookSide::Ask,
            Side::Bid => OrderbookSide::Bid,
        };
        let price = price_lots_to_ui_perp(
            event.price,
            config.base_decimals,
            config.quote_decimals,
            config.base_lot_size,
            config.quote_lot_size,
        );
        let quantity =
            base_lots_to_ui_perp(event.quantity, config.base_decimals, config.base_lot_size);
        FillEvent {
            event_type: FillEventType::Perp,
            maker: event.maker.to_string(),
            taker: event.taker.to_string(),
            taker_side,
            timestamp: event.timestamp,
            seq_num: event.seq_num,
            maker_client_order_id: event.maker_client_order_id,
            taker_client_order_id: event.taker_client_order_id,
            maker_fee: event.maker_fee,
            taker_fee: event.taker_fee,
            price,
            quantity,
        }
    }

    pub fn new_from_spot(
        maker_event: SpotEvent,
        taker_event: SpotEvent,
        timestamp: u64,
        seq_num: u64,
        config: &MarketConfig,
    ) -> Self {
        match (maker_event, taker_event) {
            (
                SpotEvent::Fill {
                    side: maker_side,
                    client_order_id: maker_client_order_id,
                    native_qty_paid: maker_native_qty_paid,
                    native_fee_or_rebate: maker_native_fee_or_rebate,
                    native_qty_received: maker_native_qty_received,
                    owner: maker_owner,
                    ..
                },
                SpotEvent::Fill {
                    side: taker_side,
                    client_order_id: taker_client_order_id,
                    native_fee_or_rebate: taker_native_fee_or_rebate,
                    owner: taker_owner,
                    ..
                },
            ) => {
                let maker_side = match maker_side as u8 {
                    0 => OrderbookSide::Bid,
                    1 => OrderbookSide::Ask,
                    _ => panic!("invalid side"),
                };
                let taker_side = match taker_side as u8 {
                    0 => OrderbookSide::Bid,
                    1 => OrderbookSide::Ask,
                    _ => panic!("invalid side"),
                };
                let maker_client_order_id: u64 = match maker_client_order_id {
                    Some(id) => id.into(),
                    None => 0u64,
                };
                let taker_client_order_id: u64 = match taker_client_order_id {
                    Some(id) => id.into(),
                    None => 0u64,
                };

                let base_multiplier = 10u64.pow(config.base_decimals.into());
                let quote_multiplier = 10u64.pow(config.quote_decimals.into());

                let (price, quantity) = match maker_side {
                    OrderbookSide::Bid => {
                        let price_before_fees = maker_native_qty_paid + maker_native_fee_or_rebate;

                        let top = price_before_fees * base_multiplier;
                        let bottom = quote_multiplier * maker_native_qty_received;
                        let price = top as f64 / bottom as f64;
                        let quantity = maker_native_qty_received as f64 / base_multiplier as f64;
                        (price, quantity)
                    }
                    OrderbookSide::Ask => {
                        let price_before_fees =
                            maker_native_qty_received - maker_native_fee_or_rebate;

                        let top = price_before_fees * base_multiplier;
                        let bottom = quote_multiplier * maker_native_qty_paid;
                        let price = top as f64 / bottom as f64;
                        let quantity = maker_native_qty_paid as f64 / base_multiplier as f64;
                        (price, quantity)
                    }
                };

                let maker_fee = maker_native_fee_or_rebate as f32 / quote_multiplier as f32;
                let taker_fee = taker_native_fee_or_rebate as f32 / quote_multiplier as f32;

                FillEvent {
                    event_type: FillEventType::Spot,
                    maker: Pubkey::try_from(cast_slice(&identity(maker_owner) as &[_]))
                        .unwrap()
                        .to_string(),
                    taker: Pubkey::try_from(cast_slice(&identity(taker_owner) as &[_]))
                        .unwrap()
                        .to_string(),
                    taker_side,
                    timestamp,
                    seq_num,
                    maker_client_order_id,
                    taker_client_order_id,
                    taker_fee,
                    maker_fee,
                    price,
                    quantity,
                }
            }
            (_, _) => {
                panic!("Can't build FillEvent from SpotEvent::Out")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct FillUpdate {
    pub event: FillEvent,
    pub status: FillUpdateStatus,
    pub market_key: String,
    pub market_name: String,
    pub slot: u64,
    pub write_version: u64,
}

impl Serialize for FillUpdate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("FillUpdate", 6)?;
        state.serialize_field("event", &self.event)?;
        state.serialize_field("marketKey", &self.market_key)?;
        state.serialize_field("marketName", &self.market_name)?;
        state.serialize_field("status", &self.status)?;
        state.serialize_field("slot", &self.slot)?;
        state.serialize_field("writeVersion", &self.write_version)?;

        state.end()
    }
}

#[derive(Clone, Debug)]
pub struct HeadUpdate {
    pub head: usize,
    pub prev_head: usize,
    pub head_seq_num: u64,
    pub prev_head_seq_num: u64,
    pub status: FillUpdateStatus,
    pub market_key: String,
    pub market_name: String,
    pub slot: u64,
    pub write_version: u64,
}
impl Serialize for HeadUpdate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("HeadUpdate", 6)?;
        state.serialize_field("head", &self.head)?;
        state.serialize_field("previousHead", &self.prev_head)?;
        state.serialize_field("headSeqNum", &self.head_seq_num)?;
        state.serialize_field("previousHeadSeqNum", &self.prev_head_seq_num)?;
        state.serialize_field("marketKey", &self.market_key)?;
        state.serialize_field("marketName", &self.market_name)?;
        state.serialize_field("status", &self.status)?;
        state.serialize_field("slot", &self.slot)?;
        state.serialize_field("writeVersion", &self.write_version)?;

        state.end()
    }
}

#[derive(Clone, Debug)]
pub struct FillCheckpoint {
    pub market: String,
    pub queue: String,
    pub events: Vec<FillEvent>,
    pub slot: u64,
    pub write_version: u64,
}

impl Serialize for FillCheckpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("FillCheckpoint", 3)?;
        state.serialize_field("events", &self.events)?;
        state.serialize_field("market", &self.market)?;
        state.serialize_field("queue", &self.queue)?;
        state.serialize_field("slot", &self.slot)?;
        state.serialize_field("write_version", &self.write_version)?;

        state.end()
    }
}

pub enum FillEventFilterMessage {
    Update(FillUpdate),
    HeadUpdate(HeadUpdate),
    Checkpoint(FillCheckpoint),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "command")]
pub enum Command {
    #[serde(rename = "subscribe")]
    Subscribe(SubscribeCommand),
    #[serde(rename = "unsubscribe")]
    Unsubscribe(UnsubscribeCommand),
    #[serde(rename = "getMarkets")]
    GetMarkets,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeCommand {
    pub market_id: Option<String>,
    pub market_ids: Option<Vec<String>>,
    pub account_ids: Option<Vec<String>>,
    pub head_updates: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeCommand {
    pub market_id: String,
}
