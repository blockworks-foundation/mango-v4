use mango_feeds_lib::OrderbookSide;
use serde::{ser::SerializeStruct, Serialize, Serializer};

pub type OrderbookLevel = [f64; 2];
pub type Orderbook = Vec<Order>;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub price: f64,
    pub quantity: f64,
    pub owner_pubkey: String,
}

#[derive(Clone, Debug)]
pub struct LevelUpdate {
    pub market: String,
    pub side: OrderbookSide,
    pub update: Vec<OrderbookLevel>,
    pub slot: u64,
    pub write_version: u64,
}

impl Serialize for LevelUpdate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("LevelUpdate", 5)?;
        state.serialize_field("market", &self.market)?;
        state.serialize_field("side", &self.side)?;
        state.serialize_field("update", &self.update)?;
        state.serialize_field("slot", &self.slot)?;
        state.serialize_field("write_version", &self.write_version)?;

        state.end()
    }
}

#[derive(Clone, Debug)]
pub struct LevelCheckpoint {
    pub market: String,
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
    pub slot: u64,
    pub write_version: u64,
}

impl Serialize for LevelCheckpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("LevelCheckpoint", 3)?;
        state.serialize_field("market", &self.market)?;
        state.serialize_field("bids", &self.bids)?;
        state.serialize_field("asks", &self.asks)?;
        state.serialize_field("slot", &self.slot)?;
        state.serialize_field("write_version", &self.write_version)?;

        state.end()
    }
}

#[derive(Clone, Debug)]
pub struct BookUpdate {
    pub market: String,
    pub side: OrderbookSide,
    pub additions: Vec<Order>,
    pub removals: Vec<Order>,
    pub slot: u64,
    pub write_version: u64,
}

impl Serialize for BookUpdate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("BookUpdate", 6)?;
        state.serialize_field("market", &self.market)?;
        state.serialize_field("side", &self.side)?;
        state.serialize_field("additions", &self.additions)?;
        state.serialize_field("removals", &self.removals)?;
        state.serialize_field("slot", &self.slot)?;
        state.serialize_field("write_version", &self.write_version)?;

        state.end()
    }
}

#[derive(Clone, Debug)]
pub struct BookCheckpoint {
    pub market: String,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
    pub slot: u64,
    pub write_version: u64,
}

impl Serialize for BookCheckpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("LevelCheckpoint", 5)?;
        state.serialize_field("market", &self.market)?;
        state.serialize_field("bids", &self.bids)?;
        state.serialize_field("asks", &self.asks)?;
        state.serialize_field("slot", &self.slot)?;
        state.serialize_field("write_version", &self.write_version)?;

        state.end()
    }
}

pub enum OrderbookFilterMessage {
    LevelUpdate(LevelUpdate),
    LevelCheckpoint(LevelCheckpoint),
    BookUpdate(BookUpdate),
    BookCheckpoint(BookCheckpoint),
}
