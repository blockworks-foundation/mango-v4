use anchor_lang::prelude::*;

use crate::state::TokenIndex;

pub type PerpMarketIndex = u16;

#[account(zero_copy)]
pub struct PerpMarket {
    // todo
    /// metadata
    // pub meta_data: MetaData,

    /// mango group
    pub group: Pubkey,

    // todo better docs
    ///
    pub oracle: Pubkey,

    /// order book
    pub bids: Pubkey,
    pub asks: Pubkey,

    // todo better docs
    ///
    pub event_queue: Pubkey,

    /// number of quote native that reresents min tick
    /// e.g. base lot size 100, quote lot size 10, then tick i.e. price increment is 10/100 i.e. 1
    // todo: why signed?
    pub quote_lot_size: i64,
    /// represents number of base native quantity; greater than 0
    /// e.g. base decimals 6, base lot size 100, base position 10000, then
    /// UI position is 1
    // todo: why signed?
    pub base_lot_size: i64,

    // todo
    /// an always increasing number (except in case of socializing losses), incremented by
    /// funding delta, funding delta is difference between book and index price which needs to be paid every day,
    /// funding delta is measured per day - per base lots - the larger users position the more funding
    /// he pays, funding is always paid in quote
    // pub long_funding: I80F48,
    // pub short_funding: I80F48,
    // todo
    /// timestamp when funding was last updated
    // pub last_updated: u64,

    // todo
    /// This is i64 to keep consistent with the units of contracts, but should always be > 0
    // todo: why signed?
    // pub open_interest: i64,

    // todo
    /// number of orders generated
    pub seq_num: u64,

    // todo
    /// in native quote currency
    // pub fees_accrued: I80F48,

    // todo
    /// liquidity mining
    // pub liquidity_mining_info: LiquidityMiningInfo,

    // todo
    /// token vault which holds mango tokens to be disbursed as liquidity incentives for this perp market
    // pub mngo_vault: Pubkey,

    /// pda bump
    pub bump: u8,

    /// useful for looking up respective perp account
    pub perp_market_index: PerpMarketIndex,
    /// useful for looking up respective base token,
    /// note: is optional, since perp market can exist without a corresponding base token,
    /// should be TokenIndex::MAX in that case
    pub base_token_index: TokenIndex,
    /// useful for looking up respective quote token
    pub quote_token_index: TokenIndex,
}
