use anchor_lang::prelude::*;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;

use super::IndexedPosition;

const INDEX_START: I80F48 = I80F48!(1_000_000);

#[account(zero_copy)]
pub struct TokenBank {
    /// the index used to scale the value of an IndexedPosition
    /// TODO: should always be >= 0, add checks?
    pub deposit_index: I80F48,
    pub borrow_index: I80F48,

    /// total deposits/borrows, for utilization
    pub indexed_total_deposits: I80F48,
    pub indexed_total_borrows: I80F48,
    // todo: multi-leg interest
    // pub optimal_util: I80F48,
    // pub optimal_rate: I80F48,
    // pub max_rate: I80F48,
}

impl TokenBank {
    pub fn initialize(&mut self) {
        self.deposit_index = INDEX_START;
        self.borrow_index = INDEX_START;
    }

    pub fn native_total_deposits(&self) -> I80F48 {
        self.deposit_index * self.indexed_total_deposits
    }

    pub fn deposit(&mut self, position: &mut IndexedPosition, native_amount: u64) {
        let mut native_amount = I80F48::from_num(native_amount);
        let native_position = position.native(self);

        if native_position.is_negative() {
            if -native_position >= native_amount {
                // pay back borrows only
                let indexed_change = native_amount / self.borrow_index;
                self.indexed_total_borrows -= indexed_change;
                position.indexed_value += indexed_change;
                return;
            }

            // pay back all borrows first
            self.indexed_total_borrows += position.indexed_value; // position.value is negative
            position.indexed_value = I80F48::ZERO;
            native_amount += native_position;
        }

        // add to deposits
        let indexed_change = native_amount / self.deposit_index;
        self.indexed_total_deposits += indexed_change;
        position.indexed_value += indexed_change;
    }

    pub fn withdraw(&mut self, position: &mut IndexedPosition, native_amount: u64) {
        let mut native_amount = I80F48::from_num(native_amount);
        let native_position = position.native(self);

        if native_position.is_positive() {
            if native_position >= native_amount {
                // withdraw deposits only
                let indexed_change = native_amount / self.deposit_index;
                self.indexed_total_deposits -= indexed_change;
                position.indexed_value -= indexed_change;
                return;
            }

            // withdraw all deposits first
            self.indexed_total_deposits -= position.indexed_value;
            position.indexed_value = I80F48::ZERO;
            native_amount -= native_position;
        }

        // add to borrows
        let indexed_change = native_amount / self.borrow_index;
        self.indexed_total_borrows += indexed_change;
        position.indexed_value -= indexed_change;
    }
}
