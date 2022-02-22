use anchor_lang::prelude::*;
use super::mango_group::*;
use fixed::types::I80F48;

const MAX_INDEXED_POSITIONS: usize = 32;

#[zero_copy]
pub struct IndexedPosition {
    // TODO: Why did we have deposits and borrows as two different values
    //       if only one of them was allowed to be != 0 at a time?
    // todo: maybe we want to split collateral and lending?
    // todo: see https://github.com/blockworks-foundation/mango-v4/issues/1
    // todo: how does ftx do this?
    /// The deposit_index (if positive) or borrow_index (if negative) scaled position
    value: I80F48,

    /// index into MangoGroup.tokens
    token_index: TokenIndex,
}
// TODO: static assert the size and alignment

impl IndexedPosition {
    pub fn is_active(&self) -> bool {
        // maybe want to reserve token_index == 0?
        self.value != I80F48::ZERO
    }
}

#[account(zero_copy)]
pub struct MangoAccount {
    pub mango_group: Pubkey,
    pub owner: Pubkey,

    // Alternative authority/signer of transactions for a mango account
    pub delegate: Pubkey,

    // pub in_margin_basket: [bool; MAX_PAIRS],
    // pub num_in_margin_basket: u8,

    pub indexed_position: [I80F48; MAX_INDEXED_POSITIONS],

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

    // pub info: [u8; INFO_LEN], // TODO: Info could be in a separate PDA?


    pub reserved: [u8; 5],
}
// TODO: static assert the size and alignment
