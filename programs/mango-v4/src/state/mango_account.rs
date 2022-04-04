use anchor_lang::prelude::*;
use checked_math as cm;
use fixed::types::I80F48;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::error::*;
use crate::state::*;

// todo: these are arbitrary
// ckamm: Either we put hard limits on everything, or we have a simple model for how much
// compute a token/serum/perp market needs, so users who don't use serum markets can have
// more perp markets open at the same time etc
// In particular if perp markets don't require the base token to be active on the account,
// we could probably support 1 token (quote currency) + 15 active perp markets at the same time
// It's a tradeoff between allowing users to trade on many markets with one account,
// MangoAccount size and health compute needs.
const MAX_TOKEN_ACCOUNTS: usize = 16;
const MAX_SERUM3_ACCOUNTS: usize = 8;
const MAX_PERP_ACCOUNTS: usize = 8;
pub const MAX_PERP_OPEN_ORDERS: usize = 8;

pub const FREE_ORDER_SLOT: PerpMarketIndex = PerpMarketIndex::MAX;

#[zero_copy]
pub struct TokenAccount {
    // TODO: Why did we have deposits and borrows as two different values
    //       if only one of them was allowed to be != 0 at a time?
    // todo: maybe we want to split collateral and lending?
    // todo: see https://github.com/blockworks-foundation/mango-v4/issues/1
    // todo: how does ftx do this?
    /// The deposit_index (if positive) or borrow_index (if negative) scaled position
    pub indexed_value: I80F48,

    /// index into Group.tokens
    pub token_index: TokenIndex,

    /// incremented when a market requires this position to stay alive
    pub in_use_count: u8,

    pub reserved: [u8; 5],
}
const_assert_eq!(size_of::<TokenAccount>(), 24);
const_assert_eq!(size_of::<TokenAccount>() % 8, 0);

impl TokenAccount {
    pub fn is_active(&self) -> bool {
        self.token_index != TokenIndex::MAX
    }

    pub fn is_active_for_token(&self, token_index: TokenIndex) -> bool {
        self.token_index == token_index
    }

    pub fn native(&self, bank: &Bank) -> I80F48 {
        if self.indexed_value.is_positive() {
            self.indexed_value * bank.deposit_index
        } else {
            self.indexed_value * bank.borrow_index
        }
    }

    pub fn is_in_use(&self) -> bool {
        self.in_use_count > 0
    }
}

#[zero_copy]
pub struct MangoAccountTokens {
    pub values: [TokenAccount; MAX_TOKEN_ACCOUNTS],
}
const_assert_eq!(
    size_of::<MangoAccountTokens>(),
    MAX_TOKEN_ACCOUNTS * size_of::<TokenAccount>()
);
const_assert_eq!(size_of::<MangoAccountTokens>() % 8, 0);

impl Default for MangoAccountTokens {
    fn default() -> Self {
        Self::new()
    }
}

impl MangoAccountTokens {
    pub fn new() -> Self {
        Self {
            values: [TokenAccount {
                indexed_value: I80F48::ZERO,
                token_index: TokenIndex::MAX,
                in_use_count: 0,
                reserved: Default::default(),
            }; MAX_TOKEN_ACCOUNTS],
        }
    }

    pub fn get(&self, token_index: TokenIndex) -> Result<&TokenAccount> {
        self.values
            .iter()
            .find(|p| p.is_active_for_token(token_index))
            .ok_or_else(|| error!(MangoError::SomeError)) // TODO: not found error
    }

    pub fn get_mut(&mut self, token_index: TokenIndex) -> Result<&mut TokenAccount> {
        self.values
            .iter_mut()
            .find(|p| p.is_active_for_token(token_index))
            .ok_or_else(|| error!(MangoError::SomeError)) // TODO: not found error
    }

    pub fn get_mut_or_create(
        &mut self,
        token_index: TokenIndex,
    ) -> Result<(&mut TokenAccount, usize)> {
        // This function looks complex because of lifetimes.
        // Maybe there's a smart way to write it with double iter_mut()
        // that doesn't confuse the borrow checker.
        let mut pos = self
            .values
            .iter()
            .position(|p| p.is_active_for_token(token_index));
        if pos.is_none() {
            pos = self.values.iter().position(|p| !p.is_active());
            if let Some(i) = pos {
                self.values[i] = TokenAccount {
                    indexed_value: I80F48::ZERO,
                    token_index,
                    in_use_count: 0,
                    reserved: Default::default(),
                };
            }
        }
        if let Some(i) = pos {
            Ok((&mut self.values[i], i))
        } else {
            err!(MangoError::SomeError) // TODO: No free space
        }
    }

    pub fn deactivate(&mut self, index: usize) {
        assert!(self.values[index].in_use_count == 0);
        self.values[index].token_index = TokenIndex::MAX;
    }

    pub fn iter_active(&self) -> impl Iterator<Item = &TokenAccount> {
        self.values.iter().filter(|p| p.is_active())
    }

    pub fn find(&self, token_index: TokenIndex) -> Option<&TokenAccount> {
        self.values
            .iter()
            .find(|p| p.is_active_for_token(token_index))
    }
}

#[zero_copy]
pub struct Serum3Account {
    pub open_orders: Pubkey,

    pub market_index: Serum3MarketIndex,

    /// Store the base/quote token index, so health computations don't need
    /// to get passed the static SerumMarket to find which tokens a market
    /// uses and look up the correct oracles.
    pub base_token_index: TokenIndex,
    pub quote_token_index: TokenIndex,

    pub reserved: [u8; 2],
}
const_assert_eq!(size_of::<Serum3Account>(), 40);
const_assert_eq!(size_of::<Serum3Account>() % 8, 0);

impl Serum3Account {
    pub fn is_active(&self) -> bool {
        self.market_index != Serum3MarketIndex::MAX
    }

    pub fn is_active_for_market(&self, market_index: Serum3MarketIndex) -> bool {
        self.market_index == market_index
    }
}

impl Default for Serum3Account {
    fn default() -> Self {
        Self {
            open_orders: Pubkey::default(),
            market_index: Serum3MarketIndex::MAX,
            base_token_index: TokenIndex::MAX,
            quote_token_index: TokenIndex::MAX,
            reserved: Default::default(),
        }
    }
}

#[zero_copy]
pub struct MangoAccountSerum3 {
    pub values: [Serum3Account; MAX_SERUM3_ACCOUNTS],
}
const_assert_eq!(
    size_of::<MangoAccountSerum3>(),
    MAX_SERUM3_ACCOUNTS * size_of::<Serum3Account>()
);
const_assert_eq!(size_of::<MangoAccountSerum3>() % 8, 0);

impl Default for MangoAccountSerum3 {
    fn default() -> Self {
        Self::new()
    }
}

impl MangoAccountSerum3 {
    pub fn new() -> Self {
        Self {
            values: [Serum3Account::default(); MAX_SERUM3_ACCOUNTS],
        }
    }

    pub fn create(&mut self, market_index: Serum3MarketIndex) -> Result<&mut Serum3Account> {
        if self.find(market_index).is_some() {
            return err!(MangoError::SomeError); // exists already
        }
        if let Some(v) = self.values.iter_mut().find(|p| !p.is_active()) {
            *v = Serum3Account {
                market_index: market_index as Serum3MarketIndex,
                ..Serum3Account::default()
            };
            Ok(v)
        } else {
            err!(MangoError::SomeError) // no space
        }
    }

    pub fn deactivate(&mut self, index: usize) {
        self.values[index].market_index = Serum3MarketIndex::MAX;
    }

    pub fn iter_active(&self) -> impl Iterator<Item = &Serum3Account> {
        self.values.iter().filter(|p| p.is_active())
    }

    pub fn find(&self, market_index: Serum3MarketIndex) -> Option<&Serum3Account> {
        self.values
            .iter()
            .find(|p| p.is_active_for_market(market_index))
    }
}

#[zero_copy]
pub struct PerpAccount {
    pub market_index: PerpMarketIndex,
    pub reserved: [u8; 6],

    /// Active position size, measured in base lots
    pub base_position_lots: i64,
    /// Active position in quote (conversation rate is that of the time the order was settled)
    /// measured in native quote
    pub quote_position_native: I80F48,

    /// Already settled funding
    // pub long_settled_funding: I80F48,
    // pub short_settled_funding: I80F48,

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
const_assert_eq!(size_of::<PerpAccount>(), 8 + 8 * 5 + 16);
const_assert_eq!(size_of::<PerpAccount>() % 8, 0);

impl Default for PerpAccount {
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
        }
    }
}

impl PerpAccount {
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
}

#[zero_copy]
pub struct MangoAccountPerps {
    pub accounts: [PerpAccount; MAX_PERP_ACCOUNTS],

    // TODO: possibly it's more convenient to store a single list of PerpOpenOrder structs?
    pub order_market: [PerpMarketIndex; MAX_PERP_OPEN_ORDERS],
    pub order_side: [Side; MAX_PERP_OPEN_ORDERS], // TODO: storing enums isn't POD
    pub order_id: [i128; MAX_PERP_OPEN_ORDERS],
    pub order_client_id: [u64; MAX_PERP_OPEN_ORDERS],
}
const_assert_eq!(
    size_of::<MangoAccountPerps>(),
    MAX_PERP_ACCOUNTS * size_of::<PerpAccount>() + MAX_PERP_OPEN_ORDERS * (2 + 1 + 16 + 8)
);
const_assert_eq!(size_of::<MangoAccountPerps>() % 8, 0);

impl MangoAccountPerps {
    pub fn new() -> Self {
        Self {
            accounts: [PerpAccount::default(); MAX_PERP_ACCOUNTS],
            order_market: [FREE_ORDER_SLOT; MAX_PERP_OPEN_ORDERS],
            order_side: [Side::Bid; MAX_PERP_OPEN_ORDERS],
            order_id: [0; MAX_PERP_OPEN_ORDERS],
            order_client_id: [0; MAX_PERP_OPEN_ORDERS],
        }
    }

    pub fn get_account_mut_or_create(
        &mut self,
        perp_market_index: PerpMarketIndex,
    ) -> Result<(&mut PerpAccount, usize)> {
        let mut pos = self
            .accounts
            .iter()
            .position(|p| p.is_active_for_market(perp_market_index));
        if pos.is_none() {
            pos = self.accounts.iter().position(|p| !p.is_active());
            if let Some(i) = pos {
                self.accounts[i] = PerpAccount {
                    market_index: perp_market_index,
                    ..Default::default()
                };
            }
        }
        if let Some(i) = pos {
            Ok((&mut self.accounts[i], i))
        } else {
            err!(MangoError::SomeError) // TODO: No free space
        }
    }

    pub fn deactivate_account(&mut self, index: usize) {
        self.accounts[index].market_index = PerpMarketIndex::MAX;
    }

    pub fn iter_active_accounts(&self) -> impl Iterator<Item = &PerpAccount> {
        self.accounts.iter().filter(|p| p.is_active())
    }

    pub fn find_account(&self, market_index: PerpMarketIndex) -> Option<&PerpAccount> {
        self.accounts
            .iter()
            .find(|p| p.is_active_for_market(market_index))
    }

    pub fn next_order_slot(&self) -> Option<usize> {
        self.order_market.iter().position(|&i| i == FREE_ORDER_SLOT)
    }

    pub fn add_order(
        &mut self,
        perp_market_index: PerpMarketIndex,
        side: Side,
        order: &LeafNode,
    ) -> Result<()> {
        let mut perp_account = self.get_account_mut_or_create(perp_market_index).unwrap().0;
        match side {
            Side::Bid => {
                perp_account.bids_base_lots = cm!(perp_account.bids_base_lots + order.quantity);
            }
            Side::Ask => {
                perp_account.asks_base_lots = cm!(perp_account.asks_base_lots + order.quantity);
            }
        };
        let slot = order.owner_slot as usize;
        self.order_market[slot] = perp_market_index;
        self.order_side[slot] = side;
        self.order_id[slot] = order.key;
        self.order_client_id[slot] = order.client_order_id;
        Ok(())
    }

    pub fn remove_order(&mut self, slot: usize, quantity: i64) -> Result<()> {
        require!(
            self.order_market[slot] != FREE_ORDER_SLOT,
            MangoError::SomeError
        );
        let order_side = self.order_side[slot];
        let perp_market_index = self.order_market[slot];
        let perp_account = self.get_account_mut_or_create(perp_market_index).unwrap().0;

        // accounting
        match order_side {
            Side::Bid => {
                perp_account.bids_base_lots = cm!(perp_account.bids_base_lots - quantity);
            }
            Side::Ask => {
                perp_account.asks_base_lots = cm!(perp_account.asks_base_lots - quantity);
            }
        }

        // release space
        self.order_market[slot] = FREE_ORDER_SLOT;

        // TODO OPT - remove these; unnecessary
        self.order_side[slot] = Side::Bid;
        self.order_id[slot] = 0i128;
        self.order_client_id[slot] = 0u64;
        Ok(())
    }

    pub fn execute_maker(
        &mut self,
        perp_market_index: PerpMarketIndex,
        perp_market: &mut PerpMarket,
        fill: &FillEvent,
    ) -> Result<()> {
        let pa = self.get_account_mut_or_create(perp_market_index).unwrap().0;
        // pa.settle_funding(cache);

        let side = fill.taker_side.invert_side();
        let (base_change, quote_change) = fill.base_quote_change(side);
        pa.change_base_position(perp_market, base_change);
        let quote = I80F48::from_num(
            perp_market
                .quote_lot_size
                .checked_mul(quote_change)
                .unwrap(),
        );
        let fees = quote.abs() * fill.maker_fee;
        if !fill.market_fees_applied {
            perp_market.fees_accrued += fees;
        }
        pa.quote_position_native = pa.quote_position_native.checked_add(quote - fees).unwrap();

        if fill.maker_out {
            self.remove_order(fill.maker_slot as usize, base_change.abs())
        } else {
            match side {
                Side::Bid => {
                    pa.bids_base_lots = cm!(pa.bids_base_lots - base_change.abs());
                }
                Side::Ask => {
                    pa.asks_base_lots = cm!(pa.asks_base_lots - base_change.abs());
                }
            }
            Ok(())
        }
    }

    pub fn execute_taker(
        &mut self,
        perp_market_index: PerpMarketIndex,
        perp_market: &mut PerpMarket,
        fill: &FillEvent,
    ) -> Result<()> {
        let pa = self.get_account_mut_or_create(perp_market_index).unwrap().0;
        let (base_change, quote_change) = fill.base_quote_change(fill.taker_side);
        pa.remove_taker_trade(base_change, quote_change);
        pa.change_base_position(perp_market, base_change);
        let quote = I80F48::from_num(perp_market.quote_lot_size * quote_change);

        // fees are assessed at time of trade; no need to assess fees here

        pa.quote_position_native += quote;
        Ok(())
    }
}

impl Default for MangoAccountPerps {
    fn default() -> Self {
        Self::new()
    }
}

#[account(zero_copy)]
pub struct MangoAccount {
    pub group: Pubkey,
    pub owner: Pubkey,

    // Alternative authority/signer of transactions for a mango account
    pub delegate: Pubkey,

    // Maps token_index -> deposit/borrow account for each token
    // that is active on this MangoAccount.
    pub tokens: MangoAccountTokens,

    // Maps serum_market_index -> open orders for each serum market
    // that is active on this MangoAccount.
    pub serum3: MangoAccountSerum3,

    pub perps: MangoAccountPerps,

    /// This account cannot open new positions or borrow until `init_health >= 0`
    pub being_liquidated: u8,

    /// This account cannot do anything except go through `resolve_bankruptcy`
    pub is_bankrupt: u8,

    pub account_num: u8,
    pub bump: u8,

    // pub info: [u8; INFO_LEN], // TODO: Info could be in a separate PDA?
    pub reserved: [u8; 4],
}
const_assert_eq!(
    size_of::<MangoAccount>(),
    3 * 32
        + size_of::<MangoAccountTokens>()
        + size_of::<MangoAccountSerum3>()
        + size_of::<MangoAccountPerps>()
        + 4
        + 4
);
const_assert_eq!(size_of::<MangoAccount>() % 8, 0);

#[macro_export]
macro_rules! account_seeds {
    ( $account:expr ) => {
        &[
            $account.group.as_ref(),
            b"account".as_ref(),
            $account.owner.as_ref(),
            &$account.account_num.to_le_bytes(),
            &[$account.bump],
        ]
    };
}

pub use account_seeds;
