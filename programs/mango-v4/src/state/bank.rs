use super::{OracleConfig, TokenIndex, TokenPosition};
use crate::accounts_zerocopy::KeyedAccountReader;
use crate::error::*;
use crate::i80f48::ClampToInt;
use crate::state::{oracle, StablePriceModel};
use crate::util;

use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use derivative::Derivative;
use fixed::types::I80F48;
use static_assertions::const_assert_eq;

use std::mem::size_of;

pub const HOUR: i64 = 3600;
pub const DAY: i64 = 86400;
pub const DAY_I80F48: I80F48 = I80F48::from_bits(86_400 * I80F48::ONE.to_bits());
pub const YEAR_I80F48: I80F48 = I80F48::from_bits(31_536_000 * I80F48::ONE.to_bits());
pub const MINIMUM_MAX_RATE: I80F48 = I80F48::from_bits(I80F48::ONE.to_bits() / 2);

#[derive(Derivative)]
#[derivative(Debug)]
#[account(zero_copy(safe_bytemuck_derives))]
pub struct Bank {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,

    #[derivative(Debug(format_with = "util::format_zero_terminated_utf8_bytes"))]
    pub name: [u8; 16],

    pub mint: Pubkey,
    pub vault: Pubkey,
    pub oracle: Pubkey,

    pub oracle_config: OracleConfig,
    pub stable_price_model: StablePriceModel,

    /// the index used to scale the value of an IndexedPosition
    /// TODO: should always be >= 0, add checks?
    pub deposit_index: I80F48,
    pub borrow_index: I80F48,

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

    pub index_last_updated: u64,
    pub bank_rate_last_updated: u64,

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

    /// Min fraction of deposits that must remain in the vault when borrowing.
    pub min_vault_to_deposits_ratio: f64,

    /// Size in seconds of a net borrows window
    pub net_borrow_limit_window_size_ts: u64,
    /// Timestamp at which the last net borrows window started
    pub last_net_borrows_window_start_ts: u64,
    /// Net borrow limit per window in quote native; set to -1 to disable.
    pub net_borrow_limit_per_window_quote: i64,
    /// Sum of all deposits and borrows in the last window, in native units.
    pub net_borrows_in_window: i64,

    /// Soft borrow limit in native quote
    ///
    /// Once the borrows on the bank exceed this quote value, init_liab_weight is scaled up.
    /// Set to f64::MAX to disable.
    ///
    /// See scaled_init_liab_weight().
    pub borrow_weight_scale_start_quote: f64,

    /// Limit for collateral of deposits in native quote
    ///
    /// Once the deposits in the bank exceed this quote value, init_asset_weight is scaled
    /// down to keep the total collateral value constant.
    /// Set to f64::MAX to disable.
    ///
    /// See scaled_init_asset_weight().
    pub deposit_weight_scale_start_quote: f64,

    pub reduce_only: u8,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 2119],
}
const_assert_eq!(
    size_of::<Bank>(),
    32 + 16
        + 32 * 3
        + 96
        + 288
        + 16 * 2
        + 16 * 2
        + 8 * 2
        + 16
        + 16 * 6
        + 16 * 3
        + 16 * 4
        + 16
        + 16
        + 8
        + 8
        + 2
        + 1
        + 1
        + 4
        + 8
        + 8 * 4
        + 8
        + 8
        + 1
        + 2119
);
const_assert_eq!(size_of::<Bank>(), 3064);
const_assert_eq!(size_of::<Bank>() % 8, 0);

impl Bank {
    pub fn from_existing_bank(
        existing_bank: &Bank,
        vault: Pubkey,
        bank_num: u32,
        bump: u8,
    ) -> Self {
        Self {
            // values that must be reset/changed
            vault,
            indexed_deposits: I80F48::ZERO,
            indexed_borrows: I80F48::ZERO,
            collected_fees_native: I80F48::ZERO,
            dust: I80F48::ZERO,
            flash_loan_approved_amount: 0,
            flash_loan_token_account_initial: u64::MAX,
            bump,
            bank_num,

            // values that can be copied
            // these are listed explicitly, so someone must make the decision when a
            // new field is added!
            name: existing_bank.name,
            group: existing_bank.group,
            mint: existing_bank.mint,
            oracle: existing_bank.oracle,
            deposit_index: existing_bank.deposit_index,
            borrow_index: existing_bank.borrow_index,
            index_last_updated: existing_bank.index_last_updated,
            bank_rate_last_updated: existing_bank.bank_rate_last_updated,
            avg_utilization: existing_bank.avg_utilization,
            adjustment_factor: existing_bank.adjustment_factor,
            util0: existing_bank.util0,
            rate0: existing_bank.rate0,
            util1: existing_bank.util1,
            rate1: existing_bank.rate1,
            max_rate: existing_bank.max_rate,
            loan_origination_fee_rate: existing_bank.loan_origination_fee_rate,
            loan_fee_rate: existing_bank.loan_fee_rate,
            maint_asset_weight: existing_bank.maint_asset_weight,
            init_asset_weight: existing_bank.init_asset_weight,
            maint_liab_weight: existing_bank.maint_liab_weight,
            init_liab_weight: existing_bank.init_liab_weight,
            liquidation_fee: existing_bank.liquidation_fee,
            token_index: existing_bank.token_index,
            mint_decimals: existing_bank.mint_decimals,
            oracle_config: existing_bank.oracle_config,
            stable_price_model: StablePriceModel::default(),
            min_vault_to_deposits_ratio: existing_bank.min_vault_to_deposits_ratio,
            net_borrow_limit_per_window_quote: existing_bank.net_borrow_limit_per_window_quote,
            net_borrow_limit_window_size_ts: existing_bank.net_borrow_limit_window_size_ts,
            last_net_borrows_window_start_ts: existing_bank.last_net_borrows_window_start_ts,
            net_borrows_in_window: 0,
            borrow_weight_scale_start_quote: f64::MAX,
            deposit_weight_scale_start_quote: f64::MAX,
            reduce_only: 0,
            reserved: [0; 2119],
        }
    }

    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn is_reduce_only(&self) -> bool {
        self.reduce_only == 1
    }

    #[inline(always)]
    pub fn native_borrows(&self) -> I80F48 {
        self.borrow_index * self.indexed_borrows
    }

    #[inline(always)]
    pub fn native_deposits(&self) -> I80F48 {
        self.deposit_index * self.indexed_deposits
    }

    /// Prevent borrowing away the full bank vault.
    /// Keep some in reserve to satisfy non-borrow withdraws.
    pub fn enforce_min_vault_to_deposits_ratio(&self, vault_ai: &AccountInfo) -> Result<()> {
        require_keys_eq!(self.vault, vault_ai.key());

        let vault = Account::<TokenAccount>::try_from(vault_ai)?;
        let vault_amount = vault.amount as f64;

        let bank_native_deposits = self.native_deposits();
        if bank_native_deposits != I80F48::ZERO {
            let bank_native_deposits: f64 = bank_native_deposits.to_num();
            if vault_amount < self.min_vault_to_deposits_ratio * bank_native_deposits {
                return err!(MangoError::BankBorrowLimitReached).with_context(|| {
                format!(
                    "vault_amount ({:?}) below min_vault_to_deposits_ratio * bank_native_deposits ({:?})",
                    vault_amount, self.min_vault_to_deposits_ratio * bank_native_deposits,
                )
            });
            }
        }
        Ok(())
    }

    /// Deposits `native_amount`.
    ///
    /// If the token position ends up positive but below one native token and this token
    /// position isn't marked as in-use, the token balance will be dusted, the position
    /// will be set to zero and this function returns Ok(false).
    ///
    /// native_amount must be >= 0
    /// fractional deposits can be relevant during liquidation, for example
    pub fn deposit(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
    ) -> Result<bool> {
        self.deposit_internal_wrapper(position, native_amount, !position.is_in_use(), now_ts)
    }

    /// Like `deposit()`, but allows dusting of in-use accounts.
    ///
    /// Returns Ok(false) if the position was dusted and was not in-use.
    pub fn deposit_with_dusting(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
    ) -> Result<bool> {
        self.deposit_internal_wrapper(position, native_amount, true, now_ts)
            .map(|not_dusted| not_dusted || position.is_in_use())
    }

    pub fn deposit_internal_wrapper(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        allow_dusting: bool,
        now_ts: u64,
    ) -> Result<bool> {
        let opening_indexed_position = position.indexed_position;
        let result = self.deposit_internal(position, native_amount, allow_dusting, now_ts)?;
        self.update_cumulative_interest(position, opening_indexed_position);
        Ok(result)
    }

    /// Internal function to deposit funds
    pub fn deposit_internal(
        &mut self,
        position: &mut TokenPosition,
        mut native_amount: I80F48,
        allow_dusting: bool,
        now_ts: u64,
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
            let indexed = native / index;
            if (indexed * index) < native {
                indexed + I80F48::DELTA
            } else {
                indexed
            }
        };

        if native_position.is_negative() {
            // Only account for the borrows we are repaying
            self.update_net_borrows(native_position.max(-native_amount), now_ts);

            let new_native_position = native_position + native_amount;
            let indexed_change = div_rounding_up(native_amount, self.borrow_index);
            // this is only correct if it's not positive, because it scales the whole amount by borrow_index
            let new_indexed_value = position.indexed_position + indexed_change;
            if new_indexed_value.is_negative() {
                // pay back borrows only, leaving a negative position
                self.indexed_borrows -= indexed_change;
                position.indexed_position = new_indexed_value;
                return Ok(true);
            } else if new_native_position < I80F48::ONE && allow_dusting {
                // if there's less than one token deposited, zero the position
                self.dust += new_native_position;
                self.indexed_borrows += position.indexed_position;
                position.indexed_position = I80F48::ZERO;
                return Ok(false);
            }

            // pay back all borrows
            self.indexed_borrows += position.indexed_position; // position.value is negative
            position.indexed_position = I80F48::ZERO;
            // deposit the rest
            // note: .max(0) because there's a scenario where new_indexed_value == 0 and new_native_position < 0
            native_amount = new_native_position.max(I80F48::ZERO);
        }

        // add to deposits
        let indexed_change = div_rounding_up(native_amount, self.deposit_index);
        self.indexed_deposits += indexed_change;
        position.indexed_position += indexed_change;

        Ok(true)
    }

    /// Withdraws `native_amount` without applying the loan origination fee.
    ///
    /// If the token position ends up positive but below one native token and this token
    /// position isn't marked as in-use, the token balance will be dusted, the position
    /// will be set to zero and this function returns Ok(false).
    ///
    /// native_amount must be >= 0
    /// fractional withdraws can be relevant during liquidation, for example
    pub fn withdraw_without_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
        oracle_price: I80F48,
    ) -> Result<bool> {
        let (position_is_active, _) = self.withdraw_internal_wrapper(
            position,
            native_amount,
            false,
            !position.is_in_use(),
            now_ts,
            Some(oracle_price),
        )?;

        Ok(position_is_active)
    }

    /// Like `withdraw_without_fee()` but allows dusting of in-use token accounts.
    ///
    /// Returns Ok(false) on dusted positions that weren't in-use.
    pub fn withdraw_without_fee_with_dusting(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
        oracle_price: I80F48,
    ) -> Result<bool> {
        self.withdraw_internal_wrapper(
            position,
            native_amount,
            false,
            true,
            now_ts,
            Some(oracle_price),
        )
        .map(|(not_dusted, _)| not_dusted || position.is_in_use())
    }

    /// Withdraws `native_amount` while applying the loan origination fee if a borrow is created.
    ///
    /// If the token position ends up positive but below one native token and this token
    /// position isn't marked as in-use, the token balance will be dusted, the position
    /// will be set to zero and this function returns Ok(false).
    ///
    /// native_amount must be >= 0
    /// fractional withdraws can be relevant during liquidation, for example
    pub fn withdraw_with_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
        oracle_price: I80F48,
    ) -> Result<(bool, I80F48)> {
        self.withdraw_internal_wrapper(
            position,
            native_amount,
            true,
            !position.is_in_use(),
            now_ts,
            Some(oracle_price),
        )
    }

    /// Internal function to withdraw funds
    fn withdraw_internal_wrapper(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        with_loan_origination_fee: bool,
        allow_dusting: bool,
        now_ts: u64,
        oracle_price: Option<I80F48>,
    ) -> Result<(bool, I80F48)> {
        let opening_indexed_position = position.indexed_position;
        let res = self.withdraw_internal(
            position,
            native_amount,
            with_loan_origination_fee,
            allow_dusting,
            now_ts,
            oracle_price,
        );
        self.update_cumulative_interest(position, opening_indexed_position);
        res
    }

    /// Internal function to withdraw funds
    fn withdraw_internal(
        &mut self,
        position: &mut TokenPosition,
        mut native_amount: I80F48,
        with_loan_origination_fee: bool,
        allow_dusting: bool,
        now_ts: u64,
        oracle_price: Option<I80F48>,
    ) -> Result<(bool, I80F48)> {
        require_gte!(native_amount, 0);
        let native_position = position.native(self);

        if native_position.is_positive() {
            let new_native_position = native_position - native_amount;
            if !new_native_position.is_negative() {
                // withdraw deposits only
                if new_native_position < I80F48::ONE && allow_dusting {
                    // zero the account collecting the leftovers in `dust`
                    self.dust += new_native_position;
                    self.indexed_deposits -= position.indexed_position;
                    position.indexed_position = I80F48::ZERO;
                    return Ok((false, I80F48::ZERO));
                } else {
                    // withdraw some deposits leaving a positive balance
                    let indexed_change = native_amount / self.deposit_index;
                    self.indexed_deposits -= indexed_change;
                    position.indexed_position -= indexed_change;
                    return Ok((true, I80F48::ZERO));
                }
            }

            // withdraw all deposits
            self.indexed_deposits -= position.indexed_position;
            position.indexed_position = I80F48::ZERO;
            // borrow the rest
            native_amount = -new_native_position;
        }

        let mut loan_origination_fee = I80F48::ZERO;
        if with_loan_origination_fee {
            loan_origination_fee = self.loan_origination_fee_rate * native_amount;
            self.collected_fees_native += loan_origination_fee;
            native_amount += loan_origination_fee;
        }

        // add to borrows
        let indexed_change = native_amount / self.borrow_index;
        self.indexed_borrows += indexed_change;
        position.indexed_position -= indexed_change;

        // net borrows requires updating in only this case, since other branches of the method deal with
        // withdraws and not borrows
        self.update_net_borrows(native_amount, now_ts);
        if let Some(oracle_price) = oracle_price {
            self.check_net_borrows(oracle_price)?;
        }

        Ok((true, loan_origination_fee))
    }

    // withdraw the loan origination fee for a borrow that happenend earlier
    pub fn withdraw_loan_origination_fee(
        &mut self,
        position: &mut TokenPosition,
        already_borrowed_native_amount: I80F48,
        now_ts: u64,
    ) -> Result<(bool, I80F48)> {
        let loan_origination_fee = self.loan_origination_fee_rate * already_borrowed_native_amount;
        self.collected_fees_native += loan_origination_fee;

        let (position_is_active, _) = self.withdraw_internal_wrapper(
            position,
            loan_origination_fee,
            false,
            !position.is_in_use(),
            now_ts,
            None,
        )?;

        Ok((position_is_active, loan_origination_fee))
    }

    /// Change a position without applying the loan origination fee
    pub fn change_without_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
        oracle_price: I80F48,
    ) -> Result<bool> {
        if native_amount >= 0 {
            self.deposit(position, native_amount, now_ts)
        } else {
            self.withdraw_without_fee(position, -native_amount, now_ts, oracle_price)
        }
    }

    /// Change a position, while taking the loan origination fee into account
    pub fn change_with_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
        oracle_price: I80F48,
    ) -> Result<(bool, I80F48)> {
        if native_amount >= 0 {
            Ok((self.deposit(position, native_amount, now_ts)?, I80F48::ZERO))
        } else {
            self.withdraw_with_fee(position, -native_amount, now_ts, oracle_price)
        }
    }

    /// Update the bank's net_borrows fields.
    ///
    /// If oracle_price is set, also do a net borrows check and error if the threshold is exceeded.
    pub fn update_net_borrows(&mut self, native_amount: I80F48, now_ts: u64) {
        let in_new_window =
            now_ts >= self.last_net_borrows_window_start_ts + self.net_borrow_limit_window_size_ts;

        let amount = native_amount.ceil().clamp_to_i64();

        self.net_borrows_in_window = if in_new_window {
            // reset to latest window
            self.last_net_borrows_window_start_ts = now_ts / self.net_borrow_limit_window_size_ts
                * self.net_borrow_limit_window_size_ts;
            amount
        } else {
            self.net_borrows_in_window + amount
        };
    }

    pub fn check_net_borrows(&self, oracle_price: I80F48) -> Result<()> {
        if self.net_borrows_in_window < 0 || self.net_borrow_limit_per_window_quote < 0 {
            return Ok(());
        }

        let price = oracle_price.max(self.stable_price());
        let net_borrows_quote = price
            .checked_mul_int(self.net_borrows_in_window.into())
            .unwrap();
        if net_borrows_quote > self.net_borrow_limit_per_window_quote {
            return Err(error_msg_typed!(MangoError::BankNetBorrowsLimitReached,
                    "net_borrows_in_window ({:?}) exceeds net_borrow_limit_per_window_quote ({:?}) for last_net_borrows_window_start_ts ({:?}) ",
                    self.net_borrows_in_window, self.net_borrow_limit_per_window_quote, self.last_net_borrows_window_start_ts

            ));
        }

        Ok(())
    }

    pub fn update_cumulative_interest(
        &self,
        position: &mut TokenPosition,
        opening_indexed_position: I80F48,
    ) {
        if opening_indexed_position.is_positive() {
            let interest = ((self.deposit_index - position.previous_index)
                * opening_indexed_position)
                .to_num::<f64>();
            position.cumulative_deposit_interest += interest;
        } else {
            let interest = ((self.borrow_index - position.previous_index)
                * opening_indexed_position)
                .to_num::<f64>();
            position.cumulative_borrow_interest -= interest;
        }

        if position.indexed_position.is_positive() {
            position.previous_index = self.deposit_index
        } else {
            position.previous_index = self.borrow_index
        }
    }

    pub fn compute_index(
        &self,
        indexed_total_deposits: I80F48,
        indexed_total_borrows: I80F48,
        diff_ts: I80F48,
    ) -> Result<(I80F48, I80F48, I80F48, I80F48, I80F48)> {
        // compute index based on utilization
        let native_total_deposits = self.deposit_index * indexed_total_deposits;
        let native_total_borrows = self.borrow_index * indexed_total_borrows;

        // This will be >= 0, but can also be > 1
        let instantaneous_utilization = if native_total_deposits == I80F48::ZERO {
            I80F48::ZERO
        } else {
            native_total_borrows / native_total_deposits
        };

        let borrow_rate = self.compute_interest_rate(instantaneous_utilization);

        // We want to grant depositors a rate that exactly matches the amount that is
        // taken from borrowers. That means:
        //   (new_deposit_index - old_deposit_index) * indexed_deposits
        //      = (new_borrow_index - old_borrow_index) * indexed_borrows
        // with
        //   new_deposit_index = old_deposit_index * (1 + deposit_rate) and
        //   new_borrow_index = old_borrow_index * (1 * borrow_rate)
        // we have
        //   deposit_rate = borrow_rate * (old_borrow_index * indexed_borrows) / (old_deposit_index * indexed_deposits)
        // and the latter factor is exactly instantaneous_utilization.
        let deposit_rate = borrow_rate * instantaneous_utilization;

        // The loan fee rate is not distributed to depositors.
        let borrow_rate_with_fees = borrow_rate + self.loan_fee_rate;
        let borrow_fees = native_total_borrows * self.loan_fee_rate * diff_ts / YEAR_I80F48;

        let borrow_index =
            (self.borrow_index * borrow_rate_with_fees * diff_ts) / YEAR_I80F48 + self.borrow_index;
        let deposit_index =
            (self.deposit_index * deposit_rate * diff_ts) / YEAR_I80F48 + self.deposit_index;

        Ok((
            deposit_index,
            borrow_index,
            borrow_fees,
            borrow_rate,
            deposit_rate,
        ))
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
            let slope = rate0 / util0;
            slope * utilization
        } else if utilization <= util1 {
            let extra_util = utilization - util0;
            let slope = (rate1 - rate0) / (util1 - util0);
            rate0 + slope * extra_util
        } else {
            let extra_util = utilization - util1;
            let slope = (max_rate - rate1) / (I80F48::ONE - util1);
            rate1 + slope * extra_util
        }
    }

    // compute new avg utilization
    pub fn compute_new_avg_utilization(
        &self,
        indexed_total_deposits: I80F48,
        indexed_total_borrows: I80F48,
        now_ts: u64,
    ) -> I80F48 {
        if now_ts == 0 {
            return I80F48::ZERO;
        }

        let native_total_deposits = self.deposit_index * indexed_total_deposits;
        let native_total_borrows = self.borrow_index * indexed_total_borrows;
        let instantaneous_utilization = if native_total_deposits == I80F48::ZERO {
            I80F48::ZERO
        } else {
            native_total_borrows / native_total_deposits
        };

        // Compute a time-weighted average since bank_rate_last_updated.
        let previous_avg_time =
            I80F48::from_num(self.index_last_updated - self.bank_rate_last_updated);
        let diff_ts = I80F48::from_num(now_ts - self.index_last_updated);
        let new_avg_time = I80F48::from_num(now_ts - self.bank_rate_last_updated);
        if new_avg_time <= 0 {
            return instantaneous_utilization;
        }
        (self.avg_utilization * previous_avg_time + instantaneous_utilization * diff_ts)
            / new_avg_time
    }

    // computes new optimal rates and max rate
    pub fn compute_rates(&self) -> (I80F48, I80F48, I80F48) {
        // interest rate legs 2 and 3 are seen as punitive legs, encouraging utilization to move towards optimal utilization
        // lets choose util0 as optimal utilization and 0 to utli0 as the leg where we want the utlization to preferably be
        let optimal_util = self.util0;
        // use avg_utilization and not instantaneous_utilization so that rates cannot be manipulated easily
        let avg_util = self.avg_utilization;
        // move rates up when utilization is above optimal utilization, and vice versa
        // util factor is between -1 (avg util = 0) and +1 (avg util = 100%)
        let util_factor = if avg_util > optimal_util {
            (avg_util - optimal_util) / (I80F48::ONE - optimal_util)
        } else {
            (avg_util - optimal_util) / optimal_util
        };
        let adjustment = I80F48::ONE + self.adjustment_factor * util_factor;

        // 1. irrespective of which leg current utilization is in, update all rates
        // 2. only update rates as long as new adjusted rates are above MINIMUM_MAX_RATE,
        //  since we don't want to fall to such low rates that it would take a long time to
        //  recover to high rates if utilization suddently increases to a high value
        if (self.max_rate * adjustment) > MINIMUM_MAX_RATE {
            (
                (self.rate0 * adjustment),
                (self.rate1 * adjustment),
                (self.max_rate * adjustment),
            )
        } else {
            (self.rate0, self.rate1, self.max_rate)
        }
    }

    pub fn oracle_price(
        &self,
        oracle_acc: &impl KeyedAccountReader,
        staleness_slot: Option<u64>,
    ) -> Result<I80F48> {
        require_keys_eq!(self.oracle, *oracle_acc.key());
        oracle::oracle_price(
            oracle_acc,
            &self.oracle_config,
            self.mint_decimals,
            staleness_slot,
        )
    }

    pub fn stable_price(&self) -> I80F48 {
        I80F48::from_num(self.stable_price_model.stable_price)
    }

    /// Returns the init asset weight, adjusted for the number of deposits on the bank.
    ///
    /// If max_collateral is 0, then the scaled init weight will be 0.
    /// Otherwise the weight is unadjusted until max_collateral and then scaled down
    /// such that scaled_init_weight * deposits remains constant.
    #[inline(always)]
    pub fn scaled_init_asset_weight(&self, price: I80F48) -> I80F48 {
        if self.deposit_weight_scale_start_quote == f64::MAX {
            return self.init_asset_weight;
        }
        // The next line is around 500 CU
        let deposits_quote = self.native_deposits().to_num::<f64>() * price.to_num::<f64>();
        if deposits_quote <= self.deposit_weight_scale_start_quote {
            self.init_asset_weight
        } else {
            // The next line is around 500 CU
            let scale = self.deposit_weight_scale_start_quote / deposits_quote;
            self.init_asset_weight * I80F48::from_num(scale)
        }
    }

    #[inline(always)]
    pub fn scaled_init_liab_weight(&self, price: I80F48) -> I80F48 {
        if self.borrow_weight_scale_start_quote == f64::MAX {
            return self.init_liab_weight;
        }
        // The next line is around 500 CU
        let borrows_quote = self.native_borrows().to_num::<f64>() * price.to_num::<f64>();
        if borrows_quote <= self.borrow_weight_scale_start_quote {
            self.init_liab_weight
        } else if self.borrow_weight_scale_start_quote == 0.0 {
            // TODO: will certainly cause overflow, so it's not exactly what is needed; health should be -MAX?
            // maybe handling this case isn't super helpful?
            I80F48::MAX
        } else {
            // The next line is around 500 CU
            let scale = borrows_quote / self.borrow_weight_scale_start_quote;
            self.init_liab_weight * I80F48::from_num(scale)
        }
    }
}

#[macro_export]
macro_rules! bank_seeds {
    ( $bank:expr ) => {
        &[
            b"Bank".as_ref(),
            $bank.group.as_ref(),
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
                bank.net_borrow_limit_window_size_ts = 1; // dummy
                bank.net_borrow_limit_per_window_quote = i64::MAX; // max since we don't want this to interfere
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
                    in_use_count: u8::from(is_in_use),
                    cumulative_deposit_interest: 0.0,
                    cumulative_borrow_interest: 0.0,
                    previous_index: I80F48::ZERO,
                    padding: Default::default(),
                    reserved: [0; 128],
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
                let dummy_now_ts = 1 as u64;
                let dummy_price = I80F48::ZERO;
                let (is_active, _) =
                    bank.change_with_fee(&mut account, change, dummy_now_ts, dummy_price)?;

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
        bank.bank_rate_last_updated = 1000;
        bank.index_last_updated = 1000;

        let compute_new_avg_utilization_runner =
            |bank: &mut Bank, utilization: I80F48, now_ts: u64| {
                bank.avg_utilization =
                    bank.compute_new_avg_utilization(I80F48::ONE, utilization, now_ts);
                bank.index_last_updated = now_ts;
            };

        compute_new_avg_utilization_runner(&mut bank, I80F48::ZERO, 1000);
        assert_eq!(bank.avg_utilization, I80F48::ZERO);

        compute_new_avg_utilization_runner(&mut bank, I80F48::from_num(0.5), 1010);
        assert!((bank.avg_utilization - I80F48::from_num(0.5)).abs() < 0.0001);

        compute_new_avg_utilization_runner(&mut bank, I80F48::from_num(0.8), 1015);
        assert!((bank.avg_utilization - I80F48::from_num(0.6)).abs() < 0.0001);

        compute_new_avg_utilization_runner(&mut bank, I80F48::ONE, 1020);
        assert!((bank.avg_utilization - I80F48::from_num(0.7)).abs() < 0.0001);

        bank.bank_rate_last_updated = 1020;
        compute_new_avg_utilization_runner(&mut bank, I80F48::ONE, 1040);
        assert_eq!(bank.avg_utilization, I80F48::ONE);
    }

    #[test]
    pub fn test_net_borrows() -> Result<()> {
        let mut bank = Bank::zeroed();
        bank.net_borrow_limit_window_size_ts = 100;
        bank.net_borrow_limit_per_window_quote = 1000;
        bank.deposit_index = I80F48::from_num(100.0);
        bank.borrow_index = I80F48::from_num(100.0);

        let price = I80F48::from(2);

        let mut account = TokenPosition::default();

        bank.change_without_fee(&mut account, I80F48::from(100), 0, price)
            .unwrap();
        assert_eq!(bank.net_borrows_in_window, 0);
        bank.change_without_fee(&mut account, I80F48::from(-100), 0, price)
            .unwrap();
        assert_eq!(bank.net_borrows_in_window, 0);

        account = TokenPosition::default();
        bank.change_without_fee(&mut account, I80F48::from(10), 0, price)
            .unwrap();
        bank.change_without_fee(&mut account, I80F48::from(-110), 0, price)
            .unwrap();
        assert_eq!(bank.net_borrows_in_window, 100);
        bank.change_without_fee(&mut account, I80F48::from(50), 0, price)
            .unwrap();
        assert_eq!(bank.net_borrows_in_window, 50);
        bank.change_without_fee(&mut account, I80F48::from(100), 0, price)
            .unwrap();
        assert_eq!(bank.net_borrows_in_window, 1); // rounding

        account = TokenPosition::default();
        bank.net_borrows_in_window = 0;
        bank.change_without_fee(&mut account, I80F48::from(-450), 0, price)
            .unwrap();
        bank.change_without_fee(&mut account, I80F48::from(-51), 0, price)
            .unwrap_err();

        account = TokenPosition::default();
        bank.net_borrows_in_window = 0;
        bank.change_without_fee(&mut account, I80F48::from(-450), 0, price)
            .unwrap();
        bank.change_without_fee(&mut account, I80F48::from(-50), 0, price)
            .unwrap();
        bank.change_without_fee(&mut account, I80F48::from(-50), 101, price)
            .unwrap();

        Ok(())
    }
}
