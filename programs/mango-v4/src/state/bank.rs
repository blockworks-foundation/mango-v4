use super::{TokenAccount, TokenIndex};
use crate::error::MangoError;
use crate::util::checked_math as cm;
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use static_assertions::const_assert_eq;
use std::mem::size_of;

pub const YEAR: I80F48 = I80F48!(31536000);

#[account(zero_copy)]
pub struct Bank {
    pub name: [u8; 16],

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

    pub last_updated: i64,
    pub util0: I80F48,
    pub rate0: I80F48,
    pub util1: I80F48,
    pub rate1: I80F48,
    pub max_rate: I80F48,

    // TODO: add ix/logic to regular send this to DAO
    pub collected_fees_native: I80F48,
    pub loan_origination_fee_rate: I80F48,
    pub loan_fee_rate: I80F48,

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

    pub reserved: [u8; 6],
}
const_assert_eq!(size_of::<Bank>(), 16 + 32 * 4 + 8 + 16 * 18 + 2 + 6);
const_assert_eq!(size_of::<Bank>() % 8, 0);

impl std::fmt::Debug for Bank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bank")
            .field("name", &self.name())
            .field("group", &self.group)
            .field("mint", &self.mint)
            .field("vault", &self.vault)
            .field("oracle", &self.oracle)
            .field("deposit_index", &self.deposit_index)
            .field("borrow_index", &self.borrow_index)
            .field("indexed_total_deposits", &self.indexed_total_deposits)
            .field("indexed_total_borrows", &self.indexed_total_borrows)
            .field("last_updated", &self.last_updated)
            .field("util0", &self.util0)
            .field("rate0", &self.rate0)
            .field("util1", &self.util1)
            .field("rate1", &self.rate1)
            .field("max_rate", &self.max_rate)
            .field("collected_fees_native", &self.collected_fees_native)
            .field("loan_origination_fee_rate", &self.loan_origination_fee_rate)
            .field("loan_fee_rate", &self.loan_fee_rate)
            .field("maint_asset_weight", &self.maint_asset_weight)
            .field("init_asset_weight", &self.init_asset_weight)
            .field("maint_liab_weight", &self.maint_liab_weight)
            .field("init_liab_weight", &self.init_liab_weight)
            .field("liquidation_fee", &self.liquidation_fee)
            .field("dust", &self.dust)
            .field("token_index", &self.token_index)
            .field("reserved", &self.reserved)
            .finish()
    }
}

impl Bank {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn native_total_borrows(&self) -> I80F48 {
        self.borrow_index * self.indexed_total_borrows
    }

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

    // collect loan origination fee for borrows
    pub fn charge_loan_origination_fee(
        &mut self,
        withdraw_amount: I80F48,
        position: I80F48,
    ) -> Result<I80F48> {
        require!(withdraw_amount.is_positive(), MangoError::SomeError);

        let mut loan_origination_fees = I80F48::ZERO;
        if withdraw_amount > position {
            let native_position_deposits_only = if position.is_negative() {
                I80F48::ZERO
            } else {
                position
            };
            let borrow = cm!(withdraw_amount - native_position_deposits_only);
            loan_origination_fees = cm!(self.loan_origination_fee_rate * borrow);
            self.collected_fees_native = cm!(self.collected_fees_native + loan_origination_fees);
        }
        Ok(loan_origination_fees)
    }

    // Borrows continously expose insurance fund to risk, collect fees from borrowers
    pub fn charge_loan_fee(&mut self, diff_ts: I80F48) {
        let native_total_borrows_old = self.native_total_borrows();
        self.indexed_total_borrows =
            cm!((self.indexed_total_borrows
                * (I80F48::ONE + self.loan_fee_rate * (diff_ts / YEAR))));
        self.collected_fees_native = cm!(
            self.collected_fees_native + self.native_total_borrows() - native_total_borrows_old
        );
    }

    // TODO: daffy: use optimal interest from oracle
    pub fn update_index(&mut self, now_ts: i64) -> Result<()> {
        let diff_ts = I80F48::from_num(now_ts - self.last_updated);
        self.last_updated = now_ts;

        self.charge_loan_fee(diff_ts);

        // Update index based on utilization
        let utilization = if self.native_total_deposits() == I80F48::ZERO {
            I80F48::ZERO
        } else {
            cm!(self.native_total_borrows() / self.native_total_deposits())
        };

        let interest_rate = self.compute_interest_rate(utilization);

        let borrow_interest: I80F48 = cm!(interest_rate * diff_ts);
        let deposit_interest = cm!(borrow_interest * utilization);

        if borrow_interest <= I80F48::ZERO || deposit_interest <= I80F48::ZERO {
            return Ok(());
        }

        self.borrow_index = cm!((self.borrow_index * borrow_interest) / YEAR + self.borrow_index);
        self.deposit_index =
            cm!((self.deposit_index * deposit_interest) / YEAR + self.deposit_index);

        Ok(())
    }

    /// returns the current interest rate in APR
    #[inline(always)]
    pub fn compute_interest_rate(&self, utilization: I80F48) -> I80F48 {
        Bank::interest_rate_curve_calculator(
            utilization,
            self.util0,
            self.rate0,
            self.util1,
            self.rate1,
            self.max_rate,
        )
    }

    /// calcualtor function that can be used to compute an interest
    /// rate based on the given parameters
    #[inline(always)]
    pub fn interest_rate_curve_calculator(
        utilization: I80F48,
        util0: I80F48,
        rate0: I80F48,
        util1: I80F48,
        rate1: I80F48,
        max_rate: I80F48,
    ) -> I80F48 {
        if utilization <= util0 {
            let slope = cm!(rate0 / util0);
            cm!(slope * utilization)
        } else if utilization <= util1 {
            let extra_util = cm!(utilization - util0);
            let slope = cm!((rate1 - rate0) / (util1 - util0));
            cm!(rate0 + slope * extra_util)
        } else {
            let extra_util = cm!(utilization - util1);
            let slope = cm!((max_rate - rate1) / (I80F48::ONE - util1));
            cm!(rate1 + slope * extra_util)
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
                    reserved: Default::default(),
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
