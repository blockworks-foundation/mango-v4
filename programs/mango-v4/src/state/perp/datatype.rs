use num_enum::{IntoPrimitive, TryFromPrimitive};

#[repr(u8)]
#[derive(IntoPrimitive, TryFromPrimitive)]
pub enum DataType {
    MangoGroup = 0,
    MangoAccount,
    RootBank,
    NodeBank,
    PerpMarket,
    Bids,
    Asks,
    MangoCache,
    EventQueue,
    AdvancedOrders,
    ReferrerMemory,
    ReferrerIdRecord,
}
