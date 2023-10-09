pub mod serum;

use serde::{ser::SerializeStruct, Serialize, Serializer};
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug)]
pub struct StatusResponse<'a> {
    pub success: bool,
    pub message: &'a str,
}

impl<'a> Serialize for StatusResponse<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Status", 2)?;
        state.serialize_field("success", &self.success)?;
        state.serialize_field("message", &self.message)?;

        state.end()
    }
}

#[derive(Clone, Debug)]
pub enum OrderbookSide {
    Bid = 0,
    Ask = 1,
}

impl Serialize for OrderbookSide {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            OrderbookSide::Bid => serializer.serialize_unit_variant("Side", 0, "bid"),
            OrderbookSide::Ask => serializer.serialize_unit_variant("Side", 1, "ask"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MarketConfig {
    pub name: String,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_queue: Pubkey,
    pub oracle: Pubkey,
    pub base_decimals: u8,
    pub quote_decimals: u8,
    pub base_lot_size: i64,
    pub quote_lot_size: i64,
}

pub fn base_lots_to_ui(
    native: i64,
    base_decimals: u8,
    _quote_decimals: u8,
    base_lot_size: i64,
    _quote_lot_size: i64,
) -> f64 {
    (native * base_lot_size) as f64 / 10i64.pow(base_decimals.into()) as f64
}

pub fn base_lots_to_ui_perp(native: i64, decimals: u8, base_lot_size: i64) -> f64 {
    native as f64 * (base_lot_size as f64 / (10i64.pow(decimals.into()) as f64))
}

pub fn price_lots_to_ui(
    native: i64,
    base_decimals: u8,
    quote_decimals: u8,
    base_lot_size: i64,
    quote_lot_size: i64,
) -> f64 {
    let base_multiplier = 10i64.pow(base_decimals.into());
    let quote_multiplier = 10i64.pow(quote_decimals.into());

    let left: u128 = native as u128 * quote_lot_size as u128 * base_multiplier as u128;
    let right: u128 = base_lot_size as u128 * quote_multiplier as u128;

    left as f64 / right as f64
}

pub fn spot_price_to_ui(
    native: i64,
    native_size: i64,
    base_decimals: u8,
    quote_decimals: u8,
) -> f64 {
    // TODO: account for fees
    ((native * 10i64.pow(base_decimals.into())) / (10i64.pow(quote_decimals.into()) * native_size))
        as f64
}

pub fn price_lots_to_ui_perp(
    native: i64,
    base_decimals: u8,
    quote_decimals: u8,
    base_lot_size: i64,
    quote_lot_size: i64,
) -> f64 {
    let decimals = base_decimals.checked_sub(quote_decimals).unwrap();
    let multiplier = 10u64.pow(decimals.into()) as f64;
    native as f64 * ((multiplier * quote_lot_size as f64) / base_lot_size as f64)
}
