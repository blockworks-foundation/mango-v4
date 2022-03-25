use anchor_lang::prelude::*;
use fixed::types::I80F48;

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
const MAX_INDEXED_POSITIONS: usize = 16;
const MAX_SERUM_OPEN_ORDERS: usize = 8;
const MAX_PERP_OPEN_ORDERS: usize = 8;

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
}
// TODO: static assert the size and alignment

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
pub struct TokenAccountMap {
    pub values: [TokenAccount; MAX_INDEXED_POSITIONS],
}

impl Default for TokenAccountMap {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenAccountMap {
    pub fn new() -> Self {
        Self {
            values: [TokenAccount {
                indexed_value: I80F48::ZERO,
                token_index: TokenIndex::MAX,
                in_use_count: 0,
            }; MAX_INDEXED_POSITIONS],
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
}
// TODO: static assert the size and alignment

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
        }
    }
}

#[zero_copy]
pub struct Serum3AccountMap {
    pub values: [Serum3Account; MAX_SERUM_OPEN_ORDERS],
}

impl Default for Serum3AccountMap {
    fn default() -> Self {
        Self::new()
    }
}

impl Serum3AccountMap {
    pub fn new() -> Self {
        Self {
            values: [Serum3Account::default(); MAX_SERUM_OPEN_ORDERS],
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

    /// Active position size, measured in base lots
    pub base_position: i64,
    /// Active position in quote (conversation rate is that of the time the order was settled)
    /// measured in native quote
    pub quote_position: I80F48,

    /// Already settled funding
    // pub long_settled_funding: I80F48,
    // pub short_settled_funding: I80F48,

    // Contracts/quote in open orders
    /// total contracts in sell orders
    pub bids_quantity: i64,
    /// total quote currency in buy orders
    pub asks_quantity: i64,

    /// Liquidity mining rewards
    // pub mngo_accrued: u64,

    /// Amount that's on EventQueue waiting to be processed
    pub taker_base: i64,
    pub taker_quote: i64,
}

impl Default for PerpAccount {
    fn default() -> Self {
        Self {
            market_index: PerpMarketIndex::MAX,
            base_position: 0,
            quote_position: I80F48::ZERO,
            bids_quantity: 0,
            asks_quantity: 0,
            taker_base: 0,
            taker_quote: 0,
        }
    }
}

impl PerpAccount {
    /// Add taker trade after it has been matched but before it has been process on EventQueue
    pub fn add_taker_trade(&mut self, base_change: i64, quote_change: i64) {
        // TODO make checked? estimate chances of overflow here
        self.taker_base += base_change;
        self.taker_quote += quote_change;
    }
    /// Remove taker trade after it has been processed on EventQueue
    pub fn remove_taker_trade(&mut self, base_change: i64, quote_change: i64) {
        self.taker_base -= base_change;
        self.taker_quote -= quote_change;
    }

    pub fn is_active(&self) -> bool {
        self.market_index != PerpMarketIndex::MAX
    }

    pub fn is_active_for_market(&self, market_index: PerpMarketIndex) -> bool {
        self.market_index == market_index
    }
}

#[zero_copy]
pub struct PerpAccountMap {
    pub values: [PerpAccount; MAX_PERP_OPEN_ORDERS],
}

impl PerpAccountMap {
    pub fn new() -> Self {
        Self {
            values: [PerpAccount::default(); MAX_PERP_OPEN_ORDERS],
        }
    }

    pub fn get_mut_or_create(
        &mut self,
        perp_market_index: PerpMarketIndex,
    ) -> Result<(&mut PerpAccount, usize)> {
        let mut pos = self
            .values
            .iter()
            .position(|p| p.is_active_for_market(perp_market_index));
        if pos.is_none() {
            pos = self.values.iter().position(|p| !p.is_active());
            if let Some(i) = pos {
                self.values[i] = PerpAccount {
                    ..Default::default()
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
        self.values[index].market_index = PerpMarketIndex::MAX;
    }

    pub fn iter_active(&self) -> impl Iterator<Item = &PerpAccount> {
        self.values.iter().filter(|p| p.is_active())
    }

    pub fn find(&self, market_index: PerpMarketIndex) -> Option<&PerpAccount> {
        self.values
            .iter()
            .find(|p| p.is_active_for_market(market_index))
    }
}

impl Default for PerpAccountMap {
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
    pub token_account_map: TokenAccountMap,

    // Maps serum_market_index -> open orders for each serum market
    // that is active on this MangoAccount.
    pub serum3_account_map: Serum3AccountMap,

    pub perp_account_map: PerpAccountMap,

    /// This account cannot open new positions or borrow until `init_health >= 0`
    pub being_liquidated: bool, // TODO: for strict Pod compat, these should be u8, not bool

    /// This account cannot do anything except go through `resolve_bankruptcy`
    pub is_bankrupt: bool,

    pub account_num: u8,
    pub bump: u8,

    // pub info: [u8; INFO_LEN], // TODO: Info could be in a separate PDA?
    pub reserved: [u8; 5],
}
// TODO: static assert the size and alignment

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
