use anchor_lang::prelude::*;
use fixed::types::I80F48;

use super::{TokenAccount, TokenIndex};
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

    // a fraction of the price, like 0.05 for a 5% fee during liquidation
    //
    // Liquidation always involves two tokens, and the sum of the two configured fees is used.
    pub liquidation_fee: I80F48,

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
    ///
    /// native_amount must be >= 0
    /// fractional deposits can be relevant during liquidation, for example
    pub fn deposit(
        &mut self,
        position: &mut TokenAccount,
        mut native_amount: I80F48,
    ) -> Result<bool> {
        let native_position = position.native(self);

        if native_position.is_negative() {
            let new_native_position = cm!(native_position + native_amount);
            if !new_native_position.is_positive() {
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
    ///
    /// native_amount must be >= 0
    /// fractional withdraws can be relevant during liquidation, for example
    pub fn withdraw(
        &mut self,
        position: &mut TokenAccount,
        mut native_amount: I80F48,
    ) -> Result<bool> {
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

    pub fn change(&mut self, position: &mut TokenAccount, native_amount: I80F48) -> Result<bool> {
        if native_amount >= 0 {
            self.deposit(position, native_amount)
        } else {
            self.withdraw(position, -native_amount)
        }
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::Zeroable;

    use super::*;

    #[test]
    pub fn change() -> Result<()> {
        let epsilon = I80F48::from_bits(1);
        let cases = [
            (-10.1, 1),
            (-10.1, 10),
            (-10.1, 11),
            (-10.1, 50),
            (-2.0, 2),
            (-2.0, 3),
            (-0.1, 1),
            (0.0, 1),
            (0.1, 1),
            (10.1, -1),
            (10.1, -9),
            (10.1, -10),
            (10.1, -11),
            (1.0, -1),
            (0.1, -1),
            (0.0, -1),
            (-0.1, -1),
            (-1.1, -10),
        ];

        for is_in_use in [false, true] {
            for (start, change) in cases {
                println!(
                    "testing: in use: {}, start: {}, change: {}",
                    is_in_use, start, change
                );

                //
                // SETUP
                //

                let mut bank = Bank::zeroed();
                bank.deposit_index = I80F48::from_num(100.0);
                bank.borrow_index = I80F48::from_num(10.0);
                let indexed = |v: I80F48, b: &Bank| {
                    if v > 0 {
                        v / b.deposit_index
                    } else {
                        v / b.borrow_index
                    }
                };

                let mut account = TokenAccount {
                    indexed_value: I80F48::ZERO,
                    token_index: 0,
                    in_use_count: if is_in_use { 1 } else { 0 },
                };

                account.indexed_value = indexed(I80F48::from_num(start), &bank);
                if start >= 0.0 {
                    bank.indexed_total_deposits = account.indexed_value;
                } else {
                    bank.indexed_total_borrows = -account.indexed_value;
                }

                // get the rounded start value
                let start_native = account.native(&bank);

                //
                // TEST
                //

                let change = I80F48::from(change);
                let is_active = bank.change(&mut account, change)?;

                let mut expected_native = start_native + change;
                if expected_native >= 0.0 && expected_native < 1.0 && !is_in_use {
                    assert!(!is_active);
                    assert_eq!(bank.dust, expected_native);
                    expected_native = I80F48::ZERO;
                } else {
                    assert!(is_active);
                    assert_eq!(bank.dust, I80F48::ZERO);
                }
                let expected_indexed = indexed(expected_native, &bank);

                // at most one epsilon error in the resulting indexed value
                assert!((account.indexed_value - expected_indexed).abs() <= epsilon);

                if account.indexed_value.is_positive() {
                    assert_eq!(bank.indexed_total_deposits, account.indexed_value);
                    assert_eq!(bank.indexed_total_borrows, I80F48::ZERO);
                } else {
                    assert_eq!(bank.indexed_total_deposits, I80F48::ZERO);
                    assert_eq!(bank.indexed_total_borrows, -account.indexed_value);
                }
            }
        }
        Ok(())
    }
}
