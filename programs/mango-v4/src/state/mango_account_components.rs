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

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 16],

    // bookkeeping variable for onchain interest calculation
    // either deposit_index or borrow_index at last indexed_position change
    pub previous_index: I80F48,

    // (Display only)
    // Cumulative deposit interest in token native units
    pub cumulative_deposit_interest: f32,
    // (Display only)
    // Cumulative borrow interest in token native units
    pub cumulative_borrow_interest: f32,
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
            reserved: [0; 16],
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
    pub padding: [u8; 6],

    /// Active position size, measured in base lots
    base_position_lots: i64,
    /// Active position in quote (conversation rate is that of the time the order was settled)
    /// measured in native quote
    quote_position_native: I80F48,

    /// Tracks what the position is to calculate average entry & break even price
    pub quote_entry_native: i64,
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

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 16],

    // bookkeeping variable for onchain funding calculation
    // either short funding index or long funding index at last base position change
    pub previous_funding_index: I80F48,
    // (Display only)
    // Cumulative long funding in base native units
    pub cumulative_long_funding: f32,
    // (Display only)
    // Cumulative short funding in base native units
    pub cumulative_short_funding: f32,
    // (Display only)
    // Cumulative maker volume in quote native units
    pub maker_volume: i64,
    // (Display only)
    // Cumulative maker volume in quote native units
    pub taker_volume: i64,
    // (Display only)
    // Cumulative realized pnl in quote native units
    pub realized_pnl: i64,
}
const_assert_eq!(size_of::<PerpPosition>(), 8 + 7 * 8 + 3 * 16 + 64);
const_assert_eq!(size_of::<PerpPosition>() % 8, 0);

unsafe impl bytemuck::Pod for PerpPosition {}
unsafe impl bytemuck::Zeroable for PerpPosition {}

impl Default for PerpPosition {
    fn default() -> Self {
        Self {
            market_index: PerpMarketIndex::MAX,
            base_position_lots: 0,
            quote_position_native: I80F48::ZERO,
            quote_entry_native: 0,
            quote_running_native: 0,
            bids_base_lots: 0,
            asks_base_lots: 0,
            taker_base_lots: 0,
            taker_quote_lots: 0,
            reserved: [0; 16],
            long_settled_funding: I80F48::ZERO,
            short_settled_funding: I80F48::ZERO,
            padding: Default::default(),
            previous_funding_index: I80F48::ZERO,
            cumulative_long_funding: 0.0,
            cumulative_short_funding: 0.0,
            maker_volume: 0,
            taker_volume: 0,
            realized_pnl: 0,
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
        self.long_settled_funding = perp_market.long_funding;
        self.short_settled_funding = perp_market.short_funding;
    }

    /// Update the quote entry position
    fn update_entry_price(&mut self, base_change: i64, quote_change_native: i64) {
        if base_change == 0 {
            return;
        }
        let old_position = self.base_position_lots;
        let is_increasing = old_position == 0 || old_position.signum() == base_change.signum();
        cm!(self.quote_running_native += quote_change_native);
        match is_increasing {
            true => {
                cm!(self.quote_entry_native += quote_change_native);
            }
            false => {
                let new_position = cm!(old_position + base_change);
                let changes_side = old_position.signum() == -new_position.signum();
                self.quote_entry_native = if changes_side {
                    cm!(((new_position as f64) * (quote_change_native as f64)
                        / (base_change as f64))
                        .round()) as i64
                } else {
                    let remaining_frac =
                        (1f64 - (base_change.abs() as f64) / (old_position.abs() as f64)).max(0f64);
                    let initial_entry = self.quote_entry_native as f64;
                    (initial_entry * remaining_frac).round() as i64
                }
            }
        }
    }

    /// Change the base and quote positions as the result of a trade
    pub fn change_base_and_quote_positions(
        &mut self,
        perp_market: &mut PerpMarket,
        base_change: i64,
        quote_change_native: I80F48,
    ) {
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

    /// Calculate the average entry price of the position
    pub fn avg_entry_price(&self) -> I80F48 {
        if self.base_position_lots == 0 {
            return I80F48::ZERO; // TODO: What should this actually return? Error? NaN?
        }
        (I80F48::from(self.quote_entry_native) / I80F48::from(self.base_position_lots)).abs()
    }

    /// Calculate the break even price of the position
    pub fn break_even_price(&self) -> I80F48 {
        if self.base_position_lots == 0 {
            return I80F48::ZERO; // TODO: What should this actually return? Error? NaN?
        }
        (I80F48::from(self.quote_running_native) / I80F48::from(self.base_position_lots)).abs()
    }
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct PerpOpenOrder {
    pub order_side: Side, // TODO: storing enums isn't POD
    pub padding1: [u8; 1],
    pub order_market: PerpMarketIndex,
    pub padding2: [u8; 4],
    pub client_order_id: u64,
    pub order_id: i128,
    pub reserved: [u8; 64],
}

impl Default for PerpOpenOrder {
    fn default() -> Self {
        Self {
            order_side: Side::Bid,
            padding1: Default::default(),
            order_market: FREE_ORDER_SLOT,
            padding2: Default::default(),
            client_order_id: 0,
            order_id: 0,
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

    fn create_perp_position(base_pos: i64, quote_pos: i64, entry_pos: i64) -> PerpPosition {
        let mut pos = PerpPosition::default();
        pos.base_position_lots = base_pos;
        pos.quote_position_native = I80F48::from(quote_pos);
        pos.quote_entry_native = entry_pos;
        pos.quote_running_native = quote_pos;
        pos
    }

    #[test]
    fn test_quote_entry_long_increasing_from_zero() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(0, 0, 0);
        // Go long 10 @ 10
        pos.change_base_and_quote_positions(&mut market, 10, I80F48::from(-100));
        assert_eq!(pos.quote_entry_native, -100);
        assert_eq!(pos.quote_running_native, -100);
        assert_eq!(pos.avg_entry_price(), I80F48::from(10));
    }

    #[test]
    fn test_quote_entry_short_increasing_from_zero() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(0, 0, 0);
        // Go short 10 @ 10
        pos.change_base_and_quote_positions(&mut market, -10, I80F48::from(100));
        assert_eq!(pos.quote_entry_native, 100);
        assert_eq!(pos.quote_running_native, 100);
        assert_eq!(pos.avg_entry_price(), I80F48::from(10));
    }

    #[test]
    fn test_quote_entry_long_increasing_from_long() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(10, -100, -100);
        // Go long 10 @ 30
        pos.change_base_and_quote_positions(&mut market, 10, I80F48::from(-300));
        assert_eq!(pos.quote_entry_native, -400);
        assert_eq!(pos.quote_running_native, -400);
        assert_eq!(pos.avg_entry_price(), I80F48::from(20));
    }

    #[test]
    fn test_quote_entry_short_increasing_from_short() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(-10, 100, 100);
        // Go short 10 @ 10
        pos.change_base_and_quote_positions(&mut market, -10, I80F48::from(300));
        assert_eq!(pos.quote_entry_native, 400);
        assert_eq!(pos.quote_running_native, 400);
        assert_eq!(pos.avg_entry_price(), I80F48::from(20));
    }

    #[test]
    fn test_quote_entry_long_decreasing_from_short() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(-10, 100, 100);
        // Go long 5 @ 50
        pos.change_base_and_quote_positions(&mut market, 5, I80F48::from(-250));
        assert_eq!(pos.quote_entry_native, 50);
        assert_eq!(pos.quote_running_native, -150);
        assert_eq!(pos.avg_entry_price(), I80F48::from(10)); // Entry price remains the same when decreasing
    }

    #[test]
    fn test_quote_entry_short_decreasing_from_long() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(10, -100, -100);
        // Go short 5 @ 50
        pos.change_base_and_quote_positions(&mut market, -5, I80F48::from(250));
        assert_eq!(pos.quote_entry_native, -50);
        assert_eq!(pos.quote_running_native, 150);
        assert_eq!(pos.avg_entry_price(), I80F48::from(10)); // Entry price remains the same when decreasing
    }

    #[test]
    fn test_quote_entry_long_close_with_short() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(10, -100, -100);
        // Go short 10 @ 50
        pos.change_base_and_quote_positions(&mut market, -10, I80F48::from(250));
        assert_eq!(pos.quote_entry_native, 0);
        assert_eq!(pos.quote_running_native, 150);
        assert_eq!(pos.avg_entry_price(), I80F48::from(0)); // Entry price zero when no position
    }

    #[test]
    fn test_quote_entry_short_close_with_long() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(-10, 100, 100);
        // Go long 10 @ 50
        pos.change_base_and_quote_positions(&mut market, 10, I80F48::from(-250));
        assert_eq!(pos.quote_entry_native, 0);
        assert_eq!(pos.quote_running_native, -150);
        assert_eq!(pos.avg_entry_price(), I80F48::from(0)); // Entry price zero when no position
    }

    #[test]
    fn test_quote_entry_long_close_short_with_overflow() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(10, -100, -100);
        // Go short 15 @ 20
        pos.change_base_and_quote_positions(&mut market, -15, I80F48::from(300));
        assert_eq!(pos.quote_entry_native, 100);
        assert_eq!(pos.quote_running_native, 200);
        assert_eq!(pos.avg_entry_price(), I80F48::from(20)); // Entry price zero when no position
    }

    #[test]
    fn test_quote_entry_short_close_long_with_overflow() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(-10, 100, 100);
        // Go short 15 @ 20
        pos.change_base_and_quote_positions(&mut market, 15, I80F48::from(-300));
        assert_eq!(pos.quote_entry_native, -100);
        assert_eq!(pos.quote_running_native, -200);
        assert_eq!(pos.avg_entry_price(), I80F48::from(20)); // Entry price zero when no position
    }

    #[test]
    fn test_quote_entry_break_even_price() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(0, 0, 0);
        // Buy 11 @ 10,000
        pos.change_base_and_quote_positions(&mut market, 11, I80F48::from(-11 * 10_000));
        // Sell 1 @ 12,000
        pos.change_base_and_quote_positions(&mut market, -1, I80F48::from(12_000));
        assert_eq!(pos.quote_entry_native, -10 * 10_000);
        assert_eq!(pos.quote_running_native, -98_000);
        assert_eq!(pos.base_position_lots, 10);
        assert_eq!(pos.break_even_price(), I80F48::from(9_800)); // We made 2k on the trade, so we can sell our contract up to a loss of 200 each
    }

    #[test]
    fn test_quote_entry_multiple_and_reversed_changes_return_entry_to_zero() {
        let mut market = PerpMarket::default_for_tests();
        let mut pos = create_perp_position(0, 0, 0);

        // Generate array of random trades
        let mut rng = rand::thread_rng();
        let mut trades: Vec<[i64; 2]> = Vec::with_capacity(500);
        for _ in 0..trades.capacity() {
            let qty: i64 = rng.gen_range(-1000..=1000);
            let px: f64 = rng.gen_range(0.1..=100.0);
            let quote: i64 = (-qty as f64 * px).round() as i64;
            trades.push([qty, quote]);
        }
        // Apply all of the trades going forward
        trades.iter().for_each(|[qty, quote]| {
            pos.change_base_and_quote_positions(&mut market, *qty, I80F48::from(*quote));
        });
        // base_position should be sum of all base quantities
        assert_eq!(
            pos.base_position_lots,
            trades.iter().map(|[qty, _]| qty).sum::<i64>()
        );
        // Reverse out all the trades
        trades.iter().for_each(|[qty, quote]| {
            pos.change_base_and_quote_positions(&mut market, -*qty, I80F48::from(-*quote));
        });
        // base position should be 0
        assert_eq!(pos.base_position_lots, 0);
        // quote entry position should be 0
        assert_eq!(pos.quote_entry_native, 0);
        // running quote should be 0
        assert_eq!(pos.quote_running_native, 0);
    }
}
