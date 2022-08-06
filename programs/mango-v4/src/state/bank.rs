use super::{OracleConfig, TokenIndex, TokenPosition};
use crate::util::checked_math as cm;
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use static_assertions::const_assert_eq;

use std::mem::size_of;

pub const HOUR: i64 = 3600;
pub const DAY: i64 = 86400;
pub const DAY_I80F48: I80F48 = I80F48!(86400);
pub const YEAR_I80F48: I80F48 = I80F48!(31536000);
pub const MINIMUM_MAX_RATE: I80F48 = I80F48!(0.5);

#[account(zero_copy)]
pub struct Bank {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,

    pub name: [u8; 16],

    pub mint: Pubkey,
    pub vault: Pubkey,
    pub oracle: Pubkey,

    pub oracle_config: OracleConfig,

    /// the index used to scale the value of an IndexedPosition
    /// TODO: should always be >= 0, add checks?
    pub deposit_index: I80F48,
    pub borrow_index: I80F48,

    /// total deposits/borrows, only updated during UpdateIndexAndRate
    /// TODO: These values could be dropped from the bank, they're written in UpdateIndexAndRate
    ///       and never read.
    pub cached_indexed_total_deposits: I80F48,
    pub cached_indexed_total_borrows: I80F48,

    /// deposits/borrows for this bank
    ///
    /// Note that these may become negative. It's perfectly fine for users to borrow one one bank
    /// (increasing indexed_borrows there) and paying back on another (possibly decreasing indexed_borrows
    /// below zero).
    ///
    /// The vault amount is not deducable from these values.
    ///
    /// These become meaningful when summed over all banks (like in update_index_and_rate).
    pub indexed_deposits: I80F48,
    pub indexed_borrows: I80F48,

    pub index_last_updated: i64,
    pub bank_rate_last_updated: i64,

    pub avg_utilization: I80F48,

    pub adjustment_factor: I80F48,
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

    pub flash_loan_token_account_initial: u64,
    pub flash_loan_approved_amount: u64,

    // Index into TokenInfo on the group
    pub token_index: TokenIndex,

    pub bump: u8,

    pub mint_decimals: u8,

    pub bank_num: u32,

    pub reserved: [u8; 256],
}
const_assert_eq!(
    size_of::<Bank>(),
    32 + 16 + 32 * 3 + 16 + 16 * 6 + 8 * 2 + 16 * 16 + 8 * 2 + 2 + 1 + 1 + 4 + 256
);
const_assert_eq!(size_of::<Bank>() % 8, 0);

impl std::fmt::Debug for Bank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bank")
            .field("name", &self.name())
            .field("group", &self.group)
            .field("mint", &self.mint)
            .field("vault", &self.vault)
            .field("oracle", &self.oracle)
            .field("oracle_config", &self.oracle_config)
            .field("deposit_index", &self.deposit_index)
            .field("borrow_index", &self.borrow_index)
            .field(
                "cached_indexed_total_deposits",
                &self.cached_indexed_total_deposits,
            )
            .field(
                "cached_indexed_total_borrows",
                &self.cached_indexed_total_borrows,
            )
            .field("indexed_deposits", &self.indexed_deposits)
            .field("indexed_borrows", &self.indexed_borrows)
            .field("index_last_updated", &self.index_last_updated)
            .field("bank_rate_last_updated", &self.bank_rate_last_updated)
            .field("avg_utilization", &self.avg_utilization)
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
            .field(
                "flash_loan_approved_amount",
                &self.flash_loan_approved_amount,
            )
            .field(
                "flash_loan_token_account_initial",
                &self.flash_loan_token_account_initial,
            )
            .field("reserved", &self.reserved)
            .finish()
    }
}

impl Bank {
    pub fn from_existing_bank(existing_bank: &Bank, vault: Pubkey, bank_num: u32) -> Self {
        Self {
            name: existing_bank.name,
            group: existing_bank.group,
            mint: existing_bank.mint,
            vault,
            oracle: existing_bank.oracle,
            oracle_config: existing_bank.oracle_config,
            deposit_index: existing_bank.deposit_index,
            borrow_index: existing_bank.borrow_index,
            cached_indexed_total_deposits: existing_bank.cached_indexed_total_deposits,
            cached_indexed_total_borrows: existing_bank.cached_indexed_total_borrows,
            indexed_deposits: I80F48::ZERO,
            indexed_borrows: I80F48::ZERO,
            index_last_updated: existing_bank.index_last_updated,
            bank_rate_last_updated: existing_bank.bank_rate_last_updated,
            avg_utilization: existing_bank.avg_utilization,
            adjustment_factor: existing_bank.adjustment_factor,
            util0: existing_bank.util0,
            rate0: existing_bank.rate0,
            util1: existing_bank.util1,
            rate1: existing_bank.rate1,
            max_rate: existing_bank.max_rate,
            collected_fees_native: existing_bank.collected_fees_native,
            loan_origination_fee_rate: existing_bank.loan_origination_fee_rate,
            loan_fee_rate: existing_bank.loan_fee_rate,
            maint_asset_weight: existing_bank.maint_asset_weight,
            init_asset_weight: existing_bank.init_asset_weight,
            maint_liab_weight: existing_bank.maint_liab_weight,
            init_liab_weight: existing_bank.init_liab_weight,
            liquidation_fee: existing_bank.liquidation_fee,
            dust: I80F48::ZERO,
            flash_loan_approved_amount: 0,
            flash_loan_token_account_initial: u64::MAX,
            token_index: existing_bank.token_index,
            bump: existing_bank.bump,
            mint_decimals: existing_bank.mint_decimals,
            reserved: [0; 256],
            bank_num,
        }
    }

    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn native_borrows(&self) -> I80F48 {
        self.borrow_index * self.indexed_borrows
    }

    pub fn native_deposits(&self) -> I80F48 {
        self.deposit_index * self.indexed_deposits
    }

    /// Returns whether the position is active
    ///
    /// native_amount must be >= 0
    /// fractional deposits can be relevant during liquidation, for example
    pub fn deposit(
        &mut self,
        position: &mut TokenPosition,
        mut native_amount: I80F48,
    ) -> Result<bool> {
        require_gte!(native_amount, 0);
        let native_position = position.native(self);

        // Adding DELTA to amount/index helps because (amount/index)*index <= amount, but
        // we want to ensure that users can withdraw the same amount they have deposited, so
        // (amount/index + delta)*index >= amount is a better guarantee.
        // Additionally, we require that we don't adjust values if
        // (native / index) * index == native, because we sometimes call this function with
        // values that are products of index.
        let div_rounding_up = |native: I80F48, index: I80F48| {
            let indexed = cm!(native / index);
            if cm!(indexed * index) < native {
                cm!(indexed + I80F48::DELTA)
            } else {
                indexed
            }
        };

        if native_position.is_negative() {
            let new_native_position = cm!(native_position + native_amount);
            let indexed_change = div_rounding_up(native_amount, self.borrow_index);
            // this is only correct if it's not positive, because it scales the whole amount by borrow_index
            let new_indexed_value = cm!(position.indexed_position + indexed_change);
            if new_indexed_value.is_negative() {
                // pay back borrows only, leaving a negative position
                self.indexed_borrows = cm!(self.indexed_borrows - indexed_change);
                position.indexed_position = cm!(position.indexed_position + indexed_change);
                return Ok(true);
            } else if new_native_position < I80F48::ONE && !position.is_in_use() {
                // if there's less than one token deposited, zero the position
                self.dust = cm!(self.dust + new_native_position);
                self.indexed_borrows = cm!(self.indexed_borrows + position.indexed_position);
                position.indexed_position = I80F48::ZERO;
                return Ok(false);
            }

            // pay back all borrows
            self.indexed_borrows = cm!(self.indexed_borrows + position.indexed_position); // position.value is negative
            position.indexed_position = I80F48::ZERO;
            // deposit the rest
            native_amount = cm!(native_amount + native_position);
        }

        // add to deposits
        let indexed_change = div_rounding_up(native_amount, self.deposit_index);
        self.indexed_deposits = cm!(self.indexed_deposits + indexed_change);
        position.indexed_position = cm!(position.indexed_position + indexed_change);

        Ok(true)
    }

    /// Returns whether the position is active after withdrawing from a position
    /// without applying the loan origination fee.
    ///
    /// native_amount must be >= 0
    /// fractional withdraws can be relevant during liquidation, for example
    pub fn withdraw_without_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
    ) -> Result<bool> {
        self.withdraw_internal(position, native_amount, false)
    }

    /// Returns whether the position is active after withdrawing from a position
    /// while applying the loan origination fee if a borrow is created.
    ///
    /// native_amount must be >= 0
    /// fractional withdraws can be relevant during liquidation, for example
    pub fn withdraw_with_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
    ) -> Result<bool> {
        self.withdraw_internal(position, native_amount, true)
    }

    fn withdraw_internal(
        &mut self,
        position: &mut TokenPosition,
        mut native_amount: I80F48,
        with_loan_origination_fee: bool,
    ) -> Result<bool> {
        require_gte!(native_amount, 0);
        let native_position = position.native(self);

        if native_position.is_positive() {
            let new_native_position = cm!(native_position - native_amount);
            if !new_native_position.is_negative() {
                // withdraw deposits only
                if new_native_position < I80F48::ONE && !position.is_in_use() {
                    // zero the account collecting the leftovers in `dust`
                    self.dust = cm!(self.dust + new_native_position);
                    self.indexed_deposits = cm!(self.indexed_deposits - position.indexed_position);
                    position.indexed_position = I80F48::ZERO;
                    return Ok(false);
                } else {
                    // withdraw some deposits leaving a positive balance
                    let indexed_change = cm!(native_amount / self.deposit_index);
                    self.indexed_deposits = cm!(self.indexed_deposits - indexed_change);
                    position.indexed_position = cm!(position.indexed_position - indexed_change);
                    return Ok(true);
                }
            }

            // withdraw all deposits
            self.indexed_deposits = cm!(self.indexed_deposits - position.indexed_position);
            position.indexed_position = I80F48::ZERO;
            // borrow the rest
            native_amount = -new_native_position;
        }

        if with_loan_origination_fee {
            let loan_origination_fee = cm!(self.loan_origination_fee_rate * native_amount);
            self.collected_fees_native = cm!(self.collected_fees_native + loan_origination_fee);
            native_amount = cm!(native_amount + loan_origination_fee);
        }

        // add to borrows
        let indexed_change = cm!(native_amount / self.borrow_index);
        self.indexed_borrows = cm!(self.indexed_borrows + indexed_change);
        position.indexed_position = cm!(position.indexed_position - indexed_change);

        Ok(true)
    }

    // withdraw the loan origination fee for a borrow that happenend earlier
    pub fn withdraw_loan_origination_fee(
        &mut self,
        position: &mut TokenPosition,
        already_borrowed_native_amount: I80F48,
    ) -> Result<bool> {
        let loan_origination_fee =
            cm!(self.loan_origination_fee_rate * already_borrowed_native_amount);
        self.collected_fees_native = cm!(self.collected_fees_native + loan_origination_fee);

        self.withdraw_internal(position, loan_origination_fee, false)
    }

    /// Change a position without applying the loan origination fee
    pub fn change_without_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
    ) -> Result<bool> {
        if native_amount >= 0 {
            self.deposit(position, native_amount)
        } else {
            self.withdraw_without_fee(position, -native_amount)
        }
    }

    /// Change a position, while taking the loan origination fee into account
    pub fn change_with_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
    ) -> Result<bool> {
        if native_amount >= 0 {
            self.deposit(position, native_amount)
        } else {
            self.withdraw_with_fee(position, -native_amount)
        }
    }

    // Borrows continously expose insurance fund to risk, collect fees from borrowers
    pub fn charge_loan_fee(&mut self, diff_ts: I80F48) {
        let native_borrows_old = self.native_borrows();
        self.indexed_borrows =
            cm!((self.indexed_borrows
                * (I80F48::ONE + self.loan_fee_rate * (diff_ts / YEAR_I80F48))));
        self.collected_fees_native =
            cm!(self.collected_fees_native + self.native_borrows() - native_borrows_old);
    }

    pub fn compute_index(
        &self,
        indexed_total_deposits: I80F48,
        indexed_total_borrows: I80F48,
        diff_ts: I80F48,
    ) -> Result<(I80F48, I80F48)> {
        // compute index based on utilization
        let native_total_deposits = cm!(self.deposit_index * indexed_total_deposits);
        let native_total_borrows = cm!(self.borrow_index * indexed_total_borrows);

        let instantaneous_utilization = if native_total_deposits == I80F48::ZERO {
            I80F48::ZERO
        } else {
            cm!(native_total_borrows / native_total_deposits)
        };

        let borrow_interest_rate = self.compute_interest_rate(instantaneous_utilization);

        let borrow_interest: I80F48 = cm!(borrow_interest_rate * diff_ts);
        let deposit_interest = cm!(borrow_interest * instantaneous_utilization);

        // msg!("utilization {}", utilization);
        // msg!("interest_rate {}", interest_rate);
        // msg!("borrow_interest {}", borrow_interest);
        // msg!("deposit_interest {}", deposit_interest);

        if borrow_interest <= I80F48::ZERO || deposit_interest <= I80F48::ZERO {
            return Ok((self.deposit_index, self.borrow_index));
        }

        let borrow_index =
            cm!((self.borrow_index * borrow_interest) / YEAR_I80F48 + self.borrow_index);
        let deposit_index =
            cm!((self.deposit_index * deposit_interest) / YEAR_I80F48 + self.deposit_index);

        Ok((deposit_index, borrow_index))
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

    // compute new avg utilization
    pub fn compute_new_avg_utilization(
        &self,
        indexed_total_deposits: I80F48,
        indexed_total_borrows: I80F48,
        now_ts: I80F48,
    ) -> I80F48 {
        if now_ts == I80F48::ZERO {
            return I80F48::ZERO;
        }

        let native_total_deposits = self.deposit_index * indexed_total_deposits;
        let native_total_borrows = self.borrow_index * indexed_total_borrows;
        let instantaneous_utilization = if native_total_deposits == I80F48::ZERO {
            I80F48::ZERO
        } else {
            cm!(native_total_borrows / native_total_deposits)
        };

        // combine old and new with relevant factors to form new avg_utilization
        // scaling factor for previous avg_utilization is old_ts/new_ts
        // scaling factor for instantaneous utilization is (new_ts - old_ts) / new_ts
        let bank_rate_last_updated_i80f48 = I80F48::from_num(self.bank_rate_last_updated);
        (self.avg_utilization * bank_rate_last_updated_i80f48
            + instantaneous_utilization * (now_ts - bank_rate_last_updated_i80f48))
            / now_ts
    }

    // computes new optimal rates and max rate
    pub fn compute_rates(&self) -> (I80F48, I80F48, I80F48) {
        // interest rate legs 2 and 3 are seen as punitive legs, encouraging utilization to move towards optimal utilization
        // lets choose util0 as optimal utilization and 0 to utli0 as the leg where we want the utlization to preferably be
        let optimal_util = self.util0;
        // use avg_utilization and not instantaneous_utilization so that rates cannot be manupulated easily
        let util_diff = self.avg_utilization - optimal_util;
        // move rates up when utilization is above optimal utilization, and vice versa
        let adjustment = I80F48::ONE + self.adjustment_factor * util_diff;

        // 1. irrespective of which leg current utilization is in, update all rates
        // 2. only update rates as long as new adjusted rates are above MINIMUM_MAX_RATE,
        //  since we don't want to fall to such low rates that it would take a long time to
        //  recover to high rates if utilization suddently increases to a high value
        if cm!(self.max_rate * adjustment) > MINIMUM_MAX_RATE {
            (
                cm!(self.rate0 * adjustment),
                cm!(self.rate1 * adjustment),
                cm!(self.max_rate * adjustment),
            )
        } else {
            (self.rate0, self.rate1, self.max_rate)
        }
    }
}

#[macro_export]
macro_rules! bank_seeds {
    ( $bank:expr ) => {
        &[
            $bank.group.as_ref(),
            b"Bank".as_ref(),
            $bank.token_index.to_le_bytes(),
            &bank.bank_num.to_le_bytes(),
            &[$bank.bump],
        ]
    };
}

pub use bank_seeds;

#[cfg(test)]
mod tests {
    use bytemuck::Zeroable;
    use std::cmp::min;

    use super::*;

    #[test]
    pub fn change() -> Result<()> {
        let epsilon = I80F48::from_bits(1);
        let cases = [
            (-10.1, 1),
            (-10.1, 10),
            (-10.1, 11),
            (-10.1, 50),
            (-10.0, 10),
            (-10.0, 11),
            (-2.0, 2),
            (-2.0, 3),
            (-0.1, 1),
            (0.0, 1),
            (0.1, 1),
            (10.1, -1),
            (10.1, -9),
            (10.1, -10),
            (10.1, -11),
            (10.0, -10),
            (10.0, -9),
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
                bank.loan_origination_fee_rate = I80F48::from_num(0.1);
                let indexed = |v: I80F48, b: &Bank| {
                    if v > 0 {
                        v / b.deposit_index
                    } else {
                        v / b.borrow_index
                    }
                };

                let mut account = TokenPosition {
                    indexed_position: I80F48::ZERO,
                    token_index: 0,
                    in_use_count: if is_in_use { 1 } else { 0 },
                    padding: Default::default(),
                    reserved: [0; 40],
                };

                account.indexed_position = indexed(I80F48::from_num(start), &bank);
                if start >= 0.0 {
                    bank.indexed_deposits = account.indexed_position;
                } else {
                    bank.indexed_borrows = -account.indexed_position;
                }

                // get the rounded start value
                let start_native = account.native(&bank);

                //
                // TEST
                //

                let change = I80F48::from(change);
                let is_active = bank.change_with_fee(&mut account, change)?;

                let mut expected_native = start_native + change;
                if expected_native >= 0.0 && expected_native < 1.0 && !is_in_use {
                    assert!(!is_active);
                    assert_eq!(bank.dust, expected_native);
                    expected_native = I80F48::ZERO;
                } else {
                    assert!(is_active);
                    assert_eq!(bank.dust, I80F48::ZERO);
                }
                if change < 0 && expected_native < 0 {
                    let new_borrow = -(expected_native - min(start_native, I80F48::ZERO));
                    expected_native -= new_borrow * bank.loan_origination_fee_rate;
                }
                let expected_indexed = indexed(expected_native, &bank);

                // at most one epsilon error in the resulting indexed value
                assert!((account.indexed_position - expected_indexed).abs() <= epsilon);

                if account.indexed_position.is_positive() {
                    assert_eq!(bank.indexed_deposits, account.indexed_position);
                    assert_eq!(bank.indexed_borrows, I80F48::ZERO);
                } else {
                    assert_eq!(bank.indexed_deposits, I80F48::ZERO);
                    assert_eq!(bank.indexed_borrows, -account.indexed_position);
                }
            }
        }
        Ok(())
    }

    #[test]
    fn test_compute_new_avg_utilization() {
        let mut bank = Bank::zeroed();
        bank.deposit_index = I80F48::from_num(1.0);
        bank.borrow_index = I80F48::from_num(1.0);
        bank.bank_rate_last_updated = 0;

        let compute_new_avg_utilization_runner =
            |bank: &mut Bank, utilization: I80F48, now_ts: i64| {
                bank.avg_utilization = bank.compute_new_avg_utilization(
                    I80F48::ONE,
                    utilization,
                    I80F48::from_num(now_ts),
                );
                bank.bank_rate_last_updated = now_ts;
            };

        compute_new_avg_utilization_runner(&mut bank, I80F48::ZERO, 0);
        assert_eq!(bank.avg_utilization, I80F48::ZERO);

        compute_new_avg_utilization_runner(&mut bank, I80F48::from_num(0.5), 10);
        assert!((bank.avg_utilization - I80F48::from_num(0.5)).abs() < 0.0001);

        compute_new_avg_utilization_runner(&mut bank, I80F48::from_num(0.8), 15);
        assert!((bank.avg_utilization - I80F48::from_num(0.6)).abs() < 0.0001);

        compute_new_avg_utilization_runner(&mut bank, I80F48::ONE, 20);
        assert!((bank.avg_utilization - I80F48::from_num(0.7)).abs() < 0.0001);
    }
}
