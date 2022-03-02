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

pub struct IndexedPositionInfo {
    pub active_index: usize,
    pub active_len: usize,
    pub raw_index: usize,
}

impl IndexedPositions {
    pub fn get_mut(&mut self, token_index: usize) -> Result<&mut IndexedPosition> {
        self.values
            .iter_mut()
            .find(|p| p.is_active_for_index(token_index))
            .ok_or_else(|| error!(MangoError::SomeError)) // TODO: not found error
    }

    // NOTE: If no position for that token was found, info will have the active_index
    // that position will have when activated, even though the position that is returned
    // is not yet active yet. Also active_len returns the number of active positions without
    // including the potential new position.
    // TODO: Avoid confusion by storing info about "is this position active?" explicitly?
    pub fn get_mut_or_create(
        &mut self,
        token_index: usize,
    ) -> Result<(&mut IndexedPosition, IndexedPositionInfo)> {
        let mut found = false;
        let mut info = IndexedPositionInfo {
            active_index: 0,
            active_len: 0,
            raw_index: usize::MAX,
        };
        for raw_index in 0..self.values.len() {
            let position = &self.values[raw_index];
            if position.is_active() {
                if position.token_index == token_index as TokenIndex {
                    found = true;
                    info.raw_index = raw_index;
                    info.active_index = info.active_len;
                }
                info.active_len += 1;
            } else if info.raw_index == usize::MAX {
                // Store the data for the first non-active entry
                info.raw_index = raw_index;
                info.active_index = info.active_len;
            }
        }

        if !found {
            if info.raw_index == usize::MAX {
                return Err(error!(MangoError::SomeError)); // full
            }
            self.values[info.raw_index] = IndexedPosition {
                indexed_value: I80F48::ZERO,
                token_index: token_index as TokenIndex,
            };
        }
        Ok((&mut self.values[info.raw_index], info))
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
    pub address_lookup_table_selection_size: u8,
    pub address_lookup_table_selection: [u8; 255],

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
