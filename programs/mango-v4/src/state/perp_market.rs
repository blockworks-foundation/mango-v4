use std::mem::size_of;

use anchor_lang::prelude::*;
use fixed::types::I80F48;
use static_assertions::const_assert_eq;

use crate::state::orderbook::order_type::Side;
use crate::state::TokenIndex;
use crate::util::checked_math as cm;

pub type PerpMarketIndex = u16;

#[account(zero_copy)]
pub struct PerpMarket {
    pub name: [u8; 16],

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

    // These weights apply to the base asset, the quote token is always assumed to be
    // the health-reference token and have 1 for price and weights
    pub maint_asset_weight: I80F48,
    pub init_asset_weight: I80F48,
    pub maint_liab_weight: I80F48,
    pub init_liab_weight: I80F48,

    // TODO docs
    pub liquidation_fee: I80F48,
    pub maker_fee: I80F48,
    pub taker_fee: I80F48,

    /// pub long_funding: I80F48,
    /// pub short_funding: I80F48,
    /// pub funding_last_updated: u64,

    ///
    pub open_interest: i64,

    /// Total number of orders seen
    pub seq_num: u64,

    /// Fees accrued in native quote currency
    pub fees_accrued: I80F48,

    /// Liquidity mining metadata
    /// pub liquidity_mining_info: LiquidityMiningInfo,

    /// Token vault which holds mango tokens to be disbursed as liquidity incentives for this perp market
    /// pub mngo_vault: Pubkey,

    /// PDA bump
    pub bump: u8,
    pub reserved: [u8; 1],

    /// Lookup indices
    pub perp_market_index: PerpMarketIndex,
    pub base_token_index: TokenIndex,

    /// Cannot be chosen freely, must be the health-reference token, same for all PerpMarkets
    pub quote_token_index: TokenIndex,
}

const_assert_eq!(
    size_of::<PerpMarket>(),
    16 + 32 * 5 + 8 * 2 + 16 * 7 + 8 * 2 + 16 + 8
);
const_assert_eq!(size_of::<PerpMarket>() % 8, 0);

impl PerpMarket {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn gen_order_id(&mut self, side: Side, price: i64) -> i128 {
        self.seq_num += 1;

        let upper = (price as i128) << 64;
        match side {
            Side::Bid => upper | (!self.seq_num as i128),
            Side::Ask => upper | (self.seq_num as i128),
        }
    }

    /// Convert from the price stored on the book to the price used in value calculations
    pub fn lot_to_native_price(&self, price: i64) -> I80F48 {
        I80F48::from_num(price)
            .checked_mul(I80F48::from_num(self.quote_lot_size))
            .unwrap()
            .checked_div(I80F48::from_num(self.base_lot_size))
            .unwrap()
    }

    pub fn native_price_to_lot(&self, price: I80F48) -> i64 {
        price
            .checked_mul(I80F48::from_num(self.base_lot_size))
            .unwrap()
            .checked_div(I80F48::from_num(self.quote_lot_size))
            .unwrap()
            .to_num()
    }

    /// Is `native_price` an acceptable order for the `side` of this market, given `oracle_price`?
    pub fn inside_price_limit(
        &self,
        side: Side,
        native_price: I80F48,
        oracle_price: I80F48,
    ) -> bool {
        match side {
            Side::Bid => native_price <= cm!(self.maint_liab_weight * oracle_price),
            Side::Ask => native_price >= cm!(self.maint_asset_weight * oracle_price),
        }
    }
}
