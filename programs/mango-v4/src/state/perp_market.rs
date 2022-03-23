use anchor_lang::prelude::*;

use crate::state::orderbook::order_type::Side;
use crate::state::TokenIndex;

pub type PerpMarketIndex = u16;

#[account(zero_copy)]
pub struct PerpMarket {
    pub group: Pubkey,

    pub oracle: Pubkey,

    pub bids: Pubkey,
    pub asks: Pubkey,

    pub event_queue: Pubkey,

    /// Number of quote native that reresents min tick
    /// e.g. when base lot size is 100, and quote lot size is 10, then tick i.e. price increment is 10/100 i.e. 0.1
    pub quote_lot_size: i64,

    /// Represents number of base native quantity
    /// e.g. if base decimals for underlying asset are 6, base lot size is 100, and base position is 10000, then
    /// UI position is 1
    pub base_lot_size: i64,

    /// pub long_funding: I80F48,
    /// pub short_funding: I80F48,
    /// pub funding_last_updated: u64,

    /// pub open_interest: u64,

    /// Total number of orders seen
    pub seq_num: u64,

    /// Fees accrued in native quote currency
    /// pub fees_accrued: I80F48,

    /// Liquidity mining metadata
    /// pub liquidity_mining_info: LiquidityMiningInfo,

    /// Token vault which holds mango tokens to be disbursed as liquidity incentives for this perp market
    /// pub mngo_vault: Pubkey,

    /// PDA bump
    pub bump: u8,

    /// Lookup indices
    pub perp_market_index: PerpMarketIndex,
    pub base_token_index: TokenIndex,
    pub quote_token_index: TokenIndex,
}

impl PerpMarket {
    /// TODO why is this based on price?
    pub fn gen_order_id(&mut self, side: Side, price: i64) -> i128 {
        self.seq_num += 1;

        let upper = (price as i128) << 64;
        match side {
            Side::Bid => upper | (!self.seq_num as i128),
            Side::Ask => upper | (self.seq_num as i128),
        }
    }
}
