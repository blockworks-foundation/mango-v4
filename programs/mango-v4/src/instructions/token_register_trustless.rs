use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::instructions::INDEX_START;
use crate::state::*;
use crate::util::fill_from_str;

use crate::logs::{emit_stack, TokenMetaDataLogV2};

use crate::accounts_ix::*;

#[allow(clippy::too_many_arguments)]
pub fn token_register_trustless(
    ctx: Context<TokenRegisterTrustless>,
    token_index: TokenIndex,
    name: String,
) -> Result<()> {
    require_neq!(token_index, QUOTE_TOKEN_INDEX);
    require_neq!(token_index, TokenIndex::MAX);

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    {
        let mut group = ctx.accounts.group.load_mut()?;
        let week = 7 * 24 * 60 * 60;
        if now_ts >= group.fast_listing_interval_start + week {
            group.fast_listing_interval_start = now_ts / week * week;
            group.fast_listings_in_interval = 0;
        }
        group.fast_listings_in_interval += 1;
        require_gte!(
            group.allowed_fast_listings_per_interval,
            group.fast_listings_in_interval
        );
    }

    let net_borrow_limit_window_size_ts = 24 * 60 * 60u64;

    let mut bank = ctx.accounts.bank.load_init()?;
    *bank = Bank {
        group: ctx.accounts.group.key(),
        name: fill_from_str(&name)?,
        mint: ctx.accounts.mint.key(),
        vault: ctx.accounts.vault.key(),
        oracle: ctx.accounts.oracle.key(),
        oracle_config: OracleConfig {
            conf_filter: I80F48::from_num(1000.0), // effectively disabled
            max_staleness_slots: -1,
            reserved: [0; 72],
        },
        stable_price_model: StablePriceModel::default(),
        deposit_index: INDEX_START,
        borrow_index: INDEX_START,
        indexed_deposits: I80F48::ZERO,
        indexed_borrows: I80F48::ZERO,
        index_last_updated: now_ts,
        bank_rate_last_updated: now_ts,
        avg_utilization: I80F48::ZERO,
        // 10% daily adjustment at 0% or 100% utilization
        adjustment_factor: I80F48::from_num(0.004),
        util0: I80F48::from_num(0.5),
        rate0: I80F48::from_num(0.018),
        util1: I80F48::from_num(0.75),
        rate1: I80F48::from_num(0.05),
        max_rate: I80F48::from_num(0.5),
        collected_fees_native: I80F48::ZERO,
        loan_origination_fee_rate: I80F48::from_num(0.0020),
        loan_fee_rate: I80F48::from_num(0.005),
        maint_asset_weight: I80F48::from_num(0),
        init_asset_weight: I80F48::from_num(0),
        maint_liab_weight: I80F48::from_num(1.4), // 2.5x
        init_liab_weight: I80F48::from_num(1.8),  // 1.25x
        liquidation_fee: I80F48::from_num(0.05),
        platform_liquidation_fee: I80F48::from_num(0.05),
        dust: I80F48::ZERO,
        flash_loan_token_account_initial: u64::MAX,
        flash_loan_approved_amount: 0,
        token_index,
        bump: *ctx.bumps.get("bank").ok_or(MangoError::SomeError)?,
        mint_decimals: ctx.accounts.mint.decimals,
        bank_num: 0,
        min_vault_to_deposits_ratio: 0.2,
        net_borrow_limit_window_size_ts,
        last_net_borrows_window_start_ts: now_ts / net_borrow_limit_window_size_ts
            * net_borrow_limit_window_size_ts,
        net_borrow_limit_per_window_quote: 5_000_000_000, // $5k
        net_borrows_in_window: 0,
        borrow_weight_scale_start_quote: 5_000_000_000.0, // $5k
        deposit_weight_scale_start_quote: 5_000_000_000.0, // $5k
        reduce_only: 2,                                   // deposit-only
        force_close: 0,
        disable_asset_liquidation: 1,
        force_withdraw: 0,
        padding: Default::default(),
        fees_withdrawn: 0,
        token_conditional_swap_taker_fee_rate: 0.0,
        token_conditional_swap_maker_fee_rate: 0.0,
        flash_loan_swap_fee_rate: 0.0,
        interest_target_utilization: 0.5,
        interest_curve_scaling: 4.0,
        potential_serum_tokens: 0,
        maint_weight_shift_start: 0,
        maint_weight_shift_end: 0,
        maint_weight_shift_duration_inv: I80F48::ZERO,
        maint_weight_shift_asset_target: I80F48::ZERO,
        maint_weight_shift_liab_target: I80F48::ZERO,
        fallback_oracle: ctx.accounts.fallback_oracle.key(),
        deposit_limit: 0,
        zero_util_rate: I80F48::ZERO,
        collected_liquidation_fees: I80F48::ZERO,
        collected_collateral_fees: I80F48::ZERO,
        collateral_fee_per_day: 0.0, // TODO
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
        group_insurance_fund: 0,
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
