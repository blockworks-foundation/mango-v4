use fixed::types::I80F48;

pub struct PerpAccount {
    pub base_position: i64,     // measured in base lots
    pub quote_position: I80F48, // measured in native quote

    pub long_settled_funding: I80F48,
    pub short_settled_funding: I80F48,

    // orders related info
    pub bids_quantity: i64, // total contracts in sell orders
    pub asks_quantity: i64, // total quote currency in buy orders

    /// Amount that's on EventQueue waiting to be processed
    pub taker_base: i64,
    pub taker_quote: i64,

    pub mngo_accrued: u64,
}
