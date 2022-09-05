use std::mem::size_of;

use anchor_lang::prelude::*;
use fixed::types::I80F48;

use static_assertions::const_assert_eq;

use crate::state::orderbook::order_type::Side;
use crate::state::TokenIndex;
use crate::util::checked_math as cm;

use super::{Book, OracleConfig, DAY_I80F48};

pub type PerpMarketIndex = u16;

#[account(zero_copy)]
#[derive(Debug)]
pub struct PerpMarket {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,

    // TODO: Remove!
    // ABI: Clients rely on this being at offset 40
    pub base_token_index: TokenIndex,

    /// Lookup indices
    pub perp_market_index: PerpMarketIndex,

    pub padding1: [u8; 4],

    pub name: [u8; 16],

    pub oracle: Pubkey,

    pub oracle_config: OracleConfig,

    pub bids: Pubkey,
    pub asks: Pubkey,

    pub event_queue: Pubkey,

    /// Number of quote native that reresents min tick
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

    pub min_funding: I80F48,
    pub max_funding: I80F48,
    pub impact_quantity: i64,
    pub long_funding: I80F48,
    pub short_funding: I80F48,
    pub funding_last_updated: i64,

    ///
    pub open_interest: i64,

    /// Total number of orders seen
    pub seq_num: u64,

    /// Fees accrued in native quote currency
    pub fees_accrued: I80F48,

    /// Fees settled in native quote currency
    pub fees_settled: I80F48,

    /// Liquidity mining metadata
    /// pub liquidity_mining_info: LiquidityMiningInfo,

    /// Token vault which holds mango tokens to be disbursed as liquidity incentives for this perp market
    /// pub mngo_vault: Pubkey,

    /// PDA bump
    pub bump: u8,

    pub base_token_decimals: u8,

    pub padding2: [u8; 6],

    pub registration_time: i64,

    pub reserved: [u8; 128],
}

const_assert_eq!(
    size_of::<PerpMarket>(),
    32 + 2 + 2 + 4 + 16 + 32 + 16 + 32 * 3 + 8 * 2 + 16 * 12 + 8 * 2 + 8 * 2 + 16 + 2 + 6 + 8 + 128
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

    /// Use current order book price and index price to update the instantaneous funding
    pub fn update_funding(&mut self, book: &Book, oracle_price: I80F48, now_ts: u64) -> Result<()> {
        let index_price = oracle_price;

        // Get current book price & compare it to index price
        let bid = book.get_impact_price(Side::Bid, self.impact_quantity, now_ts);
        let ask = book.get_impact_price(Side::Ask, self.impact_quantity, now_ts);

        let diff_price = match (bid, ask) {
            (Some(bid), Some(ask)) => {
                // calculate mid-market rate
                let mid_price = bid.checked_add(ask).unwrap() / 2;
                let book_price = self.lot_to_native_price(mid_price);
                let diff = cm!(book_price / index_price - I80F48::ONE);
                diff.clamp(self.min_funding, self.max_funding)
            }
            (Some(_bid), None) => self.max_funding,
            (None, Some(_ask)) => self.min_funding,
            (None, None) => I80F48::ZERO,
        };

        let diff_ts = I80F48::from_num(now_ts - self.funding_last_updated as u64);
        let time_factor = cm!(diff_ts / DAY_I80F48);
        let base_lot_size = I80F48::from_num(self.base_lot_size);
        let funding_delta = cm!(index_price * diff_price * base_lot_size * time_factor);

        self.long_funding += funding_delta;
        self.short_funding += funding_delta;
        self.funding_last_updated = now_ts as i64;

        Ok(())
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
