use anchor_lang::prelude::*;
use checked_math as cm;
use derivative::Derivative;
use fixed::types::I80F48;
use static_assertions::const_assert_eq;
use std::cmp::Ordering;
use std::mem::size_of;

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
    pub settle_pnl_limit_window: u32,
    pub settle_pnl_limit_settled_in_current_window_native: i64,

    /// Active position size, measured in base lots
    base_position_lots: i64,
    /// Active position in quote (conversation rate is that of the time the order was settled)
    /// measured in native quote
    quote_position_native: I80F48,

    /// Tracks what the position is to calculate average entry & break even price
    pub quote_running_native: i64,

    /// Already settled funding
    pub long_settled_funding: I80F48,
    pub short_settled_funding: I80F48,

    /// Base lots in bids
    pub bids_base_lots: i64,
    /// Base lots in asks
    pub asks_base_lots: i64,

    /// Amount that's on EventQueue waiting to be processed
    pub taker_base_lots: i64,
    pub taker_quote_lots: i64,

    // (Display only)
    // Cumulative long funding in base native units
    pub cumulative_long_funding: f64,
    // (Display only)
    // Cumulative short funding in base native units
    pub cumulative_short_funding: f64,
    // (Display only)
    // Cumulative maker volume in quote native units
    pub maker_volume: u64,
    // (Display only)
    // Cumulative taker volume in quote native units
    pub taker_volume: u64,
    // (Display only)
    // Cumulative realized pnl in quote native units
    pub perp_spot_transfers: i64,

    pub avg_entry_price_per_base_lot: f64,

    pub realized_pnl_native: I80F48,

    pub settle_pnl_limit_realized_pnl_native: u64,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 120],
}
const_assert_eq!(
    size_of::<PerpPosition>(),
    2 + 2 + 4 + 8 + 8 + 16 + 8 + 16 * 2 + 8 * 2 + 8 * 2 + 8 * 5 + 8 + 16 + 8 + 120
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
            realized_pnl_native: I80F48::ZERO,
            settle_pnl_limit_window: 0,
            settle_pnl_limit_settled_in_current_window_native: 0,
            settle_pnl_limit_realized_pnl_native: 0,
            reserved: [0; 120],
        }
    }
}

impl PerpPosition {
    /// Add taker trade after it has been matched but before it has been process on EventQueue
    pub fn add_taker_trade(&mut self, side: Side, base_lots: i64, quote_lots: i64) {
        match side {
            Side::Bid => {
                cm!(self.taker_base_lots += base_lots);
                cm!(self.taker_quote_lots -= quote_lots);
            }
            Side::Ask => {
                cm!(self.taker_base_lots -= base_lots);
                cm!(self.taker_quote_lots += quote_lots);
            }
        }
    }
    /// Remove taker trade after it has been processed on EventQueue
    pub fn remove_taker_trade(&mut self, base_change: i64, quote_change: i64) {
        cm!(self.taker_base_lots -= base_change);
        cm!(self.taker_quote_lots -= quote_change);
    }

    pub fn is_active(&self) -> bool {
        self.market_index != PerpMarketIndex::MAX
    }

    pub fn is_active_for_market(&self, market_index: PerpMarketIndex) -> bool {
        self.market_index == market_index
    }

    // Return base position in native units for a perp market
    pub fn base_position_native(&self, market: &PerpMarket) -> I80F48 {
        I80F48::from(cm!(self.base_position_lots * market.base_lot_size))
    }

    pub fn base_position_lots(&self) -> i64 {
        self.base_position_lots
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
                cm!((perp_market.long_funding - self.long_settled_funding)
                    * I80F48::from_num(self.base_position_lots))
            }
            Ordering::Less => {
                cm!((perp_market.short_funding - self.short_settled_funding)
                    * I80F48::from_num(self.base_position_lots))
            }
            Ordering::Equal => I80F48::ZERO,
        }
    }

    /// Move unrealized funding payments into the quote_position
    pub fn settle_funding(&mut self, perp_market: &PerpMarket) {
        let funding = self.unsettled_funding(perp_market);
        cm!(self.quote_position_native -= funding);
        cm!(self.realized_pnl_native -= funding);

        if self.base_position_lots.is_positive() {
            self.cumulative_long_funding += funding.to_num::<f64>();
        } else {
            self.cumulative_short_funding -= funding.to_num::<f64>();
        }

        self.long_settled_funding = perp_market.long_funding;
        self.short_settled_funding = perp_market.short_funding;
    }

    /// Updates entry price, breakeven price, realized pnl
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
        let new_position = cm!(old_position + base_change);
        let reduced_lots;

        if new_position == 0 {
            reduced_lots = old_position;

            // clear out entry and break-even prices
            self.avg_entry_price_per_base_lot = 0.0;
            self.quote_running_native = 0;

            // There can't be unrealized pnl without a base position, so fix the
            // realized pnl to cover the whole quote position.
            self.realized_pnl_native = cm!(self.quote_position_native + quote_change_native);
        } else if old_position.signum() != new_position.signum() {
            // If the base position changes sign, we've crossed base_pos == 0 (or old_position == 0)
            reduced_lots = old_position;
            let old_position = old_position as f64;
            let new_position = new_position as f64;
            let base_change = base_change as f64;
            let old_avg_entry = self.avg_entry_price_per_base_lot;
            let new_avg_entry = (quote_change_native.to_num::<f64>() / base_change).abs();

            // Award realized pnl based on the old_position size
            let new_realized_pnl = I80F48::from_num(old_position * (new_avg_entry - old_avg_entry));
            cm!(self.realized_pnl_native += new_realized_pnl);

            // Set entry and break-even based on the new_position entered
            self.avg_entry_price_per_base_lot = new_avg_entry;
            self.quote_running_native = (-new_position * new_avg_entry) as i64;
        } else {
            // The old and new position have the same sign

            cm!(self.quote_running_native += quote_change_native
                .round_to_zero()
                .checked_to_num::<i64>()
                .unwrap());

            let is_increasing = old_position.signum() == base_change.signum();
            if is_increasing {
                // Increasing position: avg entry price updates, no new realized pnl
                reduced_lots = 0;
                let old_position_abs = old_position.abs() as f64;
                let new_position_abs = new_position.abs() as f64;
                let old_avg_entry = self.avg_entry_price_per_base_lot;
                let new_position_quote_value =
                    old_position_abs * old_avg_entry + quote_change_native.to_num::<f64>().abs();
                self.avg_entry_price_per_base_lot = new_position_quote_value / new_position_abs;
            } else {
                // Decreasing position: pnl is realized, avg entry price does not change
                reduced_lots = base_change.abs();
                let avg_entry = I80F48::from_num(self.avg_entry_price_per_base_lot);
                let new_realized_pnl =
                    cm!(quote_change_native + I80F48::from(base_change) * avg_entry);
                cm!(self.realized_pnl_native += new_realized_pnl);
            }
        }

        let realized_safe_value = cm!(
            I80F48::from(reduced_lots * perp_market.base_lot_size) * perp_market.stable_price()
        )
        .abs();
        // TODO: Bad! Must also min(actual realized pnl), otherwise null-trades bump up this limit!
        let limit_increase =
            cm!(I80F48::from_num(perp_market.settle_pnl_limit_factor) * realized_safe_value)
                .ceil()
                .to_num::<u64>();
        cm!(self.settle_pnl_limit_realized_pnl_native += limit_increase);
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
        cm!(self.quote_position_native += quote_change_native);
    }

    /// Does the perp position have any open orders or fill events?
    pub fn has_open_orders(&self) -> bool {
        self.asks_base_lots != 0
            || self.bids_base_lots != 0
            || self.taker_base_lots != 0
            || self.taker_quote_lots != 0
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
            / (cm!(self.base_position_lots * market.base_lot_size) as f64)
    }

    /// Calculate the PnL of the position for a given price
    pub fn pnl_for_price(&self, perp_market: &PerpMarket, price: I80F48) -> Result<I80F48> {
        require_eq!(self.market_index, perp_market.perp_market_index);
        let base_native = self.base_position_native(&perp_market);
        let pnl: I80F48 = cm!(self.quote_position_native() + base_native * price);
        Ok(pnl)
    }

    /// Updates the perp pnl limit time windowing, resetting the amount
    /// of used settle-pnl budget if necessary
    pub fn update_settle_limit(&mut self, market: &PerpMarket, now_ts: u64) {
        assert_eq!(self.market_index, market.perp_market_index);
        let window_size = market.settle_pnl_limit_window_size_ts;
        let new_window = now_ts >= cm!((self.settle_pnl_limit_window + 1) as u64 * window_size);
        if new_window {
            self.settle_pnl_limit_window = cm!(now_ts / window_size).try_into().unwrap();
            self.settle_pnl_limit_settled_in_current_window_native = 0;
        }
    }

    /// Returns the quote-native amount of pnl that may still be settled this settle window.
    /// Always >= 0.
    pub fn available_settle_limit(&self, market: &PerpMarket) -> i64 {
        assert_eq!(self.market_index, market.perp_market_index);
        if market.settle_pnl_limit_factor < 0.0 {
            return i64::MAX;
        }

        let base_native = self.base_position_native(market);
        let position_value = cm!(market.stable_price() * base_native)
            .abs()
            .to_num::<f64>();
        let window_unrealized =
            (market.settle_pnl_limit_factor as f64 * position_value).min(i64::MAX as f64) as i64;
        let window_total = window_unrealized.saturating_add(
            self.settle_pnl_limit_realized_pnl_native
                .min(i64::MAX as u64) as i64,
        );

        (window_total - self.settle_pnl_limit_settled_in_current_window_native).max(0)
    }

    /// Given some pnl, applies the pnl settle limit and returns the reduced pnl.
    pub fn apply_pnl_settle_limit(&self, pnl: I80F48, market: &PerpMarket) -> I80F48 {
        if market.settle_pnl_limit_factor < 0.0 {
            return pnl;
        }

        let available_settle_limit = I80F48::from(self.available_settle_limit(&market));
        if pnl < 0 {
            pnl.max(-available_settle_limit)
        } else {
            pnl.min(available_settle_limit)
        }
    }

    /// Update the perp position for pnl settlement
    ///
    /// If `pnl` is positive, then that is settled away, deducting from the quote position.
    pub fn record_settle(&mut self, full_pnl: I80F48, settled_pnl: I80F48) {
        self.change_quote_position(-settled_pnl);

        let settled_pnl_i64 = settled_pnl.round_to_zero().checked_to_num::<i64>().unwrap();
        cm!(self.settle_pnl_limit_settled_in_current_window_native += settled_pnl_i64);

        let used_realized = if settled_pnl > 0 {
            // Example: settling 100 positive pnl, with 60 realized:
            // pnl = 100 -> used_realized = 60
            settled_pnl.min(self.realized_pnl_native).max(I80F48::ZERO)
        } else {
            // Example: settling 100 negative pnl, with -60 realized:
            // pnl = -100 -> used_realized = -60
            settled_pnl.max(self.realized_pnl_native).min(I80F48::ZERO)
        };
        cm!(self.realized_pnl_native -= used_realized);

        let remaining_pnl = cm!(full_pnl - settled_pnl);
        let max_realized_remaining = remaining_pnl.abs().ceil().to_num::<u64>();
        self.settle_pnl_limit_realized_pnl_native = self
            .settle_pnl_limit_realized_pnl_native
            .min(max_realized_remaining);
    }

    pub fn record_fee(&mut self, fee: I80F48) {
        self.change_quote_position(-fee);
        cm!(self.realized_pnl_native -= fee);
    }

    pub fn record_bankruptcy_quote_change(&mut self, change: I80F48) {
        self.change_quote_position(change);
        cm!(self.realized_pnl_native += change);
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

    fn create_perp_position(market: &PerpMarket, base_pos: i64, quote_pos: i64) -> PerpPosition {
        let mut pos = PerpPosition::default();
        pos.market_index = market.perp_market_index;
        pos.base_position_lots = base_pos;
        pos.quote_position_native = I80F48::from(quote_pos);
        pos.quote_running_native = quote_pos;
        pos.avg_entry_price_per_base_lot = if base_pos != 0 {
            ((quote_pos as f64) / (base_pos as f64)).abs()
        } else {
            0.0
        };
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
        assert_eq!(pos.realized_pnl_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 0);
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
        assert_eq!(pos.realized_pnl_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 0);
    }

    #[test]
    fn test_quote_entry_long_increasing_from_long() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, -100);
        // Go long 10 @ 30
        pos.record_trade(&mut market, 10, I80F48::from(-300));
        assert_eq!(pos.quote_running_native, -400);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 0);
    }

    #[test]
    fn test_quote_entry_short_increasing_from_short() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 100);
        // Go short 10 @ 10
        pos.record_trade(&mut market, -10, I80F48::from(300));
        assert_eq!(pos.quote_running_native, 400);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 0);
    }

    #[test]
    fn test_quote_entry_long_decreasing_from_short() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 100);
        // Go long 5 @ 50
        pos.record_trade(&mut market, 5, I80F48::from(-250));
        assert_eq!(pos.quote_running_native, -150);
        assert_eq!(pos.avg_entry_price(&market), 10.0); // Entry price remains the same when decreasing
        assert_eq!(pos.break_even_price(&market), -30.0); // Already broke even
        assert_eq!(pos.realized_pnl_native, I80F48::from(-200));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 5 * 10 / 5 + 1);
    }

    #[test]
    fn test_quote_entry_short_decreasing_from_long() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, -100);
        // Go short 5 @ 50
        pos.record_trade(&mut market, -5, I80F48::from(250));
        assert_eq!(pos.quote_running_native, 150);
        assert_eq!(pos.avg_entry_price(&market), 10.0); // Entry price remains the same when decreasing
        assert_eq!(pos.break_even_price(&market), -30.0); // Already broke even
        assert_eq!(pos.realized_pnl_native, I80F48::from(200));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 5 * 10 / 5 + 1);
    }

    #[test]
    fn test_quote_entry_long_close_with_short() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, -100);
        // Go short 10 @ 25
        pos.record_trade(&mut market, -10, I80F48::from(250));
        assert_eq!(pos.quote_running_native, 0);
        assert_eq!(pos.avg_entry_price(&market), 0.0); // Entry price zero when no position
        assert_eq!(pos.break_even_price(&market), 0.0);
        assert_eq!(pos.realized_pnl_native, I80F48::from(150));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 10 * 10 / 5 + 1);
    }

    #[test]
    fn test_quote_entry_short_close_with_long() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 100);
        // Go long 10 @ 25
        pos.record_trade(&mut market, 10, I80F48::from(-250));
        assert_eq!(pos.quote_running_native, 0);
        assert_eq!(pos.avg_entry_price(&market), 0.0); // Entry price zero when no position
        assert_eq!(pos.break_even_price(&market), 0.0);
        assert_eq!(pos.realized_pnl_native, I80F48::from(-150));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 10 * 10 / 5 + 1);
    }

    #[test]
    fn test_quote_entry_long_close_short_with_overflow() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, 10, -100);
        // Go short 15 @ 20
        pos.record_trade(&mut market, -15, I80F48::from(300));
        assert_eq!(pos.quote_running_native, 100);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_native, I80F48::from(100));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 10 * 10 / 5 + 1);
    }

    #[test]
    fn test_quote_entry_short_close_long_with_overflow() {
        let mut market = test_perp_market(10.0);
        let mut pos = create_perp_position(&market, -10, 100);
        // Go long 15 @ 20
        pos.record_trade(&mut market, 15, I80F48::from(-300));
        assert_eq!(pos.quote_running_native, -100);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_native, I80F48::from(-100));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 10 * 10 / 5 + 1);
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
        assert_eq!(pos.realized_pnl_native, I80F48::from(2_000));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 1 * 10 / 5 + 1);
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
        assert_eq!(pos.realized_pnl_native, I80F48::from(20_000));
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
        assert_eq!(pos.realized_pnl_native, I80F48::from(0));

        // Sell 2 @ 4
        pos.record_trade(&mut market, -2, I80F48::from(2 * 4));

        assert!((pos.avg_entry_price(&market) - 1.66666).abs() < 0.001);
        assert!((pos.realized_pnl_native.to_num::<f64>() - 4.6666).abs() < 0.01);

        // Sell 1 @ 2
        pos.record_trade(&mut market, -1, I80F48::from(2));

        assert_eq!(pos.avg_entry_price(&market), 0.0);
        assert!((pos.quote_position_native.to_num::<f64>() - 5.1).abs() < 0.001);
        assert!((pos.realized_pnl_native.to_num::<f64>() - 5.1).abs() < 0.01);
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

        let long_pos = create_perp_position(&market, 50, -5000);
        let pnl = long_pos.pnl_for_price(&market, I80F48::from(11)).unwrap();
        assert_eq!(pnl, I80F48::from(50 * 10 * 1), "long profitable");
        let pnl = long_pos.pnl_for_price(&market, I80F48::from(9)).unwrap();
        assert_eq!(pnl, I80F48::from(50 * 10 * -1), "long unprofitable");

        let short_pos = create_perp_position(&market, -50, 5000);
        let pnl = short_pos.pnl_for_price(&market, I80F48::from(11)).unwrap();
        assert_eq!(pnl, I80F48::from(50 * 10 * -1), "short unprofitable");
        let pnl = short_pos.pnl_for_price(&market, I80F48::from(9)).unwrap();
        assert_eq!(pnl, I80F48::from(50 * 10 * 1), "short profitable");
    }

    #[test]
    fn test_perp_realized_pnl_consumption() {
        let market = test_perp_market(10.0);

        let mut pos = create_perp_position(&market, 0, 0);
        assert_eq!(pos.realized_pnl_native, I80F48::from(0));

        pos.settle_pnl_limit_realized_pnl_native = 1000;
        pos.record_settle(I80F48::from(100), I80F48::from(10));
        assert_eq!(pos.realized_pnl_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 90);

        pos.record_settle(I80F48::from(-100), I80F48::from(-20));
        assert_eq!(pos.realized_pnl_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 80);

        pos.realized_pnl_native = I80F48::from(5);
        pos.record_settle(I80F48::from(-100), I80F48::from(-20));
        assert_eq!(pos.realized_pnl_native, I80F48::from(5));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 80);

        pos.record_settle(I80F48::from(100), I80F48::from(2));
        assert_eq!(pos.realized_pnl_native, I80F48::from(3));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 80);

        pos.record_settle(I80F48::from(100), I80F48::from(10));
        assert_eq!(pos.realized_pnl_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 80);

        pos.realized_pnl_native = I80F48::from(-5);
        pos.record_settle(I80F48::from(20), I80F48::from(20));
        assert_eq!(pos.realized_pnl_native, I80F48::from(-5));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 0);

        pos.record_settle(I80F48::from(-100), I80F48::from(-2));
        assert_eq!(pos.realized_pnl_native, I80F48::from(-3));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 0);

        pos.record_settle(I80F48::from(-100), I80F48::from(-10));
        assert_eq!(pos.realized_pnl_native, I80F48::from(0));
        assert_eq!(pos.settle_pnl_limit_realized_pnl_native, 0);
    }

    #[test]
    fn test_perp_settle_limit() {
        let mut market = test_perp_market(0.5);

        let mut pos = create_perp_position(&market, 100, -50);
        pos.realized_pnl_native = I80F48::from(60); // no effect
        pos.settle_pnl_limit_realized_pnl_native = 5;

        let limited_pnl = |pos: &PerpPosition, market: &PerpMarket, pnl: i64| {
            pos.apply_pnl_settle_limit(I80F48::from(pnl), &market)
                .to_num::<f64>()
        };

        assert_eq!(pos.available_settle_limit(&market), 15); // 0.2 factor * 0.5 stable price * 100 lots + 5 realized
        assert_eq!(limited_pnl(&pos, &market, 100), 15.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -15.0);

        pos.settle_pnl_limit_settled_in_current_window_native = 2;
        assert_eq!(pos.available_settle_limit(&market), 13);
        assert_eq!(limited_pnl(&pos, &market, 100), 13.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -13.0);

        pos.settle_pnl_limit_settled_in_current_window_native = 16;
        assert_eq!(pos.available_settle_limit(&market), 0);
        assert_eq!(limited_pnl(&pos, &market, 100), 0.0);
        assert_eq!(limited_pnl(&pos, &market, -100), 0.0);

        pos.settle_pnl_limit_realized_pnl_native = 0;
        pos.settle_pnl_limit_settled_in_current_window_native = 2;
        assert_eq!(pos.available_settle_limit(&market), 8);
        assert_eq!(limited_pnl(&pos, &market, 100), 8.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -8.0);

        pos.settle_pnl_limit_settled_in_current_window_native = -2;
        assert_eq!(pos.available_settle_limit(&market), 12);
        assert_eq!(limited_pnl(&pos, &market, 100), 12.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -12.0);

        market.stable_price_model.stable_price = 1.0;
        assert_eq!(pos.available_settle_limit(&market), 22);
        assert_eq!(limited_pnl(&pos, &market, 100), 22.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -22.0);

        pos.settle_pnl_limit_realized_pnl_native = 1000;
        pos.settle_pnl_limit_settled_in_current_window_native = 2;
        assert_eq!(limited_pnl(&pos, &market, 100), 100.0);
        assert_eq!(limited_pnl(&pos, &market, -100), -100.0);
    }
}
