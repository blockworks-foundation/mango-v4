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
#[derive(AnchorDeserialize, AnchorSerialize, Derivative, PartialEq)]
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
    pub in_use_count: u16,

    #[derivative(Debug = "ignore")]
    pub padding: [u8; 4],

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
    16 + 2 + 2 + 4 + 16 + 8 + 8 + 128
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

    pub fn increment_in_use(&mut self) {
        self.in_use_count += 1; // panic on overflow
    }

    pub fn decrement_in_use(&mut self) {
        self.in_use_count = self.in_use_count.saturating_sub(1);
    }
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize, Derivative, PartialEq)]
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

    /// Track something like the highest open bid / lowest open ask, in native/native units.
    ///
    /// Tracking it exactly isn't possible since we don't see fills. So instead track
    /// the min/max of the _placed_ bids and asks.
    ///
    /// The value is reset in serum3_place_order when a new order is placed without an
    /// existing one on the book.
    ///
    /// 0 is a special "unset" state.
    pub highest_placed_bid_inv: f64,
    pub lowest_placed_ask: f64,

    /// An overestimate of the amount of tokens that might flow out of the open orders account.
    ///
    /// The bank still considers these amounts user deposits (see Bank::potential_serum_tokens)
    /// and that value needs to be updated in conjunction with these numbers.
    ///
    /// This estimation is based on the amount of tokens in the open orders account
    /// (see update_bank_potential_tokens() in serum3_place_order and settle)
    pub potential_base_tokens: u64,
    pub potential_quote_tokens: u64,

    /// Track lowest bid/highest ask, same way as for highest bid/lowest ask.
    ///
    /// 0 is a special "unset" state.
    pub lowest_placed_bid_inv: f64,
    pub highest_placed_ask: f64,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 16],
}
const_assert_eq!(
    size_of::<Serum3Orders>(),
    32 + 8 * 2 + 2 * 3 + 2 + 6 * 8 + 16
);
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
            padding: Default::default(),
            base_borrows_without_fee: 0,
            quote_borrows_without_fee: 0,
            highest_placed_bid_inv: 0.0,
            lowest_placed_ask: 0.0,
            potential_base_tokens: 0,
            potential_quote_tokens: 0,
            lowest_placed_bid_inv: 0.0,
            highest_placed_ask: 0.0,
            reserved: [0; 16],
        }
    }
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize, Derivative, PartialEq)]
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

    /// Active position in oracle quote native. At the same time this is 1:1 a settle_token native amount.
    ///
    /// Example: Say there's a perp market on the BTC/USD price using SOL for settlement. The user buys
    /// one long contract for $20k, then base = 1, quote = -20k. The price goes to $21k. Now their
    /// unsettled pnl is (1 * 21k - 20k) __SOL__ = 1000 SOL. This is because the perp contract arbitrarily
    /// decides that each unit of price difference creates 1 SOL worth of settlement.
    /// (yes, causing 1 SOL of settlement for each $1 price change implies a lot of extra leverage; likely
    /// there should be an extra configurable scaling factor before we use this for cases like that)
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

    /// Deprecated field: Amount of pnl that was realized by bringing the base position closer to 0.
    pub deprecated_realized_trade_pnl_native: I80F48,

    /// Amount of pnl that can be settled once.
    ///
    /// - The value is signed: a negative number means negative pnl can be settled.
    /// - A settlement in the right direction will decrease this amount.
    ///
    /// Typically added for fees, funding and liquidation.
    pub oneshot_settle_pnl_allowance: I80F48,

    /// Amount of pnl that can be settled in each settle window.
    ///
    /// - Unsigned, the settlement can happen in both directions. Value is >= 0.
    /// - Previously stored a similar value that was signed, so in migration cases
    ///   this value can be negative and should be .abs()ed.
    /// - If this value exceeds the current stable-upnl, it should be decreased,
    ///   see apply_recurring_settle_pnl_allowance_constraint()
    ///
    /// When the base position is reduced, the settle limit contribution from the reduced
    /// base position is materialized into this value. When the base position increases,
    /// some of the allowance is taken away.
    ///
    /// This also gets increased when a liquidator takes over pnl.
    pub recurring_settle_pnl_allowance: i64,

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
            deprecated_realized_trade_pnl_native: I80F48::ZERO,
            oneshot_settle_pnl_allowance: I80F48::ZERO,
            settle_pnl_limit_window: 0,
            settle_pnl_limit_settled_in_current_window_native: 0,
            recurring_settle_pnl_allowance: 0,
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

    pub fn adjust_maker_lots(&mut self, side: Side, base_lots: i64) {
        match side {
            Side::Bid => {
                self.bids_base_lots += base_lots;
            }
            Side::Ask => {
                self.asks_base_lots += base_lots;
            }
        };
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
        self.oneshot_settle_pnl_allowance -= funding;
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
    ///
    /// Returns realized trade pnl
    fn update_trade_stats(
        &mut self,
        base_change: i64,
        quote_change_native: I80F48,
        perp_market: &PerpMarket,
    ) -> I80F48 {
        if base_change == 0 {
            return I80F48::ZERO;
        }

        let old_position = self.base_position_lots;
        let new_position = old_position + base_change;

        // abs amount of lots that were reduced:
        // - going from -5 to 10 lots is a reduction of 5
        // - going from 10 to -5 is a reduction of 10
        let reduced_lots;
        // same for increases
        // - going from -5 to 10 lots is an increase of 10
        // - going from 10 to -5 is an increase of 5
        let increased_lots;
        // amount of pnl that was realized by the reduction (signed)
        let newly_realized_pnl;

        if new_position == 0 {
            reduced_lots = old_position.abs();
            increased_lots = 0;

            let avg_entry = I80F48::from_num(self.avg_entry_price_per_base_lot);
            newly_realized_pnl = quote_change_native + I80F48::from(base_change) * avg_entry;

            // clear out display fields that live only while the position lasts
            self.avg_entry_price_per_base_lot = 0.0;
            self.quote_running_native = 0;
            self.realized_pnl_for_position_native = I80F48::ZERO;
        } else if old_position.signum() != new_position.signum() {
            // If the base position changes sign, we've crossed base_pos == 0 (or old_position == 0)
            reduced_lots = old_position.abs();
            increased_lots = new_position.abs();
            let old_position = old_position as f64;
            let new_position = new_position as f64;
            let base_change = base_change as f64;
            let old_avg_entry = self.avg_entry_price_per_base_lot;
            let new_avg_entry = (quote_change_native.to_num::<f64>() / base_change).abs();

            // Award realized pnl based on the old_position size
            newly_realized_pnl = I80F48::from_num(old_position * (new_avg_entry - old_avg_entry));

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
                increased_lots = base_change.abs();
                newly_realized_pnl = I80F48::ZERO;
                let old_position_abs = old_position.abs() as f64;
                let new_position_abs = new_position.abs() as f64;
                let old_avg_entry = self.avg_entry_price_per_base_lot;
                let new_position_quote_value =
                    old_position_abs * old_avg_entry + quote_change_native.to_num::<f64>().abs();
                self.avg_entry_price_per_base_lot = new_position_quote_value / new_position_abs;
            } else {
                // Decreasing position: pnl is realized, avg entry price does not change
                reduced_lots = base_change.abs();
                increased_lots = 0;
                let avg_entry = I80F48::from_num(self.avg_entry_price_per_base_lot);
                newly_realized_pnl = quote_change_native + I80F48::from(base_change) * avg_entry;
                self.realized_pnl_for_position_native += newly_realized_pnl;
            }
        }

        let net_base_increase = increased_lots - reduced_lots;
        self.recurring_settle_pnl_allowance = self.recurring_settle_pnl_allowance.abs();
        self.recurring_settle_pnl_allowance -=
            (I80F48::from(net_base_increase * perp_market.base_lot_size)
                * perp_market.stable_price()
                * I80F48::from_num(perp_market.settle_pnl_limit_factor))
            .clamp_to_i64();
        self.recurring_settle_pnl_allowance = self.recurring_settle_pnl_allowance.max(0);

        newly_realized_pnl
    }

    /// Returns the change in recurring settle allowance
    fn apply_recurring_settle_pnl_allowance_constraint(&mut self, perp_market: &PerpMarket) -> i64 {
        // deprecation/migration
        self.recurring_settle_pnl_allowance = self.recurring_settle_pnl_allowance.abs();
        self.deprecated_realized_trade_pnl_native = I80F48::ZERO;

        let before = self.recurring_settle_pnl_allowance;

        // The recurring allowance is always >= 0 and <= stable-upnl
        let upnl = self
            .unsettled_pnl(perp_market, perp_market.stable_price())
            .unwrap();
        let upnl_abs = upnl.abs().ceil().to_num::<i64>();
        self.recurring_settle_pnl_allowance =
            self.recurring_settle_pnl_allowance.min(upnl_abs).max(0);

        self.recurring_settle_pnl_allowance - before
    }

    /// Change the base and quote positions as the result of a trade
    ///
    /// Returns realized trade pnl
    pub fn record_trade(
        &mut self,
        perp_market: &mut PerpMarket,
        base_change: i64,
        quote_change_native: I80F48,
    ) -> I80F48 {
        assert_eq!(perp_market.perp_market_index, self.market_index);
        let realized_pnl = self.update_trade_stats(base_change, quote_change_native, perp_market);
        self.change_base_position(perp_market, base_change);
        self.change_quote_position(quote_change_native);
        self.apply_recurring_settle_pnl_allowance_constraint(perp_market);
        realized_pnl
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
    /// 1. a fraction of the base position stable value, which gives settlement limit
    ///    equally in both directions
    /// 2. the stored recurring settle allowance, which is mostly allowance from 1. that was
    ///    materialized when the position was reduced (see recurring_settle_pnl_allowance)
    /// 3. once-only settlement allowance in a single direction (see oneshot_settle_pnl_allowance)
    pub fn settle_limit(&self, market: &PerpMarket) -> (i64, i64) {
        assert_eq!(self.market_index, market.perp_market_index);
        if market.settle_pnl_limit_factor < 0.0 {
            return (i64::MIN, i64::MAX);
        }

        let base_native = self.base_position_native(market);
        let position_value = market.stable_price() * base_native;

        let position_value_abs = position_value.abs().to_num::<f64>();
        let unrealized =
            (market.settle_pnl_limit_factor as f64 * position_value_abs).clamp_to_i64();

        let upnl_abs = (self.quote_position_native() + position_value)
            .abs()
            .ceil()
            .to_num::<i64>();

        let mut max_pnl = unrealized
            // .abs() because of potential migration
            // .min() to do the same as apply_recurring_settle_pnl_allowance_constraint
            + self.recurring_settle_pnl_allowance.abs().min(upnl_abs);
        let mut min_pnl = -max_pnl;

        let oneshot = self.oneshot_settle_pnl_allowance;
        if oneshot >= 0 {
            max_pnl = max_pnl.saturating_add(oneshot.ceil().clamp_to_i64());
        } else {
            min_pnl = min_pnl.saturating_add(oneshot.floor().clamp_to_i64());
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
    pub fn record_settle(&mut self, settled_pnl: I80F48, perp_market: &PerpMarket) {
        self.change_quote_position(-settled_pnl);

        // Settlement reduces oneshot_settle_pnl_allowance if available.
        // Reduction only happens if settled_pnl has the same sign as oneshot_settle_pnl_allowance.
        let oneshot_reduction = if settled_pnl > 0 {
            settled_pnl
                .min(self.oneshot_settle_pnl_allowance)
                .max(I80F48::ZERO)
        } else {
            settled_pnl
                .max(self.oneshot_settle_pnl_allowance)
                .min(I80F48::ZERO)
        };
        self.oneshot_settle_pnl_allowance -= oneshot_reduction;

        // Consume settle limit budget:
        // We don't track consumption of oneshot_settle_pnl_allowance because settling already
        // reduces the available budget for subsequent settlesas well.
        let mut used_settle_limit = (settled_pnl - oneshot_reduction)
            .round_to_zero()
            .clamp_to_i64();

        // Similarly, if the recurring budget gets reduced (because stable-upnl is lower than it),
        // don't also increase settle_pnl_limit_settled_in_current_window_native.
        // Example: Settle 500 on a 1000 upnl, 1000 recurring limit account:
        //  -> 500 upnl and 500 recurring limit, if we also had 500 settled_in_current_window
        //     then no more settlement would be allowed
        let recurring_allowance_change =
            self.apply_recurring_settle_pnl_allowance_constraint(perp_market);
        if recurring_allowance_change < 0 {
            if used_settle_limit > 0 {
                used_settle_limit = (used_settle_limit + recurring_allowance_change).max(0);
            } else {
                used_settle_limit = (used_settle_limit - recurring_allowance_change).min(0);
            }
        }

        self.settle_pnl_limit_settled_in_current_window_native += used_settle_limit;
    }

    /// Update perp position for a maker/taker fee payment
    pub fn record_trading_fee(&mut self, fee: I80F48) {
        self.change_quote_position(-fee);
        self.oneshot_settle_pnl_allowance -= fee;
        self.realized_pnl_for_position_native -= fee;
    }

    /// Adds immediately-settleable realized pnl when a liqor takes over pnl during liquidation
    pub fn record_liquidation_quote_change(&mut self, change: I80F48) {
        self.change_quote_position(change);
        self.oneshot_settle_pnl_allowance += change;
    }

    /// Takes over a quote position along with recurring and oneshot settle limit allowance
    pub fn record_liquidation_pnl_takeover(
        &mut self,
        change: I80F48,
        recurring_limit: i64,
        oneshot_limit: i64,
    ) {
        self.change_quote_position(change);
        self.recurring_settle_pnl_allowance += recurring_limit;
        self.oneshot_settle_pnl_allowance += I80F48::from(oneshot_limit);
    }
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize, Derivative, PartialEq)]
#[derivative(Debug)]
pub struct PerpOpenOrder {
    pub side_and_tree: u8, // SideAndOrderTree -- enums aren't POD

    #[derivative(Debug = "ignore")]
    pub padding1: [u8; 1],

    pub market: PerpMarketIndex,

    #[derivative(Debug = "ignore")]
    pub padding2: [u8; 4],

    pub client_id: u64,
    pub id: u128,

    pub quantity: i64,

    // WARNING: When adding fields, take care of updating the clear() function
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 56],
}
const_assert_eq!(size_of::<PerpOpenOrder>(), 1 + 1 + 2 + 4 + 8 + 16 + 8 + 56);
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
            quantity: 0,
            reserved: [0; 56],
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

    pub fn is_active(&self) -> bool {
        self.market != FREE_ORDER_SLOT
    }

    pub fn clear(&mut self) {
        self.market = FREE_ORDER_SLOT;
        self.side_and_tree = SideAndOrderTree::BidFixed.into();
        self.id = 0;
        self.client_id = 0;
        self.quantity = 0;
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
        let realized = pos.record_trade(&mut market, 10, I80F48::from(-100));
        assert_eq!(pos.quote_running_native, -100);
        assert_eq!(pos.avg_entry_price(&market), 10.0);
        assert_eq!(pos.break_even_price(&market), 10.0);
        assert_eq!(pos.realized_pnl_for_position_native, realized);
        assert_eq!(realized, I80F48::ZERO);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 0);
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::from(0));
    }

    #[test]
    fn test_quote_entry_short_increasing_from_zero() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 0, 0);
        // Go short 10 @ 10
        let realized = pos.record_trade(&mut market, -10, I80F48::from(100));
        assert_eq!(pos.quote_running_native, 100);
        assert_eq!(pos.avg_entry_price(&market), 10.0);
        assert_eq!(pos.break_even_price(&market), 10.0);
        assert_eq!(pos.realized_pnl_for_position_native, realized);
        assert_eq!(realized, I80F48::ZERO);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 0);
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::from(0));
    }

    #[test]
    fn test_quote_entry_long_increasing_from_long() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, 10);
        // Go long 10 @ 30
        let realized = pos.record_trade(&mut market, 10, I80F48::from(-300));
        assert_eq!(pos.quote_running_native, -400);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_for_position_native, realized);
        assert_eq!(realized, I80F48::ZERO);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 0);
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::from(0));
    }

    #[test]
    fn test_quote_entry_short_increasing_from_short() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 10);
        // Go short 10 @ 30
        let realized = pos.record_trade(&mut market, -10, I80F48::from(300));
        assert_eq!(pos.quote_running_native, 400);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_for_position_native, realized);
        assert_eq!(realized, I80F48::ZERO);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 0);
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::from(0));
    }

    #[test]
    fn test_quote_entry_long_decreasing_from_short() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 10);
        // Go long 5 @ 50
        let realized = pos.record_trade(&mut market, 5, I80F48::from(-250));
        assert_eq!(pos.quote_running_native, -150);
        assert_eq!(pos.avg_entry_price(&market), 10.0); // Entry price remains the same when decreasing
        assert_eq!(pos.break_even_price(&market), -30.0); // The short can't break even anymore
        assert_eq!(pos.realized_pnl_for_position_native, realized);
        assert_eq!(realized, I80F48::from(-200));
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 11); // 5 * 10 * 0.2 rounded up
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::ZERO);
    }

    #[test]
    fn test_quote_entry_short_decreasing_from_long() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, 10);
        // Go short 5 @ 50
        let realized = pos.record_trade(&mut market, -5, I80F48::from(250));
        assert_eq!(pos.quote_running_native, 150);
        assert_eq!(pos.avg_entry_price(&market), 10.0); // Entry price remains the same when decreasing
        assert_eq!(pos.break_even_price(&market), -30.0); // Already broke even
        assert_eq!(pos.realized_pnl_for_position_native, realized);
        assert_eq!(realized, I80F48::from(200));
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 11); // 5 * 10 * 0.2 rounded up
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::ZERO);
    }

    #[test]
    fn test_quote_entry_long_close_with_short() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, 10);
        // Go short 10 @ 25
        let realized = pos.record_trade(&mut market, -10, I80F48::from(250));
        assert_eq!(pos.quote_running_native, 0);
        assert_eq!(pos.avg_entry_price(&market), 0.0); // Entry price zero when no position
        assert_eq!(pos.break_even_price(&market), 0.0);
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(realized, I80F48::from(150));
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 21); // 10 * 10 * 0.2 rounded up
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::ZERO);
    }

    #[test]
    fn test_quote_entry_short_close_with_long() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 10);
        // Go long 10 @ 25
        let realized = pos.record_trade(&mut market, 10, I80F48::from(-250));
        assert_eq!(pos.quote_running_native, 0);
        assert_eq!(pos.avg_entry_price(&market), 0.0); // Entry price zero when no position
        assert_eq!(pos.break_even_price(&market), 0.0);
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(realized, I80F48::from(-150));
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 21); // 10 * 10 * 0.2 rounded up
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::ZERO);
    }

    #[test]
    fn test_quote_entry_long_close_short_with_overflow() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, 10);
        // Go short 15 @ 20
        let realized = pos.record_trade(&mut market, -15, I80F48::from(300));
        assert_eq!(pos.quote_running_native, 100);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::ZERO); // new position
        assert_eq!(realized, I80F48::from(100));
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 11); // 5 * 10 * 0.2 rounded up
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::ZERO);
    }

    #[test]
    fn test_quote_entry_short_close_long_with_overflow() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 10);
        // Go long 15 @ 20
        let realized = pos.record_trade(&mut market, 15, I80F48::from(-300));
        assert_eq!(pos.quote_running_native, -100);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::ZERO); // new position
        assert_eq!(realized, I80F48::from(-100));
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 11); // 5 * 10 * 0.2 rounded up
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::ZERO);
    }

    #[test]
    fn test_quote_entry_break_even_price() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 0, 0);
        // Buy 11 @ 10,000
        let realized_buy = pos.record_trade(&mut market, 11, I80F48::from(-11 * 10_000));
        // Sell 1 @ 12,000
        let realized_sell = pos.record_trade(&mut market, -1, I80F48::from(12_000));
        assert_eq!(pos.quote_running_native, -98_000);
        assert_eq!(pos.base_position_lots, 10);
        assert_eq!(pos.break_even_price(&market), 9_800.0); // We made 2k on the trade, so we can sell our contract up to a loss of 200 each
        assert_eq!(
            pos.realized_pnl_for_position_native,
            realized_buy + realized_sell
        );
        assert_eq!(realized_buy, I80F48::ZERO);
        assert_eq!(realized_sell, I80F48::from(2_000));
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 3); // 1 * 10 * 0.2 rounded up
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::ZERO);
    }

    #[test]
    fn test_entry_and_break_even_prices_with_lots() {
        let mut market = test_perp_market(10.0);
        market.base_lot_size = 10;

        let mut pos = create_perp_position(&market, 0, 0);
        // Buy 110 @ 10,000
        let realized_buy = pos.record_trade(&mut market, 11, I80F48::from(-11 * 10 * 10_000));
        // Sell 10 @ 12,000
        let realized_sell = pos.record_trade(&mut market, -1, I80F48::from(1 * 10 * 12_000));
        assert_eq!(pos.quote_running_native, -980_000);
        assert_eq!(pos.base_position_lots, 10);
        assert_eq!(pos.avg_entry_price_per_base_lot, 100_000.0);
        assert_eq!(pos.avg_entry_price(&market), 10_000.0);
        assert_eq!(pos.break_even_price(&market), 9_800.0);

        assert_eq!(
            pos.realized_pnl_for_position_native,
            realized_buy + realized_sell
        );
        assert_eq!(realized_buy, I80F48::ZERO);
        assert_eq!(realized_sell, I80F48::from(20_000));
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::ZERO);
        assert_eq!(pos.recurring_settle_pnl_allowance, 21); // 10 * 10 * 0.2 rounded up
        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::ZERO);
    }

    #[test]
    fn test_perp_realized_settle_limit_no_reduction() {
        let mut market = test_perp_market(10000.0);
        let mut pos = create_perp_position(&market, 0, 0);
        // Buy 11 @ 10,000
        pos.record_trade(&mut market, 11, I80F48::from(-11 * 10_000));

        // Sell 1 @ 11,000
        pos.record_trade(&mut market, -1, I80F48::from(11_000));
        assert_eq!(pos.recurring_settle_pnl_allowance, 1000); // 1 * 10000 * 0.2 rounded up, limited by upnl!

        // Sell 1 @ 9,500 -- actually decreases because upnl goes down
        pos.record_trade(&mut market, -1, I80F48::from(9_500));
        assert_eq!(pos.recurring_settle_pnl_allowance, 500);

        // Sell 2 @ 20,000 each -- not limited this time
        pos.record_trade(&mut market, -2, I80F48::from(40_000));
        assert_eq!(pos.recurring_settle_pnl_allowance, 4501);

        // Buy 1 @ 9,000 -- decreases allowance
        pos.record_trade(&mut market, 1, I80F48::from(-9_000));
        assert_eq!(pos.recurring_settle_pnl_allowance, 2501);

        // Sell 1 @ 8,000 -- increases limit
        market.stable_price_model.stable_price = 8000.0;
        pos.record_trade(&mut market, -1, I80F48::from(8_000));
        assert_eq!(pos.recurring_settle_pnl_allowance, 4102);

        assert_eq!(pos.deprecated_realized_trade_pnl_native, I80F48::ZERO);
    }

    #[test]
    fn test_perp_trade_without_realized_pnl() {
        let mut market = test_perp_market(10_000.0);
        let mut pos = create_perp_position(&market, 0, 0);

        // Buy 11 @ 10,000
        pos.record_trade(&mut market, 11, I80F48::from(-11 * 10_000));

        // Sell 1 @ 10,000
        let realized = pos.record_trade(&mut market, -1, I80F48::from(10_000));
        assert_eq!(realized, I80F48::ZERO);
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.recurring_settle_pnl_allowance, 0);

        // Sell 10 @ 10,000
        let realized = pos.record_trade(&mut market, -10, I80F48::from(10 * 10_000));
        assert_eq!(realized, I80F48::ZERO);
        assert_eq!(pos.realized_pnl_for_position_native, I80F48::from(0));
        assert_eq!(pos.recurring_settle_pnl_allowance, 0);

        assert_eq!(pos.base_position_lots, 0);
        assert_eq!(pos.quote_position_native, I80F48::ZERO);
    }

    #[test]
    fn test_perp_oneshot_settle_allowance() {
        let mut market = test_perp_market(10_000.0);
        let mut pos = create_perp_position(&market, 0, 0);

        pos.record_trading_fee(I80F48::from(-70));
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(70));

        pos.record_liquidation_quote_change(I80F48::from(30));
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(100));

        // Buy 1 @ 10,000
        pos.record_trade(&mut market, 1, I80F48::from(-1 * 10_000));

        // Sell 1 @ 11,000
        pos.record_trade(&mut market, -1, I80F48::from(11_000));

        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(100));
        assert_eq!(pos.recurring_settle_pnl_allowance, 1100); // limited by upnl

        pos.record_settle(I80F48::from(50), &market);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(50));
        assert_eq!(pos.recurring_settle_pnl_allowance, 1050);

        pos.record_settle(I80F48::from(100), &market);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(0));
        assert_eq!(pos.recurring_settle_pnl_allowance, 950);
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

        // Sell 2 @ 4
        let realized1 = pos.record_trade(&mut market, -2, I80F48::from(2 * 4));

        assert!((pos.avg_entry_price(&market) - 1.66666).abs() < 0.001);
        assert!((realized1.to_num::<f64>() - 4.6666).abs() < 0.01);

        // Sell 1 @ 2
        let realized2 = pos.record_trade(&mut market, -1, I80F48::from(2));

        assert_eq!(pos.avg_entry_price(&market), 0.0);
        assert!((pos.quote_position_native.to_num::<f64>() - 5.1).abs() < 0.001);
        assert!((realized2.to_num::<f64>() - 0.3333).abs() < 0.01);
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
    fn test_perp_settle_limit_allowance_consumption() {
        let market = test_perp_market(10.0);

        let mut pos = create_perp_position(&market, 0, 0);

        // setup some upnl so the recurring allowance isn't reduced immediately
        pos.quote_position_native = I80F48::from(1100);

        pos.recurring_settle_pnl_allowance = 1000;
        pos.record_settle(I80F48::from(10), &market);
        assert_eq!(pos.recurring_settle_pnl_allowance, 1000);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 10);

        pos.record_settle(I80F48::from(-2), &market);
        assert_eq!(pos.recurring_settle_pnl_allowance, 1000);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 8);

        pos.record_settle(I80F48::from(492), &market);
        assert_eq!(pos.recurring_settle_pnl_allowance, 600);
        assert_eq!(
            pos.settle_pnl_limit_settled_in_current_window_native,
            8 + 492 - 400
        );

        pos.settle_pnl_limit_settled_in_current_window_native = 0;
        pos.recurring_settle_pnl_allowance = 0;
        pos.oneshot_settle_pnl_allowance = I80F48::from(4);
        assert_eq!(pos.available_settle_limit(&market), (0, 4));
        pos.record_settle(I80F48::from(-20), &market);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, -20);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(4));
        assert_eq!(pos.available_settle_limit(&market), (0, 24));

        pos.record_settle(I80F48::from(2), &market);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, -20);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(2));
        assert_eq!(pos.available_settle_limit(&market), (0, 22));

        pos.record_settle(I80F48::from(4), &market);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, -18);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(0));
        assert_eq!(pos.available_settle_limit(&market), (0, 18));

        pos.settle_pnl_limit_settled_in_current_window_native = 0;
        pos.recurring_settle_pnl_allowance = 0;
        pos.oneshot_settle_pnl_allowance = I80F48::from(-4);
        assert_eq!(pos.available_settle_limit(&market), (-4, 0));
        pos.record_settle(I80F48::from(20), &market);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 20);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(-4));
        assert_eq!(pos.available_settle_limit(&market), (-24, 0));

        pos.record_settle(I80F48::from(-2), &market);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 20);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(-2));
        assert_eq!(pos.available_settle_limit(&market), (-22, 0));

        pos.record_settle(I80F48::from(-4), &market);
        assert_eq!(pos.settle_pnl_limit_settled_in_current_window_native, 18);
        assert_eq!(pos.oneshot_settle_pnl_allowance, I80F48::from(0));
        assert_eq!(pos.available_settle_limit(&market), (-18, 0));
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

        let limited_pnl = |pos: &PerpPosition, market: &PerpMarket, pnl: i64| {
            pos.apply_pnl_settle_limit(market, I80F48::from(pnl))
                .to_num::<f64>()
        };

        assert_eq!(pos.available_settle_limit(&market), (-10, 10)); // 0.2 factor * 0.5 stable price * 100 lots
        assert_eq!(limited_pnl(&pos, &market, 100), 10.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -10.0);

        pos.oneshot_settle_pnl_allowance = I80F48::from_num(-5);
        assert_eq!(pos.available_settle_limit(&market), (-15, 10));
        assert_eq!(limited_pnl(&pos, &market, 100), 10.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -15.0);

        pos.oneshot_settle_pnl_allowance = I80F48::from_num(5);
        assert_eq!(pos.available_settle_limit(&market), (-10, 15));
        assert_eq!(limited_pnl(&pos, &market, 100), 15.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -10.0);

        pos.recurring_settle_pnl_allowance = 11;
        assert_eq!(pos.available_settle_limit(&market), (-21, 26));
        assert_eq!(limited_pnl(&pos, &market, 100), 26.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -21.0);

        pos.settle_pnl_limit_settled_in_current_window_native = 17;
        assert_eq!(pos.available_settle_limit(&market), (-38, 9));

        pos.settle_pnl_limit_settled_in_current_window_native = 27;
        assert_eq!(pos.available_settle_limit(&market), (-48, 0));

        pos.settle_pnl_limit_settled_in_current_window_native = -17;
        assert_eq!(pos.available_settle_limit(&market), (-4, 43));

        pos.settle_pnl_limit_settled_in_current_window_native = -27;
        assert_eq!(pos.available_settle_limit(&market), (0, 53));

        pos.settle_pnl_limit_settled_in_current_window_native = 0;
        market.stable_price_model.stable_price = 1.0;
        // because the upnl is 0 the recurring allowance doesn't count
        assert_eq!(
            pos.unsettled_pnl(&market, I80F48::from_num(1.0)).unwrap(),
            I80F48::ZERO
        );
        assert_eq!(pos.available_settle_limit(&market), (-20, 25));

        pos.quote_position_native += I80F48::from(7);
        assert_eq!(pos.available_settle_limit(&market), (-27, 32));
    }
}
