use anchor_lang::prelude::*;
use fixed::types::I80F48;

use super::{IndexedPosition, TokenIndex};
use crate::util::checked_math as cm;

#[account(zero_copy)]
pub struct Bank {
    pub group: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub oracle: Pubkey,

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

    // This is a _lot_ of bytes (64) - seems unnecessary
    // (could maybe store them in one byte each, as an informal U1F7?
    // that could store values between 0-2 and converting to I80F48 would be a cheap expand+shift)
    pub maint_asset_weight: I80F48,
    pub init_asset_weight: I80F48,
    pub maint_liab_weight: I80F48,
    pub init_liab_weight: I80F48,

    // Collection of all fractions-of-native-tokens that got rounded away
    pub dust: I80F48,

    // Index into TokenInfo on the group
    pub token_index: TokenIndex,
}

impl Bank {
    pub fn native_total_deposits(&self) -> I80F48 {
        self.deposit_index * self.indexed_total_deposits
    }

    /// Returns whether the position is active
    pub fn deposit(&mut self, position: &mut IndexedPosition, native_amount: u64) -> Result<bool> {
        let mut native_amount = I80F48::from_num(native_amount);
        let native_position = position.native(self);

        if native_position.is_negative() {
            let new_native_position = cm!(native_position + native_amount);
            if new_native_position.is_negative() {
                // pay back borrows only, leaving a negative position
                let indexed_change = cm!(native_amount / self.borrow_index + I80F48::DELTA);
                self.indexed_total_borrows = cm!(self.indexed_total_borrows - indexed_change);
                position.indexed_value = cm!(position.indexed_value + indexed_change);
                return Ok(true);
            } else if new_native_position < I80F48::ONE && !position.is_in_use() {
                // if there's less than one token deposited, zero the position
                self.dust = cm!(self.dust + new_native_position);
                self.indexed_total_borrows =
                    cm!(self.indexed_total_borrows + position.indexed_value);
                position.indexed_value = I80F48::ZERO;
                return Ok(false);
            }

            // pay back all borrows
            self.indexed_total_borrows = cm!(self.indexed_total_borrows + position.indexed_value); // position.value is negative
            position.indexed_value = I80F48::ZERO;
            // deposit the rest
            native_amount = cm!(native_amount + native_position);
        }

        // add to deposits
        // Adding DELTA to amount/index helps because (amount/index)*index <= amount, but
        // we want to ensure that users can withdraw the same amount they have deposited, so
        // (amount/index + delta)*index >= amount is a better guarantee.
        let indexed_change = cm!(native_amount / self.deposit_index + I80F48::DELTA);
        self.indexed_total_deposits = cm!(self.indexed_total_deposits + indexed_change);
        position.indexed_value = cm!(position.indexed_value + indexed_change);

        Ok(true)
    }

    /// Returns whether the position is active
    pub fn withdraw(&mut self, position: &mut IndexedPosition, native_amount: u64) -> Result<bool> {
        let mut native_amount = I80F48::from_num(native_amount);
        let native_position = position.native(self);

        if native_position.is_positive() {
            let new_native_position = cm!(native_position - native_amount);
            if !new_native_position.is_negative() {
                // withdraw deposits only
                if new_native_position < I80F48::ONE && !position.is_in_use() {
                    // zero the account collecting the leftovers in `dust`
                    self.dust = cm!(self.dust + new_native_position);
                    self.indexed_total_deposits =
                        cm!(self.indexed_total_deposits - position.indexed_value);
                    position.indexed_value = I80F48::ZERO;
                    return Ok(false);
                } else {
                    // withdraw some deposits leaving a positive balance
                    let indexed_change = cm!(native_amount / self.deposit_index);
                    self.indexed_total_deposits = cm!(self.indexed_total_deposits - indexed_change);
                    position.indexed_value = cm!(position.indexed_value - indexed_change);
                    return Ok(true);
                }
            }

            // withdraw all deposits
            self.indexed_total_deposits = cm!(self.indexed_total_deposits - position.indexed_value);
            position.indexed_value = I80F48::ZERO;
            // borrow the rest
            native_amount = -new_native_position;
        }

        // add to borrows
        let indexed_change = cm!(native_amount / self.borrow_index);
        self.indexed_total_borrows = cm!(self.indexed_total_borrows + indexed_change);
        position.indexed_value = cm!(position.indexed_value - indexed_change);

        Ok(true)
    }

    pub fn change(&mut self, position: &mut IndexedPosition, native_amount: i64) -> Result<bool> {
        if native_amount >= 0 {
            self.deposit(position, native_amount as u64)
        } else {
            self.withdraw(position, (-native_amount) as u64)
        }
    }
}
