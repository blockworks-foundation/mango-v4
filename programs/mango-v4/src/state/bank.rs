use super::{OracleAccountInfos, OracleConfig, TokenIndex, TokenPosition};
use crate::accounts_zerocopy::KeyedAccountReader;
use crate::error::*;
use crate::i80f48::ClampToInt;
use crate::state::{oracle, StablePriceModel};
use crate::util;

use anchor_lang::prelude::*;
use derivative::Derivative;
use fixed::types::I80F48;
use oracle::oracle_log_context;
use static_assertions::const_assert_eq;

use std::mem::size_of;

pub const HOUR: i64 = 3600;
pub const DAY: i64 = 86400;
pub const DAY_I80F48: I80F48 = I80F48::from_bits(86_400 * I80F48::ONE.to_bits());
pub const ONE_BPS: I80F48 = I80F48::from_bits(28147497671);
pub const YEAR_I80F48: I80F48 = I80F48::from_bits(31_536_000 * I80F48::ONE.to_bits());

#[derive(Derivative)]
#[derivative(Debug)]
#[account(zero_copy)]
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

    /// The unscaled borrow interest curve is defined as continuous piecewise linear with the points:
    ///
    /// - 0% util: zero_util_rate
    /// - util0% util: rate0
    /// - util1% util: rate1
    /// - 100% util: max_rate
    ///
    /// The final rate is this unscaled curve multiplied by interest_curve_scaling.
    pub util0: I80F48,
    pub rate0: I80F48,
    pub util1: I80F48,
    pub rate1: I80F48,

    /// the 100% utilization rate
    ///
    /// This isn't the max_rate, since this still gets scaled by interest_curve_scaling,
    /// which is >=1.
    pub max_rate: I80F48,

    /// Fees collected over the lifetime of the bank
    ///
    /// See fees_withdrawn for how much of the fees was withdrawn.
    /// See collected_liquidation_fees for the (included) subtotal for liquidation related fees.
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

    /// Liquidation fee that goes to the liqor.
    ///
    /// Liquidation always involves two tokens, and the sum of the two configured fees is used.
    ///
    /// A fraction of the price, like 0.05 for a 5% fee during liquidation.
    ///
    /// See also platform_liquidation_fee.
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

    /// The maximum utilization allowed when borrowing is 1-this value
    /// WARNING: Outdated name, kept for IDL compatibility
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

    // We have 3 modes
    // 0 - Off,
    // 1 - ReduceDepositsReduceBorrows - standard
    // 2 - ReduceBorrows - borrows can only be reduced, but deposits have no restriction, special case for
    //                 force close mode, where liqor should first acquire deposits before closing liqee's borrows
    pub reduce_only: u8,
    pub force_close: u8,

    /// If set to 1, deposits cannot be liquidated when an account is liquidatable.
    /// That means bankrupt accounts may still have assets of this type deposited.
    pub disable_asset_liquidation: u8,

    pub force_withdraw: u8,

    #[derivative(Debug = "ignore")]
    pub padding: [u8; 4],

    // Do separate bookkeping for how many tokens were withdrawn
    // This ensures that collected_fees_native is strictly increasing for stats gathering purposes
    pub fees_withdrawn: u64,

    /// Fees for the token conditional swap feature
    pub token_conditional_swap_taker_fee_rate: f32,
    pub token_conditional_swap_maker_fee_rate: f32,

    pub flash_loan_swap_fee_rate: f32,

    /// Target utilization: If actual utilization is higher, scale up interest.
    /// If it's lower, scale down interest (if possible)
    pub interest_target_utilization: f32,

    /// Current interest curve scaling, always >= 1.0
    ///
    /// Except when first migrating to having this field, then 0.0
    pub interest_curve_scaling: f64,

    /// Largest amount of tokens that might be added the the bank based on
    /// serum open order execution.
    pub potential_serum_tokens: u64,

    /// Start timestamp in seconds at which maint weights should start to change away
    /// from maint_asset_weight, maint_liab_weight towards _asset_target and _liab_target.
    /// If _start and _end and _duration_inv are 0, no shift is configured.
    pub maint_weight_shift_start: u64,
    /// End timestamp in seconds until which the maint weights should reach the configured targets.
    pub maint_weight_shift_end: u64,
    /// Cache of the inverse of maint_weight_shift_end - maint_weight_shift_start,
    /// or zero if no shift is configured
    pub maint_weight_shift_duration_inv: I80F48,
    /// Maint asset weight to reach at _shift_end.
    pub maint_weight_shift_asset_target: I80F48,
    pub maint_weight_shift_liab_target: I80F48,

    /// Oracle that may be used if the main oracle is stale or not confident enough.
    /// If this is Pubkey::default(), no fallback is available.
    pub fallback_oracle: Pubkey,

    /// zero means none, in token native
    pub deposit_limit: u64,

    /// The unscaled borrow interest curve point for zero utilization.
    ///
    /// See util0, rate0, util1, rate1, max_rate
    pub zero_util_rate: I80F48,

    /// Additional to liquidation_fee, but goes to the group owner instead of the liqor
    pub platform_liquidation_fee: I80F48,

    /// Platform fees that were collected during liquidation (in native tokens)
    ///
    /// See also collected_fees_native and fees_withdrawn.
    pub collected_liquidation_fees: I80F48,

    /// Collateral fees that have been collected (in native tokens)
    ///
    /// See also collected_fees_native and fees_withdrawn.
    pub collected_collateral_fees: I80F48,

    /// The daily collateral fees rate for fully utilized collateral.
    pub collateral_fee_per_day: f32,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 1900],
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
        + 1
        + 6
        + 8
        + 4 * 4
        + 8 * 2
        + 8 * 2
        + 16 * 3
        + 32
        + 8
        + 16 * 4
        + 4
        + 1900
);
const_assert_eq!(size_of::<Bank>(), 3064);
const_assert_eq!(size_of::<Bank>() % 8, 0);

pub struct WithdrawResult {
    pub position_is_active: bool,
    pub loan_origination_fee: I80F48,
    pub loan_amount: I80F48,
}

impl WithdrawResult {
    pub fn has_loan(&self) -> bool {
        self.loan_amount.is_positive()
    }
}

pub struct TransferResult {
    pub source_is_active: bool,
    pub target_is_active: bool,
    pub loan_origination_fee: I80F48,
    pub loan_amount: I80F48,
}

impl TransferResult {
    pub fn has_loan(&self) -> bool {
        self.loan_amount.is_positive()
    }
}

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
            collected_liquidation_fees: I80F48::ZERO,
            collected_collateral_fees: I80F48::ZERO,
            fees_withdrawn: 0,
            dust: I80F48::ZERO,
            flash_loan_approved_amount: 0,
            flash_loan_token_account_initial: u64::MAX,
            net_borrows_in_window: 0,
            potential_serum_tokens: 0,
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
            stable_price_model: existing_bank.stable_price_model,
            min_vault_to_deposits_ratio: existing_bank.min_vault_to_deposits_ratio,
            net_borrow_limit_per_window_quote: existing_bank.net_borrow_limit_per_window_quote,
            net_borrow_limit_window_size_ts: existing_bank.net_borrow_limit_window_size_ts,
            last_net_borrows_window_start_ts: existing_bank.last_net_borrows_window_start_ts,
            borrow_weight_scale_start_quote: existing_bank.borrow_weight_scale_start_quote,
            deposit_weight_scale_start_quote: existing_bank.deposit_weight_scale_start_quote,
            reduce_only: existing_bank.reduce_only,
            force_close: existing_bank.force_close,
            disable_asset_liquidation: existing_bank.disable_asset_liquidation,
            force_withdraw: existing_bank.force_withdraw,
            padding: [0; 4],
            token_conditional_swap_taker_fee_rate: existing_bank
                .token_conditional_swap_taker_fee_rate,
            token_conditional_swap_maker_fee_rate: existing_bank
                .token_conditional_swap_maker_fee_rate,
            flash_loan_swap_fee_rate: existing_bank.flash_loan_swap_fee_rate,
            interest_target_utilization: existing_bank.interest_target_utilization,
            interest_curve_scaling: existing_bank.interest_curve_scaling,
            maint_weight_shift_start: existing_bank.maint_weight_shift_start,
            maint_weight_shift_end: existing_bank.maint_weight_shift_end,
            maint_weight_shift_duration_inv: existing_bank.maint_weight_shift_duration_inv,
            maint_weight_shift_asset_target: existing_bank.maint_weight_shift_asset_target,
            maint_weight_shift_liab_target: existing_bank.maint_weight_shift_liab_target,
            fallback_oracle: existing_bank.oracle,
            deposit_limit: existing_bank.deposit_limit,
            zero_util_rate: existing_bank.zero_util_rate,
            platform_liquidation_fee: existing_bank.platform_liquidation_fee,
            collateral_fee_per_day: existing_bank.collateral_fee_per_day,
            reserved: [0; 1900],
        }
    }

    pub fn verify(&self) -> Result<()> {
        require_gte!(self.oracle_config.conf_filter, 0.0);
        require_gte!(self.util0, I80F48::ZERO);
        require_gte!(self.util1, self.util0);
        require_gte!(I80F48::ONE, self.util1);
        require_gte!(self.rate0, I80F48::ZERO);
        require_gte!(self.rate1, I80F48::ZERO);
        require_gte!(self.max_rate, I80F48::ZERO);
        require_gte!(self.loan_fee_rate, 0.0);
        require_gte!(self.loan_origination_fee_rate, 0.0);
        require_gte!(self.maint_asset_weight, 0.0);
        require_gte!(self.init_asset_weight, 0.0);
        require_gte!(self.maint_liab_weight, 0.0);
        require_gte!(self.init_liab_weight, 0.0);
        require_gte!(self.liquidation_fee, 0.0);
        require_gte!(self.min_vault_to_deposits_ratio, 0.0);
        require_gte!(self.net_borrow_limit_per_window_quote, -1);
        require_gt!(self.borrow_weight_scale_start_quote, 0.0);
        require_gt!(self.deposit_weight_scale_start_quote, 0.0);
        require_gte!(2, self.reduce_only);
        require_gte!(self.token_conditional_swap_taker_fee_rate, 0.0);
        require_gte!(self.token_conditional_swap_maker_fee_rate, 0.0);
        require_gte!(self.flash_loan_swap_fee_rate, 0.0);
        require_gte!(self.interest_curve_scaling, 1.0);
        require_gte!(self.interest_target_utilization, 0.0);
        require_gte!(self.maint_weight_shift_duration_inv, 0.0);
        require_gte!(self.maint_weight_shift_asset_target, 0.0);
        require_gte!(self.maint_weight_shift_liab_target, 0.0);
        require_gte!(self.zero_util_rate, I80F48::ZERO);
        require_gte!(self.platform_liquidation_fee, 0.0);
        if !self.allows_asset_liquidation() {
            require!(self.are_borrows_reduce_only(), MangoError::SomeError);
            require_eq!(self.maint_asset_weight, I80F48::ZERO);
        }
        require_gte!(self.collateral_fee_per_day, 0.0);
        if self.is_force_withdraw() {
            require!(self.are_deposits_reduce_only(), MangoError::SomeError);
            require!(!self.allows_asset_liquidation(), MangoError::SomeError);
            require_eq!(self.maint_asset_weight, I80F48::ZERO);
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn are_deposits_reduce_only(&self) -> bool {
        self.reduce_only == 1
    }

    pub fn are_borrows_reduce_only(&self) -> bool {
        self.reduce_only == 1 || self.reduce_only == 2
    }

    pub fn is_force_close(&self) -> bool {
        self.force_close == 1
    }

    pub fn is_force_withdraw(&self) -> bool {
        self.force_withdraw == 1
    }

    pub fn allows_asset_liquidation(&self) -> bool {
        self.disable_asset_liquidation == 0
    }

    #[inline(always)]
    pub fn native_borrows(&self) -> I80F48 {
        self.borrow_index * self.indexed_borrows
    }

    #[inline(always)]
    pub fn native_deposits(&self) -> I80F48 {
        self.deposit_index * self.indexed_deposits
    }

    pub fn maint_weights(&self, now_ts: u64) -> (I80F48, I80F48) {
        if self.maint_weight_shift_duration_inv.is_zero() || now_ts <= self.maint_weight_shift_start
        {
            (self.maint_asset_weight, self.maint_liab_weight)
        } else if now_ts >= self.maint_weight_shift_end {
            (
                self.maint_weight_shift_asset_target,
                self.maint_weight_shift_liab_target,
            )
        } else {
            let scale = I80F48::from(now_ts - self.maint_weight_shift_start)
                * self.maint_weight_shift_duration_inv;
            let asset = self.maint_asset_weight
                + scale * (self.maint_weight_shift_asset_target - self.maint_asset_weight);
            let liab = self.maint_liab_weight
                + scale * (self.maint_weight_shift_liab_target - self.maint_liab_weight);
            (asset, liab)
        }
    }

    pub fn enforce_borrows_lte_deposits(&self) -> Result<()> {
        self.enforce_max_utilization(I80F48::ONE)
    }

    /// Prevent borrowing away the full bank vault.
    /// Keep some in reserve to satisfy non-borrow withdraws.
    pub fn enforce_max_utilization_on_borrow(&self) -> Result<()> {
        self.enforce_max_utilization(
            I80F48::ONE - I80F48::from_num(self.min_vault_to_deposits_ratio),
        )
    }

    /// Prevent borrowing away the full bank vault.
    /// Keep some in reserve to satisfy non-borrow withdraws.
    fn enforce_max_utilization(&self, max_utilization: I80F48) -> Result<()> {
        let bank_native_deposits = self.native_deposits();
        let bank_native_borrows = self.native_borrows();

        if bank_native_borrows > max_utilization * bank_native_deposits {
            return err!(MangoError::BankBorrowLimitReached).with_context(|| {
                format!(
                    "deposits {}, borrows {}, max utilization {}",
                    bank_native_deposits, bank_native_borrows, max_utilization,
                )
            });
        };

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
    ) -> Result<bool> {
        let position_is_active = self
            .withdraw_internal_wrapper(
                position,
                native_amount,
                false,
                !position.is_in_use(),
                now_ts,
            )?
            .position_is_active;

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
    ) -> Result<bool> {
        self.withdraw_internal_wrapper(position, native_amount, false, true, now_ts)
            .map(|withdraw_result| withdraw_result.position_is_active || position.is_in_use())
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
    ) -> Result<WithdrawResult> {
        self.withdraw_internal_wrapper(position, native_amount, true, !position.is_in_use(), now_ts)
    }

    /// Internal function to withdraw funds
    fn withdraw_internal_wrapper(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        with_loan_origination_fee: bool,
        allow_dusting: bool,
        now_ts: u64,
    ) -> Result<WithdrawResult> {
        let opening_indexed_position = position.indexed_position;
        let res = self.withdraw_internal(
            position,
            native_amount,
            with_loan_origination_fee,
            allow_dusting,
            now_ts,
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
    ) -> Result<WithdrawResult> {
        require_gte!(native_amount, 0);
        let native_position = position.native(self);

        if !native_position.is_negative() {
            let new_native_position = native_position - native_amount;
            if !new_native_position.is_negative() {
                // withdraw deposits only
                if new_native_position < I80F48::ONE && allow_dusting {
                    // zero the account collecting the leftovers in `dust`
                    self.dust += new_native_position;
                    self.indexed_deposits -= position.indexed_position;
                    position.indexed_position = I80F48::ZERO;
                    return Ok(WithdrawResult {
                        position_is_active: false,
                        loan_origination_fee: I80F48::ZERO,
                        loan_amount: I80F48::ZERO,
                    });
                } else {
                    // withdraw some deposits leaving a positive balance
                    let indexed_change = native_amount / self.deposit_index;
                    self.indexed_deposits -= indexed_change;
                    position.indexed_position -= indexed_change;
                    return Ok(WithdrawResult {
                        position_is_active: true,
                        loan_origination_fee: I80F48::ZERO,
                        loan_amount: I80F48::ZERO,
                    });
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

        Ok(WithdrawResult {
            position_is_active: true,
            loan_origination_fee,
            loan_amount: native_amount,
        })
    }

    // withdraw the loan origination fee for a borrow that happened earlier
    pub fn withdraw_loan_origination_fee(
        &mut self,
        position: &mut TokenPosition,
        already_borrowed_native_amount: I80F48,
        now_ts: u64,
    ) -> Result<WithdrawResult> {
        let loan_origination_fee = self.loan_origination_fee_rate * already_borrowed_native_amount;
        self.collected_fees_native += loan_origination_fee;

        let position_is_active = self
            .withdraw_internal_wrapper(
                position,
                loan_origination_fee,
                false,
                !position.is_in_use(),
                now_ts,
            )?
            .position_is_active;

        Ok(WithdrawResult {
            position_is_active,
            loan_origination_fee,
            // To avoid double counting of loans return loan_amount of 0 here (as the loan_amount has already been returned earlier with loan_origination_fee == 0)
            loan_amount: I80F48::ZERO,
        })
    }

    /// Returns true if the position remains active
    pub fn dust_if_possible(&mut self, position: &mut TokenPosition, now_ts: u64) -> Result<bool> {
        if position.is_in_use() {
            return Ok(true);
        }
        let native = position.native(self);
        if native >= 0 && native < 1 {
            // Withdrawing 0 triggers the dusting check
            return self.withdraw_without_fee(position, I80F48::ZERO, now_ts);
        }
        Ok(true)
    }

    /// Change a position without applying the loan origination fee
    pub fn change_without_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
    ) -> Result<bool> {
        if native_amount >= 0 {
            self.deposit(position, native_amount, now_ts)
        } else {
            self.withdraw_without_fee(position, -native_amount, now_ts)
        }
    }

    /// Change a position, while taking the loan origination fee into account
    pub fn change_with_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
    ) -> Result<WithdrawResult> {
        if native_amount >= 0 {
            Ok(WithdrawResult {
                position_is_active: self.deposit(position, native_amount, now_ts)?,
                loan_origination_fee: I80F48::ZERO,
                loan_amount: I80F48::ZERO,
            })
        } else {
            self.withdraw_with_fee(position, -native_amount, now_ts)
        }
    }

    /// Generic "transfer" from source to target.
    ///
    /// Amounts for source and target can differ and can be zero.
    /// Checks reduce-only, net borrow limits and deposit limits.
    pub fn checked_transfer_with_fee(
        &mut self,
        source: &mut TokenPosition,
        source_amount: I80F48,
        target: &mut TokenPosition,
        target_amount: I80F48,
        now_ts: u64,
        oracle_price: I80F48,
    ) -> Result<TransferResult> {
        let before_borrows = self.indexed_borrows;
        let before_deposits = self.indexed_deposits;

        let withdraw_result = if !source_amount.is_zero() {
            let withdraw_result = self.withdraw_with_fee(source, source_amount, now_ts)?;
            require!(
                source.indexed_position >= 0 || !self.are_borrows_reduce_only(),
                MangoError::TokenInReduceOnlyMode
            );
            withdraw_result
        } else {
            WithdrawResult {
                position_is_active: true,
                loan_amount: I80F48::ZERO,
                loan_origination_fee: I80F48::ZERO,
            }
        };

        let target_is_active = if !target_amount.is_zero() {
            let active = self.deposit(target, target_amount, now_ts)?;
            require!(
                target.indexed_position <= 0 || !self.are_deposits_reduce_only(),
                MangoError::TokenInReduceOnlyMode
            );
            active
        } else {
            true
        };

        // Adding DELTA here covers the case where we add slightly more than we withdraw
        if self.indexed_borrows > before_borrows + I80F48::DELTA {
            self.check_net_borrows(oracle_price)?;
        }
        if self.indexed_deposits > before_deposits + I80F48::DELTA {
            self.check_deposit_and_oo_limit()?;
        }

        Ok(TransferResult {
            source_is_active: withdraw_result.position_is_active,
            target_is_active,
            loan_origination_fee: withdraw_result.loan_origination_fee,
            loan_amount: withdraw_result.loan_amount,
        })
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

    pub fn remaining_net_borrows_quote(&self, oracle_price: I80F48) -> I80F48 {
        if self.net_borrows_in_window < 0 || self.net_borrow_limit_per_window_quote < 0 {
            return I80F48::MAX;
        }

        let price = oracle_price.max(self.stable_price());
        let net_borrows_quote = price
            .checked_mul_int(self.net_borrows_in_window.into())
            .unwrap();

        I80F48::from(self.net_borrow_limit_per_window_quote) - net_borrows_quote
    }

    pub fn check_net_borrows(&self, oracle_price: I80F48) -> Result<()> {
        let remaining_quote = self.remaining_net_borrows_quote(oracle_price);
        if remaining_quote < 0 {
            return Err(error_msg_typed!(MangoError::BankNetBorrowsLimitReached,
                    "net_borrows_in_window: {:?}, remaining quote: {:?}, net_borrow_limit_per_window_quote: {:?}, last_net_borrows_window_start_ts: {:?}",
                    self.net_borrows_in_window, remaining_quote, self.net_borrow_limit_per_window_quote, self.last_net_borrows_window_start_ts

            ));
        }

        Ok(())
    }

    pub fn remaining_deposits_until_limit(&self) -> I80F48 {
        if self.deposit_limit == 0 {
            return I80F48::MAX;
        }

        // Assuming slightly higher deposits than true allows the returned value
        // to be deposit()ed safely into this bank without triggering limits.
        // (because deposit() will round up in favor of the user)
        let deposits = self.deposit_index * (self.indexed_deposits + I80F48::DELTA);

        let serum = I80F48::from(self.potential_serum_tokens);
        let total = deposits + serum;

        I80F48::from(self.deposit_limit) - total
    }

    pub fn check_deposit_and_oo_limit(&self) -> Result<()> {
        if self.deposit_limit == 0 {
            return Ok(());
        }

        // Intentionally does not use remaining_deposits_until_limit(): That function
        // returns slightly less than the true limit to make sure depositing that amount
        // will not cause a limit overrun.
        let deposits = self.native_deposits();
        let serum = I80F48::from(self.potential_serum_tokens);
        let total = deposits + serum;
        let remaining = I80F48::from(self.deposit_limit) - total;
        if remaining < 0 {
            return Err(error_msg_typed!(
                MangoError::BankDepositLimit,
                "deposit limit exceeded: remaining: {}, total: {}, limit: {}, deposits: {}, serum: {}",
                remaining,
                total,
                self.deposit_limit,
                deposits,
                serum,
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

        let instantaneous_utilization =
            Self::instantaneous_utilization(native_total_deposits, native_total_borrows);

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

    /// Current utilization, clamped to 0..1
    ///
    /// Above 100% utilization can happen natually when utilization is 100% and interest is paid out,
    /// increasing borrows more than deposits.
    fn instantaneous_utilization(
        native_total_deposits: I80F48,
        native_total_borrows: I80F48,
    ) -> I80F48 {
        if native_total_deposits == I80F48::ZERO {
            I80F48::ZERO
        } else {
            (native_total_borrows / native_total_deposits)
                .max(I80F48::ZERO)
                .min(I80F48::ONE)
        }
    }

    /// returns the current interest rate in APR
    #[inline(always)]
    pub fn compute_interest_rate(&self, utilization: I80F48) -> I80F48 {
        Bank::interest_rate_curve_calculator(
            utilization,
            self.zero_util_rate,
            self.util0,
            self.rate0,
            self.util1,
            self.rate1,
            self.max_rate,
            self.interest_curve_scaling,
        )
    }

    /// calculator function that can be used to compute an interest
    /// rate based on the given parameters
    #[inline(always)]
    pub fn interest_rate_curve_calculator(
        utilization: I80F48,
        zero_util_rate: I80F48,
        util0: I80F48,
        rate0: I80F48,
        util1: I80F48,
        rate1: I80F48,
        max_rate: I80F48,
        scaling: f64,
    ) -> I80F48 {
        // Clamp to avoid negative or extremely high interest
        let utilization = utilization.max(I80F48::ZERO).min(I80F48::ONE);

        let v = if utilization <= util0 {
            let slope = (rate0 - zero_util_rate) / util0;
            zero_util_rate + slope * utilization
        } else if utilization <= util1 {
            let extra_util = utilization - util0;
            let slope = (rate1 - rate0) / (util1 - util0);
            rate0 + slope * extra_util
        } else {
            let extra_util = utilization - util1;
            let slope = (max_rate - rate1) / (I80F48::ONE - util1);
            rate1 + slope * extra_util
        };

        // scaling will be 0 when it's introduced
        if scaling == 0.0 {
            v
        } else {
            v * I80F48::from_num(scaling)
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
        let instantaneous_utilization =
            Self::instantaneous_utilization(native_total_deposits, native_total_borrows);

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
    pub fn update_interest_rate_scaling(&mut self) {
        // Interest increases above target_util, decreases below
        let target_util = self.interest_target_utilization as f64;

        // use avg_utilization and not instantaneous_utilization so that rates cannot be manipulated easily
        // also clamp to avoid unusually quick interest rate curve changes
        let avg_util = self.avg_utilization.to_num::<f64>().max(0.0).min(1.0);

        // move rates up when utilization is above optimal utilization, and vice versa
        // util factor is between -1 (avg util = 0) and +1 (avg util = 100%)
        let util_factor = if avg_util > target_util {
            (avg_util - target_util) / (1.0 - target_util)
        } else {
            (avg_util - target_util) / target_util
        };
        let adjustment = 1.0 + self.adjustment_factor.to_num::<f64>() * util_factor;

        self.interest_curve_scaling = (self.interest_curve_scaling * adjustment).max(1.0)
    }

    /// Tries to return the primary oracle price, and if there is a confidence or staleness issue returns the fallback oracle price if possible.
    pub fn oracle_price<T: KeyedAccountReader>(
        &self,
        oracle_acc_infos: &OracleAccountInfos<T>,
        staleness_slot: Option<u64>,
    ) -> Result<I80F48> {
        require_keys_eq!(self.oracle, *oracle_acc_infos.oracle.key());
        let primary_state = oracle::oracle_state_unchecked(oracle_acc_infos, self.mint_decimals)?;
        let primary_ok =
            primary_state.check_confidence_and_maybe_staleness(&self.oracle_config, staleness_slot);
        if primary_ok.is_oracle_error() && oracle_acc_infos.fallback_opt.is_some() {
            let fallback_oracle_acc = oracle_acc_infos.fallback_opt.unwrap();
            require_keys_eq!(self.fallback_oracle, *fallback_oracle_acc.key());
            let fallback_state =
                oracle::fallback_oracle_state_unchecked(&oracle_acc_infos, self.mint_decimals)?;
            let fallback_ok = fallback_state
                .check_confidence_and_maybe_staleness(&self.oracle_config, staleness_slot);
            fallback_ok.with_context(|| {
                format!(
                    "{} {}",
                    oracle_log_context(
                        self.name(),
                        &primary_state,
                        &self.oracle_config,
                        staleness_slot
                    ),
                    oracle_log_context(
                        self.name(),
                        &fallback_state,
                        &self.oracle_config,
                        staleness_slot
                    )
                )
            })?;
            Ok(fallback_state.price)
        } else {
            primary_ok.with_context(|| {
                oracle_log_context(
                    self.name(),
                    &primary_state,
                    &self.oracle_config,
                    staleness_slot,
                )
            })?;
            Ok(primary_state.price)
        }
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
        let all_deposits =
            self.native_deposits().to_num::<f64>() + self.potential_serum_tokens as f64;
        let deposits_quote = all_deposits * price.to_num::<f64>();
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

    /// Grows potential_serum_tokens if new > old, shrinks it otherwise
    #[inline(always)]
    pub fn update_potential_serum_tokens(&mut self, old: u64, new: u64) {
        if new >= old {
            self.potential_serum_tokens += new - old;
        } else {
            self.potential_serum_tokens = self.potential_serum_tokens.saturating_sub(old - new);
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

    fn bank_change_runner(start: f64, change: i32, is_in_use: bool, use_withdraw: bool) {
        println!(
            "testing: in use: {is_in_use}, start: {start}, change: {change}, use_withdraw: {use_withdraw}",
        );

        let epsilon = I80F48::from_bits(1);

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
                let i = v / b.deposit_index;
                if i * b.deposit_index < v {
                    i + I80F48::DELTA
                } else {
                    i
                }
            } else {
                v / b.borrow_index
            }
        };

        let mut account = TokenPosition {
            indexed_position: I80F48::ZERO,
            token_index: 0,
            in_use_count: u16::from(is_in_use),
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
        let is_active = if use_withdraw {
            bank.withdraw_with_fee(&mut account, change, dummy_now_ts)
                .unwrap()
                .position_is_active
        } else {
            bank.change_with_fee(&mut account, change, dummy_now_ts)
                .unwrap()
                .position_is_active
        };

        let mut expected_native = start_native + change;
        let is_deposit_into_nonnegative = start >= 0.0 && change >= 0 && !use_withdraw;
        if expected_native >= 0.0
            && expected_native < 1.0
            && !is_in_use
            && !is_deposit_into_nonnegative
        {
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

    #[test]
    pub fn bank_change() -> Result<()> {
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
            (10.0, 0),
            (1.0, 0),
            (0.1, 0),
            (0.0, 0),
            (-0.1, 0),
        ];

        for is_in_use in [false, true] {
            for (start, change) in cases {
                bank_change_runner(start, change, is_in_use, false);
                if change == 0 {
                    // check withdrawing 0
                    bank_change_runner(start, change, is_in_use, true);
                }
            }
        }
        Ok(())
    }

    #[test]
    fn bank_transfer() {
        //
        // SETUP
        //

        let mut bank_proto = Bank::zeroed();
        bank_proto.net_borrow_limit_window_size_ts = 1; // dummy
        bank_proto.net_borrow_limit_per_window_quote = i64::MAX; // max since we don't want this to interfere
        bank_proto.deposit_index = I80F48::from(1_234_567);
        bank_proto.borrow_index = I80F48::from(1_234_567);
        bank_proto.loan_origination_fee_rate = I80F48::from_num(0.1);

        let account_proto = TokenPosition {
            indexed_position: I80F48::ZERO,
            token_index: 0,
            in_use_count: 1,
            cumulative_deposit_interest: 0.0,
            cumulative_borrow_interest: 0.0,
            previous_index: I80F48::ZERO,
            padding: Default::default(),
            reserved: [0; 128],
        };

        //
        // TESTS
        //

        // simple transfer
        {
            let mut bank = bank_proto.clone();
            let mut a1 = account_proto.clone();
            let mut a2 = account_proto.clone();

            let amount = I80F48::from(100);
            bank.deposit(&mut a1, amount, 0).unwrap();
            let damount = a1.native(&bank);
            let r = bank
                .checked_transfer_with_fee(&mut a1, amount, &mut a2, amount, 0, I80F48::ONE)
                .unwrap();
            assert_eq!(a2.native(&bank), damount);
            assert!(r.source_is_active);
            assert!(r.target_is_active);
        }

        // borrow limits
        {
            let mut bank = bank_proto.clone();
            bank.net_borrow_limit_per_window_quote = 100;
            bank.loan_origination_fee_rate = I80F48::ZERO;
            let mut a1 = account_proto.clone();
            let mut a2 = account_proto.clone();

            {
                let mut b = bank.clone();
                let amount = I80F48::from(101);
                assert!(b
                    .checked_transfer_with_fee(&mut a1, amount, &mut a2, amount, 0, I80F48::ONE)
                    .is_err());
            }

            {
                let mut b = bank.clone();
                let amount = I80F48::from(100);
                b.checked_transfer_with_fee(&mut a1, amount, &mut a2, amount, 0, I80F48::ONE)
                    .unwrap();
            }

            {
                let mut b = bank.clone();
                let amount = b.remaining_net_borrows_quote(I80F48::ONE);
                b.checked_transfer_with_fee(&mut a1, amount, &mut a2, amount, 0, I80F48::ONE)
                    .unwrap();
            }
        }

        // deposit limits
        {
            let mut bank = bank_proto.clone();
            bank.deposit_limit = 100;
            let mut a1 = account_proto.clone();
            let mut a2 = account_proto.clone();

            {
                let mut b = bank.clone();
                let amount = I80F48::from(101);
                assert!(b
                    .checked_transfer_with_fee(&mut a1, amount, &mut a2, amount, 0, I80F48::ONE)
                    .is_err());
            }

            {
                // still bad because deposit() adds DELTA more than requested
                let mut b = bank.clone();
                let amount = I80F48::from(100);
                assert!(b
                    .checked_transfer_with_fee(&mut a1, amount, &mut a2, amount, 0, I80F48::ONE)
                    .is_err());
            }

            {
                let mut b = bank.clone();
                let amount = I80F48::from_num(99.999);
                b.checked_transfer_with_fee(&mut a1, amount, &mut a2, amount, 0, I80F48::ONE)
                    .unwrap();
            }

            {
                let mut b = bank.clone();
                let amount = b.remaining_deposits_until_limit();
                b.checked_transfer_with_fee(&mut a1, amount, &mut a2, amount, 0, I80F48::ONE)
                    .unwrap();
            }
        }

        // reducing transfer while limits exceeded
        {
            let mut bank = bank_proto.clone();
            bank.loan_origination_fee_rate = I80F48::ZERO;

            let amount = I80F48::from(100);
            let mut a1 = account_proto.clone();
            bank.deposit(&mut a1, amount, 0).unwrap();
            let mut a2 = account_proto.clone();
            bank.withdraw_with_fee(&mut a2, amount, 0).unwrap();

            bank.net_borrow_limit_per_window_quote = 100;
            bank.net_borrows_in_window = 200;
            bank.deposit_limit = 100;
            bank.potential_serum_tokens = 200;

            let half = I80F48::from(50);
            bank.checked_transfer_with_fee(&mut a1, half, &mut a2, half, 0, I80F48::ONE)
                .unwrap();
            bank.checked_transfer_with_fee(&mut a1, half, &mut a2, half, 0, I80F48::ONE)
                .unwrap();
            assert!(bank
                .checked_transfer_with_fee(&mut a1, half, &mut a2, half, 0, I80F48::ONE)
                .is_err());
        }
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

        bank.change_without_fee(&mut account, I80F48::from(100), 0)
            .unwrap();
        assert_eq!(bank.net_borrows_in_window, 0);
        bank.change_without_fee(&mut account, I80F48::from(-100), 0)
            .unwrap();
        assert_eq!(bank.net_borrows_in_window, 0);

        account = TokenPosition::default();
        bank.change_without_fee(&mut account, I80F48::from(10), 0)
            .unwrap();
        bank.change_without_fee(&mut account, I80F48::from(-110), 0)
            .unwrap();
        assert_eq!(bank.net_borrows_in_window, 100);
        bank.change_without_fee(&mut account, I80F48::from(50), 0)
            .unwrap();
        assert_eq!(bank.net_borrows_in_window, 50);
        bank.change_without_fee(&mut account, I80F48::from(100), 0)
            .unwrap();
        assert_eq!(bank.net_borrows_in_window, 1); // rounding

        account = TokenPosition::default();
        bank.net_borrows_in_window = 0;
        bank.change_without_fee(&mut account, I80F48::from(-450), 0)
            .unwrap();
        bank.change_without_fee(&mut account, I80F48::from(-51), 0)
            .unwrap();
        bank.check_net_borrows(price).unwrap_err();

        account = TokenPosition::default();
        bank.net_borrows_in_window = 0;
        bank.change_without_fee(&mut account, I80F48::from(-450), 0)
            .unwrap();
        bank.change_without_fee(&mut account, I80F48::from(-50), 0)
            .unwrap();
        bank.change_without_fee(&mut account, I80F48::from(-50), 101)
            .unwrap();

        Ok(())
    }

    #[test]
    pub fn test_bank_maint_weight_shift() -> Result<()> {
        let mut bank = Bank::zeroed();
        bank.maint_asset_weight = I80F48::ONE;
        bank.maint_liab_weight = I80F48::ZERO;
        bank.maint_weight_shift_start = 100;
        bank.maint_weight_shift_end = 1100;
        bank.maint_weight_shift_duration_inv = I80F48::ONE / I80F48::from(1000);
        bank.maint_weight_shift_asset_target = I80F48::from(2);
        bank.maint_weight_shift_liab_target = I80F48::from(10);

        let (a, l) = bank.maint_weights(0);
        assert_eq!(a, 1.0);
        assert_eq!(l, 0.0);

        let (a, l) = bank.maint_weights(100);
        assert_eq!(a, 1.0);
        assert_eq!(l, 0.0);

        let (a, l) = bank.maint_weights(1100);
        assert_eq!(a, 2.0);
        assert_eq!(l, 10.0);

        let (a, l) = bank.maint_weights(2000);
        assert_eq!(a, 2.0);
        assert_eq!(l, 10.0);

        let abs_diff = |x: I80F48, y: f64| (x.to_num::<f64>() - y).abs();

        let (a, l) = bank.maint_weights(600);
        assert!(abs_diff(a, 1.5) < 1e-8);
        assert!(abs_diff(l, 5.0) < 1e-8);

        let (a, l) = bank.maint_weights(200);
        assert!(abs_diff(a, 1.1) < 1e-8);
        assert!(abs_diff(l, 1.0) < 1e-8);

        let (a, l) = bank.maint_weights(1000);
        assert!(abs_diff(a, 1.9) < 1e-8);
        assert!(abs_diff(l, 9.0) < 1e-8);

        Ok(())
    }

    #[test]
    pub fn test_bank_interest() -> Result<()> {
        let index_start = I80F48::from(1_000_000);

        let mut bank = Bank::zeroed();
        bank.util0 = I80F48::from_num(0.5);
        bank.rate0 = I80F48::from_num(0.02);
        bank.util1 = I80F48::from_num(0.75);
        bank.rate1 = I80F48::from_num(0.05);
        bank.max_rate = I80F48::from_num(0.5);
        bank.interest_curve_scaling = 4.0;
        bank.deposit_index = index_start;
        bank.borrow_index = index_start;
        bank.net_borrow_limit_window_size_ts = 1;

        let mut position0 = TokenPosition::default();
        let mut position1 = TokenPosition::default();

        // create 100% utilization, meaning 0.5 * 4 = 200% interest
        bank.deposit(&mut position0, I80F48::from(1_000_000_000), 0)
            .unwrap();
        bank.withdraw_without_fee(&mut position1, I80F48::from(1_000_000_000), 0)
            .unwrap();

        // accumulate interest for a day at 5s intervals
        let interval = 5;
        for i in 0..24 * 60 * 60 / interval {
            let (deposit_index, borrow_index, borrow_fees, borrow_rate, deposit_rate) = bank
                .compute_index(
                    bank.indexed_deposits,
                    bank.indexed_borrows,
                    I80F48::from(interval),
                )
                .unwrap();
            bank.deposit_index = deposit_index;
            bank.borrow_index = borrow_index;
        }

        // the 5s rate is 2/(365*24*60*60/5), so
        // expected is (1+five_sec_rate)^(24*60*60/5)
        assert!(
            ((bank.deposit_index / index_start).to_num::<f64>() - 1.0054944908).abs() < 0.0000001
        );
        assert!(
            ((bank.borrow_index / index_start).to_num::<f64>() - 1.0054944908).abs() < 0.0000001
        );

        Ok(())
    }

    #[test]
    fn test_bank_interest_rate_curve() {
        let mut bank = Bank::zeroed();
        bank.zero_util_rate = I80F48::from(1);
        bank.rate0 = I80F48::from(3);
        bank.rate1 = I80F48::from(7);
        bank.max_rate = I80F48::from(13);

        bank.util0 = I80F48::from_num(0.5);
        bank.util1 = I80F48::from_num(0.75);

        let interest = |v: f64| {
            bank.compute_interest_rate(I80F48::from_num(v))
                .to_num::<f64>()
        };
        let d = |a: f64, b: f64| (a - b).abs();

        // the points
        let eps = 0.0001;
        assert!(d(interest(-0.5), 1.0) <= eps);
        assert!(d(interest(0.0), 1.0) <= eps);
        assert!(d(interest(0.5), 3.0) <= eps);
        assert!(d(interest(0.75), 7.0) <= eps);
        assert!(d(interest(1.0), 13.0) <= eps);
        assert!(d(interest(1.5), 13.0) <= eps);

        // midpoints
        assert!(d(interest(0.25), 2.0) <= eps);
        assert!(d(interest((0.5 + 0.75) / 2.0), 5.0) <= eps);
        assert!(d(interest((0.75 + 1.0) / 2.0), 10.0) <= eps);

        // around the points
        let delta = 0.000001;
        assert!(d(interest(0.0 + delta), 1.0) <= eps);
        assert!(d(interest(0.5 - delta), 3.0) <= eps);
        assert!(d(interest(0.5 + delta), 3.0) <= eps);
        assert!(d(interest(0.75 - delta), 7.0) <= eps);
        assert!(d(interest(0.75 + delta), 7.0) <= eps);
        assert!(d(interest(1.0 - delta), 13.0) <= eps);
    }
}
