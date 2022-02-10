use anchor_lang::prelude::*;
use fixed::types::I80F48;

pub struct LiquidityMiningInfo {
    /// Used to convert liquidity points to MNGO
    pub rate: I80F48,

    pub max_depth_bps: I80F48, // instead of max depth bps, this should be max num contracts

    /// start timestamp of current liquidity incentive period; gets updated when mngo_left goes to 0
    pub period_start: u64,

    /// Target time length of a period in seconds
    pub target_period_length: u64,

    /// Paper MNGO left for this period
    pub mngo_left: u64,

    /// Total amount of MNGO allocated for current period
    pub mngo_per_period: u64,
}

pub struct PerpMarket {
    pub meta_data: MetaData,

    pub mango_group: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_queue: Pubkey,
    pub quote_lot_size: i64, // number of quote native that reresents min tick
    pub base_lot_size: i64,  // represents number of base native quantity; greater than 0

    // TODO - consider just moving this into the cache
    pub long_funding: I80F48,
    pub short_funding: I80F48,

    pub open_interest: i64, // This is i64 to keep consistent with the units of contracts, but should always be > 0

    pub last_updated: u64,
    pub seq_num: u64,
    pub fees_accrued: I80F48, // native quote currency

    pub liquidity_mining_info: LiquidityMiningInfo,

    // mngo_vault holds mango tokens to be disbursed as liquidity incentives for this perp market
    pub mngo_vault: Pubkey,
}
