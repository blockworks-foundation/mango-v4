use anchor_lang::prelude::*;
use checked_math as cm;
use fixed::types::I80F48;
use static_assertions::const_assert_eq;
use std::cmp::Ordering;
use std::mem::size_of;

use crate::state::*;

pub const FREE_ORDER_SLOT: PerpMarketIndex = PerpMarketIndex::MAX;

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
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

    pub reserved: [u8; 5],
}

unsafe impl bytemuck::Pod for TokenPosition {}
unsafe impl bytemuck::Zeroable for TokenPosition {}

const_assert_eq!(size_of::<TokenPosition>(), 24);
const_assert_eq!(size_of::<TokenPosition>() % 8, 0);

impl Default for TokenPosition {
    fn default() -> Self {
        TokenPosition {
            indexed_position: I80F48::ZERO,
            token_index: TokenIndex::MAX,
            in_use_count: 0,
            reserved: Default::default(),
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
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct Serum3Orders {
    pub open_orders: Pubkey,

    // tracks reserved funds in open orders account,
    // used for bookkeeping of potentital loans which
    // can be charged with loan origination fees
    // e.g. serum3 settle funds ix
    pub previous_native_coin_reserved: u64,
    pub previous_native_pc_reserved: u64,

    pub market_index: Serum3MarketIndex,

    /// Store the base/quote token index, so health computations don't need
    /// to get passed the static SerumMarket to find which tokens a market
    /// uses and look up the correct oracles.
    pub base_token_index: TokenIndex,
    pub quote_token_index: TokenIndex,

    pub reserved: [u8; 2],
}
const_assert_eq!(size_of::<Serum3Orders>(), 32 + 8 * 2 + 2 * 3 + 2); // 56
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
            reserved: Default::default(),
            previous_native_coin_reserved: 0,
            previous_native_pc_reserved: 0,
        }
    }
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct PerpPositions {
    pub market_index: PerpMarketIndex,
    pub reserved: [u8; 6],

    /// Active position size, measured in base lots
    pub base_position_lots: i64,
    /// Active position in quote (conversation rate is that of the time the order was settled)
    /// measured in native quote
    pub quote_position_native: I80F48,

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
}

impl std::fmt::Debug for PerpPositions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerpAccount")
            .field("market_index", &self.market_index)
            .field("base_position_lots", &self.base_position_lots)
            .field("quote_position_native", &self.quote_position_native)
            .field("bids_base_lots", &self.bids_base_lots)
            .field("asks_base_lots", &self.asks_base_lots)
            .field("taker_base_lots", &self.taker_base_lots)
            .field("taker_quote_lots", &self.taker_quote_lots)
            .finish()
    }
}
const_assert_eq!(size_of::<PerpPositions>(), 8 + 8 * 5 + 3 * 16); // 96
const_assert_eq!(size_of::<PerpPositions>() % 8, 0);

unsafe impl bytemuck::Pod for PerpPositions {}
unsafe impl bytemuck::Zeroable for PerpPositions {}

impl Default for PerpPositions {
    fn default() -> Self {
        Self {
            market_index: PerpMarketIndex::MAX,
            base_position_lots: 0,
            quote_position_native: I80F48::ZERO,
            bids_base_lots: 0,
            asks_base_lots: 0,
            taker_base_lots: 0,
            taker_quote_lots: 0,
            reserved: Default::default(),
            long_settled_funding: I80F48::ZERO,
            short_settled_funding: I80F48::ZERO,
        }
    }
}

impl PerpPositions {
    /// Add taker trade after it has been matched but before it has been process on EventQueue
    pub fn add_taker_trade(&mut self, side: Side, base_lots: i64, quote_lots: i64) {
        match side {
            Side::Bid => {
                self.taker_base_lots = cm!(self.taker_base_lots + base_lots);
                self.taker_quote_lots = cm!(self.taker_quote_lots - quote_lots);
            }
            Side::Ask => {
                self.taker_base_lots = cm!(self.taker_base_lots - base_lots);
                self.taker_quote_lots = cm!(self.taker_quote_lots + quote_lots);
            }
        }
    }
    /// Remove taker trade after it has been processed on EventQueue
    pub fn remove_taker_trade(&mut self, base_change: i64, quote_change: i64) {
        self.taker_base_lots = cm!(self.taker_base_lots - base_change);
        self.taker_quote_lots = cm!(self.taker_quote_lots - quote_change);
    }

    pub fn is_active(&self) -> bool {
        self.market_index != PerpMarketIndex::MAX
    }

    pub fn is_active_for_market(&self, market_index: PerpMarketIndex) -> bool {
        self.market_index == market_index
    }

    /// This assumes settle_funding was already called
    pub fn change_base_position(&mut self, perp_market: &mut PerpMarket, base_change: i64) {
        let start = self.base_position_lots;
        self.base_position_lots += base_change;
        perp_market.open_interest += self.base_position_lots.abs() - start.abs();
    }

    /// Move unrealized funding payments into the quote_position
    pub fn settle_funding(&mut self, perp_market: &PerpMarket) {
        match self.base_position_lots.cmp(&0) {
            Ordering::Greater => {
                self.quote_position_native -= (perp_market.long_funding
                    - self.long_settled_funding)
                    * I80F48::from_num(self.base_position_lots);
            }
            Ordering::Less => {
                self.quote_position_native -= (perp_market.short_funding
                    - self.short_settled_funding)
                    * I80F48::from_num(self.base_position_lots);
            }
            Ordering::Equal => (),
        }
        self.long_settled_funding = perp_market.long_funding;
        self.short_settled_funding = perp_market.short_funding;
    }
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct PerpOpenOrders {
    pub order_side: Side, // TODO: storing enums isn't POD
    pub reserved1: [u8; 1],
    pub order_market: PerpMarketIndex,
    pub reserved2: [u8; 4],
    pub client_order_id: u64,
    pub order_id: i128,
}

impl Default for PerpOpenOrders {
    fn default() -> Self {
        Self {
            order_side: Side::Bid,
            reserved1: Default::default(),
            order_market: FREE_ORDER_SLOT,
            reserved2: Default::default(),
            client_order_id: 0,
            order_id: 0,
        }
    }
}

unsafe impl bytemuck::Pod for PerpOpenOrders {}
unsafe impl bytemuck::Zeroable for PerpOpenOrders {}

const_assert_eq!(size_of::<PerpOpenOrders>(), 1 + 1 + 2 + 4 + 8 + 16);
const_assert_eq!(size_of::<PerpOpenOrders>() % 8, 0);

#[macro_export]
macro_rules! account_seeds {
    ( $account:expr ) => {
        &[
            $account.group.as_ref(),
            b"MangoAccount".as_ref(),
            $account.owner.as_ref(),
            &$account.account_num.to_le_bytes(),
            &[$account.bump],
        ]
    };
}

pub use account_seeds;
