use anchor_lang::prelude::*;

use derivative::Derivative;
use fixed::types::I80F48;
use static_assertions::const_assert_eq;
use std::cmp::Ordering;
use std::mem::size_of;

use crate::i80f48::ClampToInt;
use crate::state::*;

pub const FREE_ORDER_SLOT: PerpMarketIndex = PerpMarketIndex::MAX;

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Derivative, bytemuck::Pod, bytemuck::Zeroable)]
#[derivative(Debug)]
pub struct TokenPosition {
    // TODO: Why did we have deposits and borrows as two different values
    //       if only one of them was allowed to be != 0 at a time?
    // todo: maybe we want to split collateral and lending?
    // todo: see https://github.com/blockworks-foundation/mango-v4/issues/1
    // todo: how does ftx do this?
    /// The deposit_index (if positive) or borrow_index (if negative) scaled position
    pub indexed_position: I80F48,

    /// index into Group.tokens
    pub token_index: TokenIndex,

    /// incremented when a market requires this position to stay alive
    pub in_use_count: u8,

    #[derivative(Debug = "ignore")]
    pub padding: [u8; 5],

    // bookkeeping variable for onchain interest calculation
    // either deposit_index or borrow_index at last indexed_position change
    pub previous_index: I80F48,
    // (Display only)
    // Cumulative deposit interest in token native units
    pub cumulative_deposit_interest: f64,
    // (Display only)
    // Cumulative borrow interest in token native units
    pub cumulative_borrow_interest: f64,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 128],
}

const_assert_eq!(
    size_of::<TokenPosition>(),
    16 + 2 + 1 + 5 + 16 + 8 + 8 + 128
);
const_assert_eq!(size_of::<TokenPosition>(), 184);
const_assert_eq!(size_of::<TokenPosition>() % 8, 0);

impl Default for TokenPosition {
    fn default() -> Self {
        TokenPosition {
            indexed_position: I80F48::ZERO,
            token_index: TokenIndex::MAX,
            in_use_count: 0,
            cumulative_deposit_interest: 0.0,
            cumulative_borrow_interest: 0.0,
            previous_index: I80F48::ZERO,
            padding: Default::default(),
            reserved: [0; 128],
        }
    }
}

impl TokenPosition {
    pub fn is_active(&self) -> bool {
        self.token_index != TokenIndex::MAX
    }

    pub fn is_active_for_token(&self, token_index: TokenIndex) -> bool {
        self.token_index == token_index
    }

    pub fn native(&self, bank: &Bank) -> I80F48 {
        if self.indexed_position.is_positive() {
            self.indexed_position * bank.deposit_index
        } else {
            self.indexed_position * bank.borrow_index
        }
    }

    #[cfg(feature = "client")]
    pub fn ui(&self, bank: &Bank) -> I80F48 {
        if self.indexed_position.is_positive() {
            (self.indexed_position * bank.deposit_index)
                / I80F48::from_num(10u64.pow(bank.mint_decimals as u32))
        } else {
            (self.indexed_position * bank.borrow_index)
                / I80F48::from_num(10u64.pow(bank.mint_decimals as u32))
        }
    }

    pub fn is_in_use(&self) -> bool {
        self.in_use_count > 0
    }
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize, Derivative, bytemuck::Pod, bytemuck::Zeroable)]
#[derivative(Debug)]
pub struct Serum3Orders {
    pub open_orders: Pubkey,

    /// Tracks the amount of borrows that have flowed into the serum open orders account.
    /// These borrows did not have the loan origination fee applied, and that may happen
    /// later (in serum3_settle_funds) if we can guarantee that the funds were used.
    /// In particular a place-on-book, cancel, settle should not cost fees.
    pub base_borrows_without_fee: u64,
    pub quote_borrows_without_fee: u64,

    pub market_index: Serum3MarketIndex,

    /// Store the base/quote token index, so health computations don't need
    /// to get passed the static SerumMarket to find which tokens a market
    /// uses and look up the correct oracles.
    pub base_token_index: TokenIndex,
    pub quote_token_index: TokenIndex,

    #[derivative(Debug = "ignore")]
    pub padding: [u8; 2],

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 64],
}
const_assert_eq!(size_of::<Serum3Orders>(), 32 + 8 * 2 + 2 * 3 + 2 + 64);
const_assert_eq!(size_of::<Serum3Orders>(), 120);
const_assert_eq!(size_of::<Serum3Orders>() % 8, 0);

impl Serum3Orders {
    pub fn is_active(&self) -> bool {
        self.market_index != Serum3MarketIndex::MAX
    }

    pub fn is_active_for_market(&self, market_index: Serum3MarketIndex) -> bool {
        self.market_index == market_index
    }
}

impl Default for Serum3Orders {
    fn default() -> Self {
        Self {
            open_orders: Pubkey::default(),
            market_index: Serum3MarketIndex::MAX,
            base_token_index: TokenIndex::MAX,
            quote_token_index: TokenIndex::MAX,
            reserved: [0; 64],
            padding: Default::default(),
            base_borrows_without_fee: 0,
            quote_borrows_without_fee: 0,
        }
    }
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize, Derivative, bytemuck::Pod, bytemuck::Zeroable)]
#[derivative(Debug)]
pub struct PerpPosition {
    pub market_index: PerpMarketIndex,

    #[derivative(Debug = "ignore")]
    pub padding: [u8; 2],

    /// Index of the current settle pnl limit window
    pub settle_pnl_limit_window: u32,

    /// Amount of realized trade pnl and unrealized pnl that was already settled this window.
    ///
    /// Will be negative when negative pnl was settled.
    ///
    /// Note that this will be adjusted for bookkeeping reasons when the realized_trade settle
    /// limitchanges and is not useable for actually tracking how much pnl was settled
    /// on balance.
    pub settle_pnl_limit_settled_in_current_window_native: i64,

    /// Active position size, measured in base lots
    pub base_position_lots: i64,
    /// Active position in quote (conversation rate is that of the time the order was settled)
    /// measured in native quote
    pub quote_position_native: I80F48,

    /// Tracks what the position is to calculate average entry & break even price
    pub quote_running_native: i64,

    /// Already settled long funding
    pub long_settled_funding: I80F48,
    /// Already settled short funding
    pub short_settled_funding: I80F48,

    /// Base lots in open bids
    pub bids_base_lots: i64,
    /// Base lots in open asks
    pub asks_base_lots: i64,

    /// Amount of base lots on the EventQueue waiting to be processed
    pub taker_base_lots: i64,
    /// Amount of quote lots on the EventQueue waiting to be processed
    pub taker_quote_lots: i64,

    /// Cumulative long funding in quote native units.
    /// If the user paid $1 in funding for a long position, this would be 1e6.
    /// Beware of the sign!
    ///
    /// (Display only)
    pub cumulative_long_funding: f64,
    /// Cumulative short funding in quote native units
    /// If the user paid $1 in funding for a short position, this would be -1e6.
    ///
    /// (Display only)
    pub cumulative_short_funding: f64,

    /// Cumulative maker volume in quote native units
    ///
    /// (Display only)
    pub maker_volume: u64,
    /// Cumulative taker volume in quote native units
    ///
    /// (Display only)
    pub taker_volume: u64,

    /// Cumulative number of quote native units transfered from the perp position
    /// to the settle token spot position.
    ///
    /// For example, if the user settled $1 of positive pnl into their USDC spot
    /// position, this would be 1e6.
    ///
    /// (Display only)
    pub perp_spot_transfers: i64,

    /// The native average entry price for the base lots of the current position.
    /// Reset to 0 when the base position reaches or crosses 0.
    pub avg_entry_price_per_base_lot: f64,

    /// Amount of pnl that was realized by bringing the base position closer to 0.
    ///
    /// The settlement of this type of pnl is limited by settle_pnl_limit_realized_trade.
    /// Settling pnl reduces this value once other_pnl below is exhausted.
    pub realized_trade_pnl_native: I80F48,

    /// Amount of pnl realized from fees, funding and liquidation.
    ///
    /// This type of realized pnl is always settleable.
    /// Settling pnl reduces this value first.
    pub realized_other_pnl_native: I80F48,

    /// Settle limit contribution from realized pnl.
    ///
    /// Every time pnl is realized, this is increased by a fraction of the stable
    /// value of the realization. It magnitude decreases when realized pnl drops below its value.
    pub settle_pnl_limit_realized_trade: i64,

    /// Trade pnl, fees, funding that were added over the current position's lifetime.
    ///
    /// Reset when the position changes sign or goes to zero.
    /// Not decreased by settling.
    ///
    /// This is tracked for display purposes: this value plus the difference between entry
    /// price and current price of the base position is the overall pnl.
    pub realized_pnl_for_position_native: I80F48,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 88],
}
const_assert_eq!(
    size_of::<PerpPosition>(),
    2 + 2 + 4 + 8 + 8 + 16 + 8 + 16 * 2 + 8 * 2 + 8 * 2 + 8 * 5 + 8 + 2 * 16 + 8 + 16 + 88
);
const_assert_eq!(size_of::<PerpPosition>(), 304);
const_assert_eq!(size_of::<PerpPosition>() % 8, 0);

impl Default for PerpPosition {
    fn default() -> Self {
        Self {
            market_index: PerpMarketIndex::MAX,
            base_position_lots: 0,
            quote_position_native: I80F48::ZERO,
            quote_running_native: 0,
            bids_base_lots: 0,
            asks_base_lots: 0,
            taker_base_lots: 0,
            taker_quote_lots: 0,
            long_settled_funding: I80F48::ZERO,
            short_settled_funding: I80F48::ZERO,
            padding: Default::default(),
            cumulative_long_funding: 0.0,
            cumulative_short_funding: 0.0,
            maker_volume: 0,
            taker_volume: 0,
            perp_spot_transfers: 0,
            avg_entry_price_per_base_lot: 0.0,
            realized_trade_pnl_native: I80F48::ZERO,
            realized_other_pnl_native: I80F48::ZERO,
            settle_pnl_limit_window: 0,
            settle_pnl_limit_settled_in_current_window_native: 0,
            settle_pnl_limit_realized_trade: 0,
            realized_pnl_for_position_native: I80F48::ZERO,
            reserved: [0; 88],
        }
    }
}

impl PerpPosition {
    /// Add taker trade after it has been matched but before it has been process on EventQueue
    pub fn add_taker_trade(&mut self, side: Side, base_lots: i64, quote_lots: i64) {
        match side {
            Side::Bid => {
                self.taker_base_lots += base_lots;
                self.taker_quote_lots -= quote_lots;
            }
            Side::Ask => {
                self.taker_base_lots -= base_lots;
                self.taker_quote_lots += quote_lots;
            }
        }
    }
    /// Remove taker trade after it has been processed on EventQueue
    pub fn remove_taker_trade(&mut self, base_change: i64, quote_change: i64) {
        self.taker_base_lots -= base_change;
        self.taker_quote_lots -= quote_change;
    }

    pub fn is_active(&self) -> bool {
        self.market_index != PerpMarketIndex::MAX
    }

    pub fn is_active_for_market(&self, market_index: PerpMarketIndex) -> bool {
        self.market_index == market_index
    }

    // Return base position in native units for a perp market
    pub fn base_position_native(&self, market: &PerpMarket) -> I80F48 {
        I80F48::from(self.base_position_lots * market.base_lot_size)
    }

    pub fn base_position_lots(&self) -> i64 {
        self.base_position_lots
    }

    // This takes into account base lots from unprocessed events, but not anything from open orders
    pub fn effective_base_position_lots(&self) -> i64 {
        self.base_position_lots + self.taker_base_lots
    }

    pub fn quote_position_native(&self) -> I80F48 {
        self.quote_position_native
    }

    /// This assumes settle_funding was already called
    fn change_base_position(&mut self, perp_market: &mut PerpMarket, base_change: i64) {
        let start = self.base_position_lots;
        self.base_position_lots += base_change;
        perp_market.open_interest += self.base_position_lots.abs() - start.abs();
    }

    /// The amount of funding this account still needs to pay, in native quote
    pub fn unsettled_funding(&self, perp_market: &PerpMarket) -> I80F48 {
        match self.base_position_lots.cmp(&0) {
            Ordering::Greater => {
                (perp_market.long_funding - self.long_settled_funding)
                    * I80F48::from_num(self.base_position_lots)
            }
            Ordering::Less => {
                (perp_market.short_funding - self.short_settled_funding)
                    * I80F48::from_num(self.base_position_lots)
            }
            Ordering::Equal => I80F48::ZERO,
        }
    }

    /// Move unrealized funding payments into the quote_position
    pub fn settle_funding(&mut self, perp_market: &PerpMarket) {
        let funding = self.unsettled_funding(perp_market);
        self.quote_position_native -= funding;
        self.realized_other_pnl_native -= funding;
        self.realized_pnl_for_position_native -= funding;

        if self.base_position_lots.is_positive() {
            self.cumulative_long_funding += funding.to_num::<f64>();
        } else {
            self.cumulative_short_funding -= funding.to_num::<f64>();
        }

        self.long_settled_funding = perp_market.long_funding;
        self.short_settled_funding = perp_market.short_funding;
    }

    /// Updates avg entry price, breakeven price, realized pnl, realized pnl limit
    fn update_trade_stats(
        &mut self,
        base_change: i64,
        quote_change_native: I80F48,
        perp_market: &PerpMarket,
    ) {
        if base_change == 0 {
            return;
        }

        let old_position = self.base_position_lots;
        let new_position = old_position + base_change;

        // amount of lots that were reduced (so going from -5 to 10 lots is a reduction of 5)
        let reduced_lots;
        // amount of pnl that was realized by the reduction (signed)
        let newly_realized_pnl;

        if new_position == 0 {
            reduced_lots = -old_position;

            // clear out display fields that live only while the position lasts
            self.avg_entry_price_per_base_lot = 0.0;
            self.quote_running_native = 0;
            self.realized_pnl_for_position_native = I80F48::ZERO;

            // There can't be unrealized pnl without a base position, so fix the
            // realized_trade_pnl to cover everything that isn't realized_other_pnl.
            let total_realized_pnl = self.quote_position_native + quote_change_native;
            let new_realized_trade_pnl = total_realized_pnl - self.realized_other_pnl_native;
            newly_realized_pnl = new_realized_trade_pnl - self.realized_trade_pnl_native;
            self.realized_trade_pnl_native = new_realized_trade_pnl;
        } else if old_position.signum() != new_position.signum() {
            // If the base position changes sign, we've crossed base_pos == 0 (or old_position == 0)
            reduced_lots = -old_position;
            let old_position = old_position as f64;
            let new_position = new_position as f64;
            let base_change = base_change as f64;
            let old_avg_entry = self.avg_entry_price_per_base_lot;
            let new_avg_entry = (quote_change_native.to_num::<f64>() / base_change).abs();

            // Award realized pnl based on the old_position size
            newly_realized_pnl = I80F48::from_num(old_position * (new_avg_entry - old_avg_entry));
            self.realized_trade_pnl_native += newly_realized_pnl;

            // Set entry and break-even based on the new_position entered
            self.avg_entry_price_per_base_lot = new_avg_entry;
            self.quote_running_native = (-new_position * new_avg_entry) as i64;

            // New position without realized pnl
            self.realized_pnl_for_position_native = I80F48::ZERO;
        } else {
            // The old and new position have the same sign

            self.quote_running_native += quote_change_native.round_to_zero().to_num::<i64>();

            let is_increasing = old_position.signum() == base_change.signum();
            if is_increasing {
                // Increasing position: avg entry price updates, no new realized pnl
                reduced_lots = 0;
                newly_realized_pnl = I80F48::ZERO;
                let old_position_abs = old_position.abs() as f64;
                let new_position_abs = new_position.abs() as f64;
                let old_avg_entry = self.avg_entry_price_per_base_lot;
                let new_position_quote_value =
                    old_position_abs * old_avg_entry + quote_change_native.to_num::<f64>().abs();
                self.avg_entry_price_per_base_lot = new_position_quote_value / new_position_abs;
            } else {
                // Decreasing position: pnl is realized, avg entry price does not change
                reduced_lots = base_change;
                let avg_entry = I80F48::from_num(self.avg_entry_price_per_base_lot);
                newly_realized_pnl = quote_change_native + I80F48::from(base_change) * avg_entry;
                self.realized_trade_pnl_native += newly_realized_pnl;
                self.realized_pnl_for_position_native += newly_realized_pnl;
            }
        }

        // When realized limit has a different sign from realized pnl, reset it completely
        if (self.settle_pnl_limit_realized_trade > 0 && self.realized_trade_pnl_native <= 0)
            || (self.settle_pnl_limit_realized_trade < 0 && self.realized_trade_pnl_native >= 0)
        {
            self.settle_pnl_limit_realized_trade = 0;
        }

        // Whenever realized pnl increases in magnitude, also increase realized pnl settle limit
        // magnitude.
        if newly_realized_pnl.signum() == self.realized_trade_pnl_native.signum() {
            let realized_stable_value =
                I80F48::from(reduced_lots.abs() * perp_market.base_lot_size)
                    * perp_market.stable_price();
            let stable_value_fraction =
                I80F48::from_num(perp_market.settle_pnl_limit_factor) * realized_stable_value;

            // The realized pnl settle limit change is restricted to actually realized pnl:
            // buying and then selling some base lots at the same price shouldn't affect
            // the settle limit.
            let limit_change = if newly_realized_pnl > 0 {
                newly_realized_pnl
                    .min(stable_value_fraction)
                    .ceil()
                    .clamp_to_i64()
            } else {
                newly_realized_pnl
                    .max(-stable_value_fraction)
                    .floor()
                    .clamp_to_i64()
            };
            self.settle_pnl_limit_realized_trade += limit_change;
        }

        // Ensure the realized limit doesn't exceed the realized pnl
        self.apply_realized_trade_pnl_settle_limit_constraint(newly_realized_pnl);
    }

    /// The abs(realized pnl settle limit) should be roughly < abs(realized pnl).
    ///
    /// It's not always true, since realized_pnl can change with fees and funding
    /// without updating the realized pnl settle limit. And rounding also breaks it.
    ///
    /// This function applies that constraint and deals with bookkeeping.
    fn apply_realized_trade_pnl_settle_limit_constraint(
        &mut self,
        realized_trade_pnl_change: I80F48,
    ) {
        let new_limit = if self.realized_trade_pnl_native > 0 {
            self.settle_pnl_limit_realized_trade
                .min(self.realized_trade_pnl_native.ceil().clamp_to_i64())
                .max(0)
        } else {
            self.settle_pnl_limit_realized_trade
                .max(self.realized_trade_pnl_native.floor().clamp_to_i64())
                .min(0)
        };
        let limit_change = new_limit - self.settle_pnl_limit_realized_trade;
        self.settle_pnl_limit_realized_trade = new_limit;

        // If we reduce the budget for realized pnl settling we also need to decrease the
        // used-up settle amount to keep the freely settleable amount the same.
        //
        // Example: Settling the last remaining 50 realized pnl adds 50 to settled and brings the
        // realized pnl settle budget to 0 above. That means we reduced the budget _and_ used
        // up a part of it: it was double-counted. Instead bring the budget to 0 and don't increase
        // settled.
        //
        // Example: The same thing can happen with the opposite sign. Say you have
        //     -50 realized pnl
        //     -80 pnl overall
        //    +-30 unrealized pnl settle limit
        //     -40 realized pnl settle limit
        //       0 settle limit used
        //     -70 available settle limit
        //   Settling -60 would result in
        //       0 realized pnl
        //     -20 pnl overall
        //    +-30 unrealized pnl settle limit
        //       0 realized pnl settle limit
        //     -60 settle limit used
        //       0 available settle limit
        //   Which would mean no more unrealized pnl could be settled, when -10 more should be settleable!
        //   This function notices the realized pnl limit_change was 40 and adjusts the settle limit:
        //    +-30 unrealized pnl settle limit
        //       0 realized pnl settle limit
        //     -20 settle limit used
        //     -10 available settle limit

        // Sometimes realized_pnl gets reduced by non-settles such as funding or fees.
        // To avoid overcorrecting, the adjustment is limited to the realized_pnl change
        // passed into this function.
        let realized_pnl_change = realized_trade_pnl_change.round_to_zero().clamp_to_i64();
        let used_change = if limit_change >= 0 {
            limit_change.min(realized_pnl_change).max(0)
        } else {
            limit_change.max(realized_pnl_change).min(0)
        };

        self.settle_pnl_limit_settled_in_current_window_native += used_change;
    }

    /// Change the base and quote positions as the result of a trade
    pub fn record_trade(
        &mut self,
        perp_market: &mut PerpMarket,
        base_change: i64,
        quote_change_native: I80F48,
    ) {
        assert_eq!(perp_market.perp_market_index, self.market_index);
        self.update_trade_stats(base_change, quote_change_native, perp_market);
        self.change_base_position(perp_market, base_change);
        self.change_quote_position(quote_change_native);
    }

    fn change_quote_position(&mut self, quote_change_native: I80F48) {
        self.quote_position_native += quote_change_native;
    }

    /// Does the user have any orders on the book?
    ///
    /// Note that it's possible they were matched already: This only becomes
    /// false when the fill event is processed or the orders are cancelled.
    pub fn has_open_orders(&self) -> bool {
        self.asks_base_lots != 0 || self.bids_base_lots != 0
    }

    // Did the user take orders and hasn't been filled yet?
    pub fn has_open_taker_fills(&self) -> bool {
        self.taker_base_lots != 0 || self.taker_quote_lots != 0
    }

    /// Are there any open orders or fills that haven't been processed yet?
    pub fn has_open_orders_or_fills(&self) -> bool {
        self.has_open_orders() || self.has_open_taker_fills()
    }

    /// Calculate the average entry price of the position, in native/native units
    pub fn avg_entry_price(&self, market: &PerpMarket) -> f64 {
        assert_eq!(self.market_index, market.perp_market_index);
        self.avg_entry_price_per_base_lot / (market.base_lot_size as f64)
    }

    /// Calculate the break even price of the position, in native/native units
    pub fn break_even_price(&self, market: &PerpMarket) -> f64 {
        if self.base_position_lots == 0 {
            return 0.0;
        }
        assert_eq!(self.market_index, market.perp_market_index);
        -(self.quote_running_native as f64)
            / ((self.base_position_lots * market.base_lot_size) as f64)
    }

    /// Calculate the PnL of the position for a given price
    pub fn unsettled_pnl(&self, perp_market: &PerpMarket, price: I80F48) -> Result<I80F48> {
        require_eq!(self.market_index, perp_market.perp_market_index);
        let base_native = self.base_position_native(perp_market);
        let pnl = self.quote_position_native() + base_native * price;
        Ok(pnl)
    }

    /// Updates the perp pnl limit time windowing, resetting the amount
    /// of used settle-pnl budget if necessary
    pub fn update_settle_limit(&mut self, market: &PerpMarket, now_ts: u64) {
        assert_eq!(self.market_index, market.perp_market_index);
        let window_size = market.settle_pnl_limit_window_size_ts;
        let window_start = self.settle_pnl_limit_window as u64 * window_size;
        let window_end = window_start + window_size;
        // now_ts < window_start can happen when window size is changed on the market
        let new_window = now_ts >= window_end || now_ts < window_start;
        if new_window {
            self.settle_pnl_limit_window = (now_ts / window_size).try_into().unwrap();
            self.settle_pnl_limit_settled_in_current_window_native = 0;
        }
    }

    /// Returns the (min_pnl, max_pnl) range of quote-native pnl that can be settled this window.
    ///
    /// It contains contributions from three factors:
    /// - a fraction of the base position stable value, which gives settlement limit
    ///   equally in both directions
    /// - the stored realized trade settle limit, which adds an extra settlement allowance
    ///   in a single direction
    /// - the stored realized other settle limit, which adds an extra settlement allowance
    ///   in a single direction
    pub fn settle_limit(&self, market: &PerpMarket) -> (i64, i64) {
        assert_eq!(self.market_index, market.perp_market_index);
        if market.settle_pnl_limit_factor < 0.0 {
            return (i64::MIN, i64::MAX);
        }

        let base_native = self.base_position_native(market);
        let position_value = (market.stable_price() * base_native).abs().to_num::<f64>();
        let unrealized = (market.settle_pnl_limit_factor as f64 * position_value).clamp_to_i64();

        let mut min_pnl = -unrealized;
        let mut max_pnl = unrealized;

        let realized_trade = self.settle_pnl_limit_realized_trade;
        if realized_trade >= 0 {
            max_pnl = max_pnl.saturating_add(realized_trade);
        } else {
            min_pnl = min_pnl.saturating_add(realized_trade);
        };

        let realized_other = self.realized_other_pnl_native;
        if realized_other >= 0 {
            max_pnl = max_pnl.saturating_add(realized_other.ceil().clamp_to_i64());
        } else {
            min_pnl = min_pnl.saturating_add(realized_other.floor().clamp_to_i64());
        };

        // the min/max here is just for safety
        (min_pnl.min(0), max_pnl.max(0))
    }

    /// Returns the (min_pnl, max_pnl) range of quote-native pnl that may still be settled
    /// this settle window.
    ///
    /// The available settle limit is the settle_limit() adjusted for the amount of limit
    /// that was already used up this window.
    pub fn available_settle_limit(&self, market: &PerpMarket) -> (i64, i64) {
        assert_eq!(self.market_index, market.perp_market_index);
        if market.settle_pnl_limit_factor < 0.0 {
            return (i64::MIN, i64::MAX);
        }

        let (mut min_pnl, mut max_pnl) = self.settle_limit(market);
        let used = self.settle_pnl_limit_settled_in_current_window_native;

        min_pnl = min_pnl.saturating_sub(used).min(0);
        max_pnl = max_pnl.saturating_sub(used).max(0);

        (min_pnl, max_pnl)
    }

    /// Given some pnl, applies the pnl settle limit and returns the reduced pnl.
    pub fn apply_pnl_settle_limit(&self, market: &PerpMarket, pnl: I80F48) -> I80F48 {
        if market.settle_pnl_limit_factor < 0.0 {
            return pnl;
        }

        let (min_pnl, max_pnl) = self.available_settle_limit(market);
        if pnl < 0 {
            pnl.max(I80F48::from(min_pnl))
        } else {
            pnl.min(I80F48::from(max_pnl))
        }
    }

    /// Update the perp position for pnl settlement
    ///
    /// If `pnl` is positive, then that is settled away, deducting from the quote position.
    pub fn record_settle(&mut self, settled_pnl: I80F48) {
        self.change_quote_position(-settled_pnl);

        // Settlement reduces realized_other_pnl first.
        // Reduction only happens if settled_pnl has the same sign as realized_other_pnl.
        let other_reduction = if settled_pnl > 0 {
            settled_pnl
                .min(self.realized_other_pnl_native)
                .max(I80F48::ZERO)
        } else {
            settled_pnl
                .max(self.realized_other_pnl_native)
                .min(I80F48::ZERO)
        };
        self.realized_other_pnl_native -= other_reduction;
        let trade_and_unrealized_settlement = settled_pnl - other_reduction;

        // Then reduces realized_trade_pnl, similar to other_pnl above.
        let trade_reduction = if trade_and_unrealized_settlement > 0 {
            trade_and_unrealized_settlement
                .min(self.realized_trade_pnl_native)
                .max(I80F48::ZERO)
        } else {
            trade_and_unrealized_settlement
                .max(self.realized_trade_pnl_native)
                .min(I80F48::ZERO)
        };
        self.realized_trade_pnl_native -= trade_reduction;

        // Consume settle limit budget: We don't track consumption of realized_other_pnl
        // because settling it directly reduces its budget as well.
        let settled_pnl_i64 = trade_and_unrealized_settlement
            .round_to_zero()
            .clamp_to_i64();
        self.settle_pnl_limit_settled_in_current_window_native += settled_pnl_i64;

        self.apply_realized_trade_pnl_settle_limit_constraint(-trade_reduction)
    }

    /// Update perp position for a maker/taker fee payment
    pub fn record_trading_fee(&mut self, fee: I80F48) {
        self.change_quote_position(-fee);
        self.realized_other_pnl_native -= fee;
        self.realized_pnl_for_position_native -= fee;
    }

    /// Adds immediately-settleable realized pnl when a liqor takes over pnl during liquidation
    pub fn record_liquidation_quote_change(&mut self, change: I80F48) {
        self.change_quote_position(change);
        self.realized_other_pnl_native += change;
    }

    /// Adds to the quote position and adds a recurring ("realized trade") settle limit
    pub fn record_liquidation_pnl_takeover(&mut self, change: I80F48, recurring_limit: I80F48) {
        self.change_quote_position(change);
        self.realized_trade_pnl_native += recurring_limit;
    }
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PerpOpenOrder {
    pub side_and_tree: u8, // SideAndOrderTree -- enums aren't POD
    pub padding1: [u8; 1],
    pub market: PerpMarketIndex,
    pub padding2: [u8; 4],
    pub client_id: u64,
    pub id: u128,
    pub reserved: [u8; 64],
}
const_assert_eq!(size_of::<PerpOpenOrder>(), 1 + 1 + 2 + 4 + 8 + 16 + 64);
const_assert_eq!(size_of::<PerpOpenOrder>(), 96);
const_assert_eq!(size_of::<PerpOpenOrder>() % 8, 0);

impl Default for PerpOpenOrder {
    fn default() -> Self {
        Self {
            side_and_tree: SideAndOrderTree::BidFixed.into(),
            padding1: Default::default(),
            market: FREE_ORDER_SLOT,
            padding2: Default::default(),
            client_id: 0,
            id: 0,
            reserved: [0; 64],
        }
    }
}

impl PerpOpenOrder {
    pub fn side_and_tree(&self) -> SideAndOrderTree {
        SideAndOrderTree::try_from(self.side_and_tree).unwrap()
    }

    pub fn is_active_for_market(&self, perp_market_index: PerpMarketIndex) -> bool {
        self.market == perp_market_index
    }
}

#[macro_export]
macro_rules! account_seeds {
    ( $account:expr ) => {
        &[
            b"MangoAccount".as_ref(),
            $account.group.as_ref(),
            $account.owner.as_ref(),
            &$account.account_num.to_le_bytes(),
            &[$account.bump],
        ]
    };
}

pub use account_seeds;

#[cfg(test)]
mod tests {
    use crate::state::PerpMarket;
    use fixed::types::I80F48;
    use rand::Rng;

    use super::PerpPosition;

    fn create_perp_position(
        market: &PerpMarket,
        base_pos: i64,
        entry_price_per_lot: i64,
    ) -> PerpPosition {
        let mut pos = PerpPosition::default();
        pos.market_index = market.perp_market_index;
        pos.base_position_lots = base_pos;
        pos.quote_position_native = I80F48::from(-base_pos * entry_price_per_lot);
        pos.quote_running_native = -base_pos * entry_price_per_lot;
        pos.avg_entry_price_per_base_lot = entry_price_per_lot as f64;
        pos
    }

    fn test_perp_market(stable_price: f64) -> PerpMarket {
        let mut m = PerpMarket::default_for_tests();
        m.stable_price_model.stable_price = stable_price;
        m
    }

    #[test]
    fn test_quote_entry_long_increasing_from_zero() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 0, 0);
        // Go long 10 @ 10
        pos.record_trade(&mut market, 10, I80F48::from(-100));
        assert_eq!(pos.quote_running_native, -100);
        assert_eq!(pos.avg_entry_price(&market), 10.0);
        assert_eq!(pos.break_even_price(&market), 10.0);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(0));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 0);
    }

    #[test]
    fn test_quote_entry_short_increasing_from_zero() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 0, 0);
        // Go short 10 @ 10
        pos.record_trade(&mut market, -10, I80F48::from(100));
        assert_eq!(pos.quote_running_native, 100);
        assert_eq!(pos.avg_entry_price(&market), 10.0);
        assert_eq!(pos.break_even_price(&market), 10.0);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(0));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 0);
    }

    #[test]
    fn test_quote_entry_long_increasing_from_long() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, 10);
        // Go long 10 @ 30
        pos.record_trade(&mut market, 10, I80F48::from(-300));
        assert_eq!(pos.quote_running_native, -400);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(0));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 0);
    }

    #[test]
    fn test_quote_entry_short_increasing_from_short() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 10);
        // Go short 10 @ 30
        pos.record_trade(&mut market, -10, I80F48::from(300));
        assert_eq!(pos.quote_running_native, 400);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(0));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 0);
    }

    #[test]
    fn test_quote_entry_long_decreasing_from_short() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 10);
        // Go long 5 @ 50
        pos.record_trade(&mut market, 5, I80F48::from(-250));
        assert_eq!(pos.quote_running_native, -150);
        assert_eq!(pos.avg_entry_price(&market), 10.0); // Entry price remains the same when decreasing
        assert_eq!(pos.break_even_price(&market), -30.0); // The short can't break even anymore
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(-200));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(-200));
        assert_eq!(pos.settle_pnl_limit_realized_trade, -5 * 10 / 5 - 1);
    }

    #[test]
    fn test_quote_entry_short_decreasing_from_long() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, 10);
        // Go short 5 @ 50
        pos.record_trade(&mut market, -5, I80F48::from(250));
        assert_eq!(pos.quote_running_native, 150);
        assert_eq!(pos.avg_entry_price(&market), 10.0); // Entry price remains the same when decreasing
        assert_eq!(pos.break_even_price(&market), -30.0); // Already broke even
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(200));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(200));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 5 * 10 / 5 + 1);
    }

    #[test]
    fn test_quote_entry_long_close_with_short() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, 10);
        // Go short 10 @ 25
        pos.record_trade(&mut market, -10, I80F48::from(250));
        assert_eq!(pos.quote_running_native, 0);
        assert_eq!(pos.avg_entry_price(&market), 0.0); // Entry price zero when no position
        assert_eq!(pos.break_even_price(&market), 0.0);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(150));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 10 * 10 / 5 + 1);
    }

    #[test]
    fn test_quote_entry_short_close_with_long() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 10);
        // Go long 10 @ 25
        pos.record_trade(&mut market, 10, I80F48::from(-250));
        assert_eq!(pos.quote_running_native, 0);
        assert_eq!(pos.avg_entry_price(&market), 0.0); // Entry price zero when no position
        assert_eq!(pos.break_even_price(&market), 0.0);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(-150));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, -10 * 10 / 5 - 1);
    }

    #[test]
    fn test_quote_entry_long_close_short_with_overflow() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, 10);
        // Go short 15 @ 20
        pos.record_trade(&mut market, -15, I80F48::from(300));
        assert_eq!(pos.quote_running_native, 100);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(100));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 10 * 10 / 5 + 1);
    }

    #[test]
    fn test_quote_entry_short_close_long_with_overflow() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 10);
        // Go long 15 @ 20
        pos.record_trade(&mut market, 15, I80F48::from(-300));
        assert_eq!(pos.quote_running_native, -100);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(-100));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, -10 * 10 / 5 - 1);
    }

    #[test]
    fn test_quote_entry_break_even_price() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 0, 0);
        // Buy 11 @ 10,000
        pos.record_trade(&mut market, 11, I80F48::from(-11 * 10_000));
        // Sell 1 @ 12,000
        pos.record_trade(&mut market, -1, I80F48::from(12_000));
        assert_eq!(pos.quote_running_native, -98_000);
        assert_eq!(pos.base_position_lots, 10);
        assert_eq!(pos.break_even_price(&market), 9_800.0); // We made 2k on the trade, so we can sell our contract up to a loss of 200 each
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(2_000));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(2_000));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 1 * 10 / 5 + 1);
    }

    #[test]
    fn test_entry_and_break_even_prices_with_lots() {
        let mut market = test_perp_market(10.0);
        market.base_lot_size = 10;

        let mut pos = create_perp_position(&market, 0, 0);
        // Buy 110 @ 10,000
        pos.record_trade(&mut market, 11, I80F48::from(-11 * 10 * 10_000));
        // Sell 10 @ 12,000
        pos.record_trade(&mut market, -1, I80F48::from(1 * 10 * 12_000));
        assert_eq!(pos.quote_running_native, -980_000);
        assert_eq!(pos.base_position_lots, 10);
        assert_eq!(pos.avg_entry_price_per_base_lot, 100_000.0);
        assert_eq!(pos.avg_entry_price(&market), 10_000.0);
        assert_eq!(pos.break_even_price(&market), 9_800.0);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(20_000));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(20_000));
    }

    #[test]
    fn test_perp_realized_settle_limit_no_reduction() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 0, 0);
        // Buy 11 @ 10,000
        pos.record_trade(&mut market, 11, I80F48::from(-11 * 10_000));

        // Sell 1 @ 11,000
        pos.record_trade(&mut market, -1, I80F48::from(11_000));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(1_000));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(1_000));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 1 * 10 / 5 + 1);

        // Sell 1 @ 11,000 -- increases limit
        pos.record_trade(&mut market, -1, I80F48::from(11_000));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(2_000));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(2_000));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 2 * (10 / 5 + 1));

        // Sell 1 @ 9,000 -- a loss, but doesn't flip realized_trade_pnl_native sign, no change to limit
        pos.record_trade(&mut market, -1, I80F48::from(9_000));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(1_000));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(1_000));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 2 * (10 / 5 + 1));

        // Sell 1 @ 8,000 -- flips sign, changes pnl limit
        pos.record_trade(&mut market, -1, I80F48::from(8_000));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(-1_000));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(-1_000));
        assert_eq!(pos.settle_pnl_limit_realized_trade, -(1 * 10 / 5 + 1));
    }

    #[test]
    fn test_perp_trade_without_realized_pnl() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 0, 0);

        // Buy 11 @ 10,000
        pos.record_trade(&mut market, 11, I80F48::from(-11 * 10_000));

        // Sell 1 @ 10,000
        pos.record_trade(&mut market, -1, I80F48::from(10_000));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(0));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 0);

        // Sell 10 @ 10,000
        pos.record_trade(&mut market, -10, I80F48::from(10 * 10_000));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(0));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 0);

        assert_eq!(pos.base_position_lots, 0);
        assert_eq!(pos.quote_position_native, I80F48::ZERO);
    }

    #[test]
    fn test_perp_realized_pnl_trade_other_separation() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 0, 0);

        pos.record_trading_fee(I80F48::from(-70));
        assert_eq!(pos.realized_other_pnl_native, I80F48::from(70));

        pos.record_liquidation_quote_change(I80F48::from(30));
        assert_eq!(pos.realized_other_pnl_native, I80F48::from(100));

        // Buy 1 @ 10,000
        pos.record_trade(&mut market, 1, I80F48::from(-1 * 10_000));

        // Sell 1 @ 11,000
        pos.record_trade(&mut market, -1, I80F48::from(11_000));

        assert_eq!(pos.realized_other_pnl_native, I80F48::from(100));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(1_000));
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 1 * 10 / 5 + 1);
    }

    #[test]
    fn test_realized_pnl_fractional() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 0, 0);
        pos.quote_position_native += I80F48::from_num(0.1);

        // Buy 1 @ 1
        pos.record_trade(&mut market, 1, I80F48::from(-1));
        // Buy 2 @ 2
        pos.record_trade(&mut market, 2, I80F48::from(-2 * 2));

        assert!((pos.avg_entry_price(&market) - 1.66666).abs() < 0.001);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(0));

        // Sell 2 @ 4
        pos.record_trade(&mut market, -2, I80F48::from(2 * 4));

        assert!((pos.avg_entry_price(&market) - 1.66666).abs() < 0.001);
        assert!((pos.realized_trade_pnl_native.to_num::<f64>() - 4.6666).abs() < 0.01);

        // Sell 1 @ 2
        pos.record_trade(&mut market, -1, I80F48::from(2));

        assert_eq!(pos.avg_entry_price(&market), 0.0);
        assert!((pos.quote_position_native.to_num::<f64>() - 5.1).abs() < 0.001);
        assert!((pos.realized_trade_pnl_native.to_num::<f64>() - 5.1).abs() < 0.01);
    }

    #[test]
    fn test_perp_entry_multiple_random_long() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 0, 0);

        // Generate array of random trades
        let mut rng = rand::thread_rng();
        let mut trades: Vec<[i64; 2]> = Vec::with_capacity(500);
        for _ in 0..trades.capacity() {
            let qty: i64 = rng.gen_range(1..=1000);
            let px: f64 = rng.gen_range(0.1..=100.0);
            let quote: i64 = (-qty as f64 * px).round() as i64;
            trades.push([qty, quote]);
        }
        // Apply all of the trades going forward
        let mut total_qty = 0;
        let mut total_quote = 0;
        trades.iter().for_each(|[qty, quote]| {
            pos.record_trade(&mut market, *qty, I80F48::from(*quote));
            total_qty += qty.abs();
            total_quote += quote.abs();
            let entry_actual = pos.avg_entry_price(&market);
            let entry_expected = total_quote as f64 / total_qty as f64;
            assert!(((entry_actual - entry_expected) / entry_expected).abs() < 10.0 * f64::EPSILON);
        });
        // base_position should be sum of all base quantities
        assert_eq!(pos.base_position_lots, total_qty);
        // Reverse out all the trades
        trades.iter().for_each(|[qty, quote]| {
            pos.record_trade(&mut market, -*qty, I80F48::from(-*quote));
        });
        assert_eq!(pos.base_position_lots, 0);
        assert_eq!(pos.quote_running_native, 0);
        assert_eq!(pos.avg_entry_price_per_base_lot, 0.0);
    }

    #[test]
    fn test_perp_position_pnl_returns_correct_pnl_for_oracle_price() {
        let mut market = test_perp_market(10.0);
        market.base_lot_size = 10;

        let long_pos = create_perp_position(&market, 50, 100);
        let pnl = long_pos.unsettled_pnl(&market, I80F48::from(11)).unwrap();
        assert_eq!(pnl, I80F48::from(50 * 10 * 1), "long profitable");
        let pnl = long_pos.unsettled_pnl(&market, I80F48::from(9)).unwrap();
        assert_eq!(pnl, I80F48::from(50 * 10 * -1), "long unprofitable");

        let short_pos = create_perp_position(&market, -50, 100);
        let pnl = short_pos.unsettled_pnl(&market, I80F48::from(11)).unwrap();
        assert_eq!(pnl, I80F48::from(50 * 10 * -1), "short unprofitable");
        let pnl = short_pos.unsettled_pnl(&market, I80F48::from(9)).unwrap();
        assert_eq!(pnl, I80F48::from(50 * 10 * 1), "short profitable");
    }

    #[test]
    fn test_perp_realized_pnl_consumption() {
        let market = test_perp_market(10.0);

        let mut pos = create_perp_position(&market, 0, 0);
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(0));

        pos.settle_pnl_limit_realized_trade = 1000;
        pos.realized_trade_pnl_native = I80F48::from(1500);
        pos.record_settle(I80F48::from(10));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(1490));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 1000);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 10);

        pos.record_settle(I80F48::from(-2));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(1490));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 1000);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 8);

        pos.record_settle(I80F48::from(1100));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(390));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 390);
        assert_eq!(
            pos.settle_pnl_limit_settled_in_current_window_native,
            8 + 1100 - (1000 - 390)
        );

        pos.settle_pnl_limit_realized_trade = 4;
        pos.settle_pnl_limit_settled_in_current_window_native = 0;
        pos.realized_trade_pnl_native = I80F48::from(5);
        assert_eq!(pos.available_settle_limit(&market), (0, 4));
        pos.record_settle(I80F48::from(-20));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(5));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 4);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, -20);
        assert_eq!(pos.available_settle_limit(&market), (0, 24));

        pos.record_settle(I80F48::from(2));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(3));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 3);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, -19);
        assert_eq!(pos.available_settle_limit(&market), (0, 22));

        pos.record_settle(I80F48::from(10));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 0);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, -12);
        assert_eq!(pos.available_settle_limit(&market), (0, 12));

        pos.realized_trade_pnl_native = I80F48::from(-5);
        pos.settle_pnl_limit_realized_trade = -4;
        pos.settle_pnl_limit_settled_in_current_window_native = 0;
        pos.record_settle(I80F48::from(20));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(-5));
        assert_eq!(pos.settle_pnl_limit_realized_trade, -4);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 20);

        pos.record_settle(I80F48::from(-2));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(-3));
        assert_eq!(pos.settle_pnl_limit_realized_trade, -3);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 19);

        pos.record_settle(I80F48::from(-10));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 0);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 12);

        pos.realized_other_pnl_native = I80F48::from(10);
        pos.realized_trade_pnl_native = I80F48::from(25);
        pos.settle_pnl_limit_realized_trade = 20;
        pos.record_settle(I80F48::from(1));
        assert_eq!(pos.realized_other_pnl_native, I80F48::from(9));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(25));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 20);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 12);

        pos.record_settle(I80F48::from(10));
        assert_eq!(pos.realized_other_pnl_native, I80F48::from(0));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(24));
        assert_eq!(pos.settle_pnl_limit_realized_trade, 20);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 13);

        pos.realized_other_pnl_native = I80F48::from(-10);
        pos.realized_trade_pnl_native = I80F48::from(-25);
        pos.settle_pnl_limit_realized_trade = -20;
        pos.record_settle(I80F48::from(-1));
        assert_eq!(pos.realized_other_pnl_native, I80F48::from(-9));
        assert_eq!(pos.realized_trade_pnl_native, I80F48::from(-25));
        assert_eq!(pos.settle_pnl_limit_realized_trade, -20);
    }

    #[test]
    fn test_perp_settle_limit_window() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, 100, -50);

        market.settle_pnl_limit_window_size_ts = 100;
        pos.settle_pnl_limit_settled_in_current_window_native = 10;

        pos.update_settle_limit(&market, 505);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 0);
        assert_eq!(pos.settle_pnl_limit_window, 5);

        pos.settle_pnl_limit_settled_in_current_window_native = 10;
        pos.update_settle_limit(&market, 550);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 10);
        assert_eq!(pos.settle_pnl_limit_window, 5);

        pos.settle_pnl_limit_settled_in_current_window_native = 10;
        pos.update_settle_limit(&market, 600);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 0);
        assert_eq!(pos.settle_pnl_limit_window, 6);

        market.settle_pnl_limit_window_size_ts = 400;
        pos.update_settle_limit(&market, 605);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 0);
        assert_eq!(pos.settle_pnl_limit_window, 1);
    }

    #[test]
    fn test_perp_settle_limit() {
        let mut market = test_perp_market(0.5);

        let mut pos = create_perp_position(&market, 100, 1);
        pos.realized_trade_pnl_native = I80F48::from(60); // no effect

        let limited_pnl = |pos: &PerpPosition, market: &PerpMarket, pnl: i64| {
            pos.apply_pnl_settle_limit(market, I80F48::from(pnl))
                .to_num::<f64>()
        };

        pos.settle_pnl_limit_realized_trade = 5;
        assert_eq!(pos.available_settle_limit(&market), (-10, 15)); // 0.2 factor * 0.5 stable price * 100 lots + 5 realized
        assert_eq!(limited_pnl(&pos, &market, 100), 15.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -10.0);

        pos.settle_pnl_limit_settled_in_current_window_native = 2;
        assert_eq!(pos.available_settle_limit(&market), (-12, 13));
        assert_eq!(limited_pnl(&pos, &market, 100), 13.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -12.0);

        pos.settle_pnl_limit_settled_in_current_window_native = 16;
        assert_eq!(pos.available_settle_limit(&market), (-26, 0));

        pos.settle_pnl_limit_settled_in_current_window_native = -16;
        assert_eq!(pos.available_settle_limit(&market), (0, 31));

        pos.settle_pnl_limit_realized_trade = 0;
        pos.settle_pnl_limit_settled_in_current_window_native = 2;
        assert_eq!(pos.available_settle_limit(&market), (-12, 8));

        pos.settle_pnl_limit_settled_in_current_window_native = -2;
        assert_eq!(pos.available_settle_limit(&market), (-8, 12));

        market.stable_price_model.stable_price = 1.0;
        assert_eq!(pos.available_settle_limit(&market), (-18, 22));

        pos.settle_pnl_limit_realized_trade = 1000;
        pos.settle_pnl_limit_settled_in_current_window_native = 2;
        assert_eq!(pos.available_settle_limit(&market), (-22, 1018));

        pos.realized_other_pnl_native = I80F48::from(5);
        assert_eq!(pos.available_settle_limit(&market), (-22, 1023));

        pos.realized_other_pnl_native = I80F48::from(-5);
        assert_eq!(pos.available_settle_limit(&market), (-27, 1018));
    }

    #[test]
    fn test_perp_reduced_realized_pnl_settle_limit() {
        let market = test_perp_market(0.5);
        let mut pos = create_perp_position(&market, 100, 1);

        let cases = vec![
            // No change if realized > limit
            (0, (100, 50, 70, -200), (50, 70)),
            // No change if realized > limit
            (1, (100, 50, 70, 200), (50, 70)),
            // No change if abs(realized) > abs(limit)
            (2, (-100, -50, 70, -200), (-50, 70)),
            // No change if abs(realized) > abs(limit)
            (3, (-100, -50, 70, 200), (-50, 70)),
            // reduction limited by realized change
            (4, (40, 50, 70, -5), (40, 65)),
            // reduction max
            (5, (40, 50, 70, -15), (40, 60)),
            // reduction, with realized change wrong direction
            (6, (40, 50, 70, 15), (40, 70)),
            // reduction limited by realized change
            (7, (-40, -50, -70, 5), (-40, -65)),
            // reduction max
            (8, (-40, -50, -70, 15), (-40, -60)),
            // reduction, with realized change wrong direction
            (9, (-40, -50, -70, -15), (-40, -70)),
            // reduction when used amount is opposite sign
            (10, (-40, -50, 70, -15), (-40, 70)),
            // reduction when used amount is opposite sign
            (11, (-40, -50, 70, 15), (-40, 80)),
        ];

        for (i, (realized, realized_limit, used, change), (expected_limit, expected_used)) in cases
        {
            println!("test case {i}");
            pos.realized_trade_pnl_native = I80F48::from(realized);
            pos.settle_pnl_limit_realized_trade = realized_limit;
            pos.settle_pnl_limit_settled_in_current_window_native = used;
            pos.apply_realized_trade_pnl_settle_limit_constraint(I80F48::from(change));
            assert_eq!(pos.settle_pnl_limit_realized_trade, expected_limit);
            assert_eq!(
                pos.settle_pnl_limit_settled_in_current_window_native,
                expected_used
            );
        }
    }
}
