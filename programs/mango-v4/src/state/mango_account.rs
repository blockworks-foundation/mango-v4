use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::error::*;
use crate::state::*;

const MAX_INDEXED_POSITIONS: usize = 32;

#[zero_copy]
pub struct IndexedPosition {
    // TODO: Why did we have deposits and borrows as two different values
    //       if only one of them was allowed to be != 0 at a time?
    // todo: maybe we want to split collateral and lending?
    // todo: see https://github.com/blockworks-foundation/mango-v4/issues/1
    // todo: how does ftx do this?
    /// The deposit_index (if positive) or borrow_index (if negative) scaled position
    pub indexed_value: I80F48,

    /// index into MangoGroup.tokens
    pub token_index: TokenIndex,
}
// TODO: static assert the size and alignment

impl IndexedPosition {
    pub fn is_active(&self) -> bool {
        // maybe want to reserve token_index == 0?
        // TODO: possibly consider inactive if there's less than one native token there? - that's impossible to withdraw...
        self.indexed_value != I80F48::ZERO
    }

    pub fn is_active_for_index(&self, index: usize) -> bool {
        self.token_index as usize == index && self.is_active()
    }

    pub fn native(&self, bank: &TokenBank) -> I80F48 {
        if self.indexed_value.is_positive() {
            self.indexed_value * bank.deposit_index
        } else {
            self.indexed_value * bank.borrow_index
        }
    }
}

#[zero_copy]
pub struct IndexedPositions {
    pub values: [IndexedPosition; MAX_INDEXED_POSITIONS],
}

impl IndexedPositions {
    pub fn get_mut(&mut self, token_index: usize) -> Result<&mut IndexedPosition> {
        self.values
            .iter_mut()
            .find(|p| p.is_active_for_index(token_index))
            .ok_or_else(|| error!(MangoError::SomeError)) // TODO: not found error
    }

    pub fn get_mut_or_create(&mut self, token_index: usize) -> Result<&mut IndexedPosition> {
        // This function looks complex because of lifetimes.
        // Maybe there's a smart way to write it with double iter_mut()
        // that doesn't confuse the borrow checker.
        let mut pos = self
            .values
            .iter()
            .position(|p| p.is_active_for_index(token_index));
        if pos.is_none() {
            pos = self.values.iter().position(|p| !p.is_active());
            if let Some(i) = pos {
                self.values[i] = IndexedPosition {
                    indexed_value: I80F48::ZERO,
                    token_index: token_index as TokenIndex,
                };
            }
        }
        if let Some(i) = pos {
            Ok(&mut self.values[i])
        } else {
            err!(MangoError::SomeError) // TODO: No free space
        }
    }

    pub fn iter_active(&self) -> impl Iterator<Item = &IndexedPosition> {
        self.values.iter().filter(|p| p.is_active())
    }
}

#[account(zero_copy)]
pub struct MangoAccount {
    pub group: Pubkey,
    pub owner: Pubkey,

    // Alternative authority/signer of transactions for a mango account
    pub delegate: Pubkey,

    pub address_lookup_table: Pubkey,

    // pub in_margin_basket: [bool; MAX_PAIRS],
    // pub num_in_margin_basket: u8,
    // TODO: this should be a separate struct for convenient use, like MangoGroup::tokens
    pub indexed_positions: IndexedPositions,

    // pub spot_open_orders: [Pubkey; MAX_PAIRS],
    // pub perp_accounts: [PerpAccount; MAX_PAIRS],

    // pub order_market: [u8; MAX_PERP_OPEN_ORDERS],
    // pub order_side: [Side; MAX_PERP_OPEN_ORDERS],
    // pub orders: [i128; MAX_PERP_OPEN_ORDERS],
    // pub client_order_ids: [u64; MAX_PERP_OPEN_ORDERS],

    // pub msrm_amount: u64,
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
