use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::state::*;
use crate::util::fill_from_str;

use crate::logs::{emit_stack, TokenMetaDataLogV2};

pub const INDEX_START: I80F48 = I80F48::from_bits(1_000_000 * I80F48::ONE.to_bits());

use crate::accounts_ix::*;

#[allow(clippy::too_many_arguments)]
pub fn token_register(
    ctx: Context<TokenRegister>,
    token_index: TokenIndex,
    name: String,
    oracle_config: OracleConfigParams,
    interest_rate_params: InterestRateParams,
    loan_fee_rate: f32,
    loan_origination_fee_rate: f32,
    maint_asset_weight: f32,
    init_asset_weight: f32,
    maint_liab_weight: f32,
    init_liab_weight: f32,
    liquidation_fee: f32,
    stable_price_delay_interval_seconds: u32,
    stable_price_delay_growth_limit: f32,
    stable_price_growth_limit: f32,
    min_vault_to_deposits_ratio: f64,
    net_borrow_limit_window_size_ts: u64,
    net_borrow_limit_per_window_quote: i64,
    borrow_weight_scale_start_quote: f64,
    deposit_weight_scale_start_quote: f64,
    reduce_only: u8,
    token_conditional_swap_taker_fee_rate: f32,
    token_conditional_swap_maker_fee_rate: f32,
    flash_loan_swap_fee_rate: f32,
    interest_curve_scaling: f32,
    interest_target_utilization: f32,
    group_insurance_fund: bool,
    deposit_limit: u64,
    zero_util_rate: f32,
    platform_liquidation_fee: f32,
    disable_asset_liquidation: bool,
    collateral_fee_per_day: f32,
) -> Result<()> {
    // Require token 0 to be in the insurance token
    if token_index == INSURANCE_TOKEN_INDEX {
        require_keys_eq!(
            ctx.accounts.group.load()?.insurance_mint,
            ctx.accounts.mint.key()
        );
    }
    require_neq!(token_index, TokenIndex::MAX);

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let mut bank = ctx.accounts.bank.load_init()?;
    *bank = Bank {
        group: ctx.accounts.group.key(),
        name: fill_from_str(&name)?,
        mint: ctx.accounts.mint.key(),
        vault: ctx.accounts.vault.key(),
        oracle: ctx.accounts.oracle.key(),
        deposit_index: INDEX_START,
        borrow_index: INDEX_START,
        indexed_deposits: I80F48::ZERO,
        indexed_borrows: I80F48::ZERO,
        index_last_updated: now_ts,
        bank_rate_last_updated: now_ts,
        // TODO: add a require! verifying relation between the parameters
        avg_utilization: I80F48::ZERO,
        adjustment_factor: I80F48::from_num(interest_rate_params.adjustment_factor),
        util0: I80F48::from_num(interest_rate_params.util0),
        rate0: I80F48::from_num(interest_rate_params.rate0),
        util1: I80F48::from_num(interest_rate_params.util1),
        rate1: I80F48::from_num(interest_rate_params.rate1),
        max_rate: I80F48::from_num(interest_rate_params.max_rate),
        collected_fees_native: I80F48::ZERO,
        loan_origination_fee_rate: I80F48::from_num(loan_origination_fee_rate),
        loan_fee_rate: I80F48::from_num(loan_fee_rate),
        maint_asset_weight: I80F48::from_num(maint_asset_weight),
        init_asset_weight: I80F48::from_num(init_asset_weight),
        maint_liab_weight: I80F48::from_num(maint_liab_weight),
        init_liab_weight: I80F48::from_num(init_liab_weight),
        liquidation_fee: I80F48::from_num(liquidation_fee),
        dust: I80F48::ZERO,
        flash_loan_token_account_initial: u64::MAX,
        flash_loan_approved_amount: 0,
        token_index,
        bump: *ctx.bumps.get("bank").ok_or(MangoError::SomeError)?,
        mint_decimals: ctx.accounts.mint.decimals,
        bank_num: 0,
        oracle_config: oracle_config.to_oracle_config(),
        stable_price_model: StablePriceModel {
            delay_interval_seconds: stable_price_delay_interval_seconds,
            delay_growth_limit: stable_price_delay_growth_limit,
            stable_growth_limit: stable_price_growth_limit,
            ..StablePriceModel::default()
        },
        min_vault_to_deposits_ratio,
        net_borrow_limit_window_size_ts,
        last_net_borrows_window_start_ts: now_ts / net_borrow_limit_window_size_ts
            * net_borrow_limit_window_size_ts,
        net_borrow_limit_per_window_quote,
        net_borrows_in_window: 0,
        borrow_weight_scale_start_quote,
        deposit_weight_scale_start_quote,
        reduce_only,
        force_close: 0,
        disable_asset_liquidation: u8::from(disable_asset_liquidation),
        force_withdraw: 0,
        padding: Default::default(),
        fees_withdrawn: 0,
        token_conditional_swap_taker_fee_rate,
        token_conditional_swap_maker_fee_rate,
        flash_loan_swap_fee_rate: flash_loan_swap_fee_rate,
        interest_target_utilization,
        interest_curve_scaling: interest_curve_scaling.into(),
        potential_serum_tokens: 0,
        maint_weight_shift_start: 0,
        maint_weight_shift_end: 0,
        maint_weight_shift_duration_inv: I80F48::ZERO,
        maint_weight_shift_asset_target: I80F48::ZERO,
        maint_weight_shift_liab_target: I80F48::ZERO,
        fallback_oracle: ctx.accounts.fallback_oracle.key(),
        deposit_limit,
        zero_util_rate: I80F48::from_num(zero_util_rate),
        platform_liquidation_fee: I80F48::from_num(platform_liquidation_fee),
        collected_liquidation_fees: I80F48::ZERO,
        collected_collateral_fees: I80F48::ZERO,
        collateral_fee_per_day,
        reserved: [0; 1900],
    };

    let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
    if let Ok(oracle_price) = bank.oracle_price(&OracleAccountInfos::from_reader(oracle_ref), None)
    {
        bank.stable_price_model
            .reset_to_price(oracle_price.to_num(), now_ts);
    } else {
        bank.stable_price_model.reset_on_nonzero_price = 1;
    }

    bank.verify()?;
    check_is_valid_fallback_oracle(&AccountInfoRef::borrow(
        ctx.accounts.fallback_oracle.as_ref(),
    )?)?;

    let mut mint_info = ctx.accounts.mint_info.load_init()?;
    *mint_info = MintInfo {
        group: ctx.accounts.group.key(),
        token_index,
        group_insurance_fund: if group_insurance_fund { 1 } else { 0 },
        padding1: Default::default(),
        mint: ctx.accounts.mint.key(),
        banks: Default::default(),
        vaults: Default::default(),
        oracle: ctx.accounts.oracle.key(),
        fallback_oracle: ctx.accounts.fallback_oracle.key(),
        registration_time: Clock::get()?.unix_timestamp.try_into().unwrap(),
        reserved: [0; 2528],
    };

    mint_info.banks[0] = ctx.accounts.bank.key();
    mint_info.vaults[0] = ctx.accounts.vault.key();

    emit_stack(TokenMetaDataLogV2 {
        mango_group: ctx.accounts.group.key(),
        mint: ctx.accounts.mint.key(),
        token_index,
        mint_decimals: ctx.accounts.mint.decimals,
        oracle: ctx.accounts.oracle.key(),
        fallback_oracle: ctx.accounts.fallback_oracle.key(),
        mint_info: ctx.accounts.mint_info.key(),
    });

    Ok(())
}
