pub mod postgres_types_numeric;
pub mod serum;

use serde::{ser::SerializeStruct, Serialize, Serializer};
use serde_derive::Deserialize;
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug, Deserialize)]
pub struct PostgresConfig {
    pub connection_string: String,
    /// Number of parallel postgres connections used for insertions
    pub connection_count: u64,
    /// Maximum batch size for inserts over one connection
    pub max_batch_size: usize,
    /// Max size of queues
    pub max_queue_size: usize,
    /// Number of queries retries before fatal error
    pub retry_query_max_count: u64,
    /// Seconds to sleep between query retries
    pub retry_query_sleep_secs: u64,
    /// Seconds to sleep between connection attempts
    pub retry_connection_sleep_secs: u64,
    /// Fatal error when the connection can't be reestablished this long
    pub fatal_connection_timeout_secs: u64,
    /// Allow invalid TLS certificates, passed to native_tls danger_accept_invalid_certs
    pub allow_invalid_certs: bool,
    pub tls: Option<PostgresTlsConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PostgresTlsConfig {
    /// CA Cert file or env var
    pub ca_cert_path: String,
    /// PKCS12 client cert path
    pub client_key_path: String,
}

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
