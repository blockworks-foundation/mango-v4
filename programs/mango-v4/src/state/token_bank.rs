use anchor_lang::prelude::*;
use fixed::types::I80F48;

#[account(zero_copy)]
#[derive(Default)]
pub struct TokenBank {
    /// native tokens deposited into or borrowed from this bank
    pub deposits: u64,
    pub borrows: u64,

    // todo: multi-leg interest
    // pub optimal_util: I80F48,
    // pub optimal_rate: I80F48,
    // pub max_rate: I80F48,

    /// the index used to scale the value of an IndexedPosition
    pub deposit_index: I80F48,
    pub borrow_index: I80F48,
}
