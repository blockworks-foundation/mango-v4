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
#[derive(AnchorDeserialize, AnchorSerialize, Derivative)]
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

    // TODO: When re-layouting: move this to the end
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 8],

    // bookkeeping variable for onchain interest calculation
    // either deposit_index or borrow_index at last indexed_position change
    pub previous_index: I80F48,
    // (Display only)
    // Cumulative deposit interest in token native units
    pub cumulative_deposit_interest: f64,
    // (Display only)
    // Cumulative borrow interest in token native units
    pub cumulative_borrow_interest: f64,
}

unsafe impl bytemuck::Pod for TokenPosition {}
unsafe impl bytemuck::Zeroable for TokenPosition {}

const_assert_eq!(size_of::<TokenPosition>(), 64);
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
            reserved: [0; 8],
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
#[derive(AnchorSerialize, AnchorDeserialize, Derivative)]
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
const_assert_eq!(size_of::<Serum3Orders>() % 8, 0);

unsafe impl bytemuck::Pod for Serum3Orders {}
unsafe impl bytemuck::Zeroable for Serum3Orders {}

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
#[derive(AnchorSerialize, AnchorDeserialize, Derivative)]
#[derivative(Debug)]
pub struct PerpPosition {
    pub market_index: PerpMarketIndex,
    #[derivative(Debug = "ignore")]
    pub padding: [u8; 2],

    pub settle_pnl_limit_window: u32,

    /// Active position size, measured in base lots
    base_position_lots: i64,
    /// Active position in quote (conversation rate is that of the time the order was settled)
    /// measured in native quote
    quote_position_native: I80F48,

    /// Tracks what the position is to calculate average entry & break even price
    pub padding2: [u8; 8],
    pub quote_running_native: i64,

    /// Already settled funding
    pub long_settled_funding: I80F48,
    pub short_settled_funding: I80F48,

    /// Base lots in bids
    pub bids_base_lots: i64,
    /// Base lots in asks
    pub asks_base_lots: i64,

    /// Liquidity mining rewards
    // pub mngo_accrued: u64,

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

    pub realized_pnl_native: i64,

    pub settle_pnl_limit_settled_in_current_window_native: i64,
    // #[derivative(Debug = "ignore")]
    // pub reserved: [u8; 8],
}
const_assert_eq!(size_of::<PerpPosition>(), 176);
const_assert_eq!(size_of::<PerpPosition>() % 8, 0);

unsafe impl bytemuck::Pod for PerpPosition {}
unsafe impl bytemuck::Zeroable for PerpPosition {}

impl Default for PerpPosition {
    fn default() -> Self {
        Self {
            market_index: PerpMarketIndex::MAX,
            base_position_lots: 0,
            quote_position_native: I80F48::ZERO,
            padding2: Default::default(),
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
            realized_pnl_native: 0,
            settle_pnl_limit_window: 0,
            settle_pnl_limit_settled_in_current_window_native: 0,
            //reserved: Default::default(),
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

        if self.base_position_lots.is_positive() {
            self.cumulative_long_funding += funding.to_num::<f64>();
        } else {
            self.cumulative_short_funding -= funding.to_num::<f64>();
        }

        self.long_settled_funding = perp_market.long_funding;
        self.short_settled_funding = perp_market.short_funding;
    }

    /// Update the quote entry position
    fn update_entry_price(&mut self, base_change: i64, quote_change_native: i64) {
        if base_change == 0 {
            return;
        }

        let old_position = self.base_position_lots;
        let new_position = cm!(old_position + base_change);

        if new_position == 0 {
            self.avg_entry_price_per_base_lot = 0.0;
            self.quote_running_native = 0;
            return;
        } else if old_position.signum() != new_position.signum() {
            // If the base position changes sign, reset
            self.avg_entry_price_per_base_lot =
                ((quote_change_native as f64) / (base_change as f64)).abs();
            self.quote_running_native =
                -((new_position as f64) * self.avg_entry_price_per_base_lot).round() as i64;
            return;
        }

        // Track all quote changes as long as the base position sign stays the same
        cm!(self.quote_running_native += quote_change_native);

        let is_increasing = old_position.signum() == base_change.signum();
        if is_increasing {
            let new_position_quote_value = (old_position.abs() as f64)
                * self.avg_entry_price_per_base_lot
                + (quote_change_native.abs() as f64);
            self.avg_entry_price_per_base_lot =
                new_position_quote_value / (new_position.abs() as f64);
        }
        // The average entry price does not change when the position decreases while keeping sign.
    }

    fn update_realized_pnl(&mut self, base_change: i64, quote_change_native: I80F48) {
        let old_position = self.base_position_lots;
        let new_position = cm!(old_position + base_change);

        if new_position == 0 {
            // There can't be unrealized pnl without a base position, so fix the
            // realized pnl to cover the whole quote position.
            // Always round away from 0, to ensure all fractional pnl can be settled
            let pnl = cm!(self.quote_position_native + quote_change_native);
            self.realized_pnl_native = if pnl.is_positive() {
                pnl.ceil()
            } else {
                pnl.floor()
            }
            .checked_to_num::<i64>()
            .unwrap();
        } else if old_position != 0 && old_position.signum() != base_change.signum() {
            let avg_entry_price_lots = I80F48::from_num(self.avg_entry_price_per_base_lot);
            if old_position.abs() == base_change.abs() {
                let new_realized_pnl =
                    cm!(quote_change_native + I80F48::from(base_change) * avg_entry_price_lots)
                        .checked_to_num::<i64>()
                        .unwrap();
                cm!(self.realized_pnl_native += new_realized_pnl);
            } else {
                let reduced_lots = I80F48::from(old_position.abs().min(base_change.abs()));
                cm!(
                    self.realized_pnl_native += (quote_change_native * reduced_lots
                        / I80F48::from(base_change.abs())
                        + I80F48::from(base_change.signum()) * reduced_lots * avg_entry_price_lots)
                        .checked_to_num::<i64>()
                        .unwrap()
                );
            }
        }
    }

    /// Change the base and quote positions as the result of a trade
    pub fn record_trade(
        &mut self,
        perp_market: &mut PerpMarket,
        base_change: i64,
        quote_change_native: I80F48,
    ) {
        assert_eq!(perp_market.perp_market_index, self.market_index);
        self.update_realized_pnl(base_change, quote_change_native);
        self.update_entry_price(
            base_change,
            quote_change_native.round().checked_to_num().unwrap(),
        );
        self.change_base_position(perp_market, base_change);
        self.change_quote_position(quote_change_native);
    }

    pub fn change_quote_position(&mut self, quote_change_native: I80F48) {
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
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct PerpOpenOrder {
    pub side_and_tree: SideAndOrderTree, // TODO: storing enums isn't POD
    pub padding1: [u8; 1],
    pub market: PerpMarketIndex,
    pub padding2: [u8; 4],
    pub client_id: u64,
    pub id: u128,
    pub reserved: [u8; 64],
}

impl Default for PerpOpenOrder {
    fn default() -> Self {
        Self {
            side_and_tree: SideAndOrderTree::BidFixed,
            padding1: Default::default(),
            market: FREE_ORDER_SLOT,
            padding2: Default::default(),
            client_id: 0,
            id: 0,
            reserved: [0; 64],
        }
    }
}

unsafe impl bytemuck::Pod for PerpOpenOrder {}
unsafe impl bytemuck::Zeroable for PerpOpenOrder {}

const_assert_eq!(size_of::<PerpOpenOrder>(), 1 + 1 + 2 + 4 + 8 + 16 + 64);
const_assert_eq!(size_of::<PerpOpenOrder>() % 8, 0);

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

    #[test]
    fn test_quote_entry_long_increasing_from_zero() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, 0, 0);
        // Go long 10 @ 10
        pos.record_trade(&mut market, 10, I80F48::from(-100));
        assert_eq!(pos.quote_running_native, -100);
        assert_eq!(pos.avg_entry_price(&market), 10.0);
        assert_eq!(pos.break_even_price(&market), 10.0);
        assert_eq!(pos.realized_pnl_native, 0);
    }

    #[test]
    fn test_quote_entry_short_increasing_from_zero() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, 0, 0);
        // Go short 10 @ 10
        pos.record_trade(&mut market, -10, I80F48::from(100));
        assert_eq!(pos.quote_running_native, 100);
        assert_eq!(pos.avg_entry_price(&market), 10.0);
        assert_eq!(pos.break_even_price(&market), 10.0);
        assert_eq!(pos.realized_pnl_native, 0);
    }

    #[test]
    fn test_quote_entry_long_increasing_from_long() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, 10, -100);
        // Go long 10 @ 30
        pos.record_trade(&mut market, 10, I80F48::from(-300));
        assert_eq!(pos.quote_running_native, -400);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_native, 0);
    }

    #[test]
    fn test_quote_entry_short_increasing_from_short() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, -10, 100);
        // Go short 10 @ 10
        pos.record_trade(&mut market, -10, I80F48::from(300));
        assert_eq!(pos.quote_running_native, 400);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_native, 0);
    }

    #[test]
    fn test_quote_entry_long_decreasing_from_short() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, -10, 100);
        // Go long 5 @ 50
        pos.record_trade(&mut market, 5, I80F48::from(-250));
        assert_eq!(pos.quote_running_native, -150);
        assert_eq!(pos.avg_entry_price(&market), 10.0); // Entry price remains the same when decreasing
        assert_eq!(pos.break_even_price(&market), -30.0); // Already broke even
        assert_eq!(pos.realized_pnl_native, -200);
    }

    #[test]
    fn test_quote_entry_short_decreasing_from_long() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, 10, -100);
        // Go short 5 @ 50
        pos.record_trade(&mut market, -5, I80F48::from(250));
        assert_eq!(pos.quote_running_native, 150);
        assert_eq!(pos.avg_entry_price(&market), 10.0); // Entry price remains the same when decreasing
        assert_eq!(pos.break_even_price(&market), -30.0); // Already broke even
        assert_eq!(pos.realized_pnl_native, 200);
    }

    #[test]
    fn test_quote_entry_long_close_with_short() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, 10, -100);
        // Go short 10 @ 25
        pos.record_trade(&mut market, -10, I80F48::from(250));
        assert_eq!(pos.quote_running_native, 0);
        assert_eq!(pos.avg_entry_price(&market), 0.0); // Entry price zero when no position
        assert_eq!(pos.break_even_price(&market), 0.0);
        assert_eq!(pos.realized_pnl_native, 150);
    }

    #[test]
    fn test_quote_entry_short_close_with_long() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, -10, 100);
        // Go long 10 @ 25
        pos.record_trade(&mut market, 10, I80F48::from(-250));
        assert_eq!(pos.quote_running_native, 0);
        assert_eq!(pos.avg_entry_price(&market), 0.0); // Entry price zero when no position
        assert_eq!(pos.break_even_price(&market), 0.0);
        assert_eq!(pos.realized_pnl_native, -150);
    }

    #[test]
    fn test_quote_entry_long_close_short_with_overflow() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, 10, -100);
        // Go short 15 @ 20
        pos.record_trade(&mut market, -15, I80F48::from(300));
        assert_eq!(pos.quote_running_native, 100);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_native, 100);
    }

    #[test]
    fn test_quote_entry_short_close_long_with_overflow() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, -10, 100);
        // Go short 15 @ 20
        pos.record_trade(&mut market, 15, I80F48::from(-300));
        assert_eq!(pos.quote_running_native, -100);
        assert_eq!(pos.avg_entry_price(&market), 20.0);
        assert_eq!(pos.break_even_price(&market), 20.0);
        assert_eq!(pos.realized_pnl_native, -100);
    }

    #[test]
    fn test_quote_entry_break_even_price() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, 0, 0);
        // Buy 11 @ 10,000
        pos.record_trade(&mut market, 11, I80F48::from(-11 * 10_000));
        // Sell 1 @ 12,000
        pos.record_trade(&mut market, -1, I80F48::from(12_000));
        assert_eq!(pos.quote_running_native, -98_000);
        assert_eq!(pos.base_position_lots, 10);
        assert_eq!(pos.break_even_price(&market), 9_800.0); // We made 2k on the trade, so we can sell our contract up to a loss of 200 each
        assert_eq!(pos.realized_pnl_native, 2_000);
    }

    #[test]
    fn test_entry_and_break_even_prices_with_lots() {
        let mut market = PerpMarket::default_for_tests();
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
        assert_eq!(pos.realized_pnl_native, 20_000);
    }

    #[test]
    fn test_realized_pnl_fractional() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(&market, 0, 0);
        pos.quote_position_native += I80F48::from_num(0.1);

        // Buy 1 @ 1
        pos.record_trade(&mut market, 1, I80F48::from(-1));
        // Buy 2 @ 2
        pos.record_trade(&mut market, 2, I80F48::from(-2 * 2));

        assert!((pos.avg_entry_price(&market) - 1.66666).abs() < 0.001);
        assert_eq!(pos.realized_pnl_native, 0);

        // Sell 2 @ 4
        pos.record_trade(&mut market, -2, I80F48::from(2 * 4));

        assert!((pos.avg_entry_price(&market) - 1.66666).abs() < 0.001);
        assert_eq!(pos.realized_pnl_native, 4); // 4.666 rounded down

        // Sell 1 @ 2
        pos.record_trade(&mut market, -1, I80F48::from(2));

        assert_eq!(pos.avg_entry_price(&market), 0.0);
        assert!((pos.quote_position_native.to_num::<f64>() - 5.1).abs() < 0.001);
        assert_eq!(pos.realized_pnl_native, 6); // quote position rounded up
    }

    #[test]
    fn test_perp_entry_multiple_random_long() {
        let mut market = PerpMarket::default_for_tests();
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
        let mut market = PerpMarket::default_for_tests();
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
}
