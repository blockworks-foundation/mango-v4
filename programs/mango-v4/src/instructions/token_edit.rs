use anchor_lang::prelude::*;

use fixed::types::I80F48;

use crate::accounts_zerocopy::{AccountInfoRef, LoadMutZeroCopyRef};

use crate::error::MangoError;
use crate::state::*;

use crate::accounts_ix::*;
use crate::logs::{emit_stack, TokenMetaDataLogV2};
use crate::util::fill_from_str;

#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn token_edit(
    ctx: Context<TokenEdit>,
    oracle_opt: Option<Pubkey>,
    oracle_config_opt: Option<OracleConfigParams>,
    group_insurance_fund_opt: Option<bool>,
    interest_rate_params_opt: Option<InterestRateParams>,
    loan_fee_rate_opt: Option<f32>,
    loan_origination_fee_rate_opt: Option<f32>,
    maint_asset_weight_opt: Option<f32>,
    init_asset_weight_opt: Option<f32>,
    maint_liab_weight_opt: Option<f32>,
    init_liab_weight_opt: Option<f32>,
    liquidation_fee_opt: Option<f32>,
    stable_price_delay_interval_seconds_opt: Option<u32>,
    stable_price_delay_growth_limit_opt: Option<f32>,
    stable_price_growth_limit_opt: Option<f32>,
    min_vault_to_deposits_ratio_opt: Option<f64>,
    net_borrow_limit_per_window_quote_opt: Option<i64>,
    net_borrow_limit_window_size_ts_opt: Option<u64>,
    borrow_weight_scale_start_quote_opt: Option<f64>,
    deposit_weight_scale_start_quote_opt: Option<f64>,
    reset_stable_price: bool,
    reset_net_borrow_limit: bool,
    reduce_only_opt: Option<u8>,
    name_opt: Option<String>,
    force_close_opt: Option<bool>,
    token_conditional_swap_taker_fee_rate_opt: Option<f32>,
    token_conditional_swap_maker_fee_rate_opt: Option<f32>,
    flash_loan_swap_fee_rate_opt: Option<f32>,
    interest_curve_scaling_opt: Option<f32>,
    interest_target_utilization_opt: Option<f32>,
    maint_weight_shift_start_opt: Option<u64>,
    maint_weight_shift_end_opt: Option<u64>,
    maint_weight_shift_asset_target_opt: Option<f32>,
    maint_weight_shift_liab_target_opt: Option<f32>,
    maint_weight_shift_abort: bool,
    set_fallback_oracle: bool,
    deposit_limit_opt: Option<u64>,
    zero_util_rate: Option<f32>,
    platform_liquidation_fee: Option<f32>,
    disable_asset_liquidation_opt: Option<bool>,
    collateral_fee_per_day: Option<f32>,
    force_withdraw_opt: Option<bool>,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;

    let mut mint_info = ctx.accounts.mint_info.load_mut()?;
    mint_info.verify_banks_ais(ctx.remaining_accounts)?;

    let mut require_group_admin = false;
    for ai in ctx.remaining_accounts.iter() {
        let mut bank = ai.load_mut::<Bank>()?;

        if let Some(oracle_config) = oracle_config_opt.as_ref() {
            msg!(
                "Oracle config: old - conf_filter {:?}, max_staleness_slots {:?},  new - conf_filter {:?}, max_staleness_slots {:?}",
                bank.oracle_config.conf_filter,
                bank.oracle_config.max_staleness_slots,
                oracle_config.conf_filter,
                oracle_config.max_staleness_slots
            );
            bank.oracle_config = oracle_config.to_oracle_config();
            require_group_admin = true;
        };
        if let Some(oracle) = oracle_opt {
            msg!("Oracle: old - {:?}, new - {:?}", bank.oracle, oracle,);
            bank.oracle = oracle;
            mint_info.oracle = oracle;
            require_group_admin = true;
        }
        if set_fallback_oracle {
            msg!(
                "Fallback oracle old {:?}, new {:?}",
                bank.fallback_oracle,
                ctx.accounts.fallback_oracle.key()
            );
            check_is_valid_fallback_oracle(&AccountInfoRef::borrow(
                ctx.accounts.fallback_oracle.as_ref(),
            )?)?;
            bank.fallback_oracle = ctx.accounts.fallback_oracle.key();
            mint_info.fallback_oracle = ctx.accounts.fallback_oracle.key();
            require_group_admin = true;
        }
        if reset_stable_price {
            msg!("Stable price reset");
            require_keys_eq!(bank.oracle, ctx.accounts.oracle.key());
            let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
            let oracle_price =
                bank.oracle_price(&OracleAccountInfos::from_reader(oracle_ref), None)?;
            bank.stable_price_model.reset_to_price(
                oracle_price.to_num(),
                Clock::get()?.unix_timestamp.try_into().unwrap(),
            );
            require_group_admin = true;
        }

        if let Some(group_insurance_fund) = group_insurance_fund_opt {
            msg!(
                "Group insurance fund: old - {:?}, new - {:?}",
                mint_info.group_insurance_fund,
                group_insurance_fund
            );
            mint_info.group_insurance_fund = u8::from(group_insurance_fund);
            require_group_admin = true;
        };

        if let Some(ref interest_rate_params) = interest_rate_params_opt {
            // TODO: add a require! verifying relation between the parameters
            msg!("Interest rate params: old - adjustment_factor {:?}, util0 {:?}, rate0 {:?}, util1 {:?}, rate1 {:?}, max_rate {:?}, new - adjustment_factor {:?}, util0 {:?}, rate0 {:?}, util1 {:?}, rate1 {:?}, max_rate {:?}",
            bank.adjustment_factor,
            bank.util0,
            bank.rate0,
            bank.util1,
            bank.rate1,
            bank.max_rate,
            interest_rate_params.adjustment_factor,
            interest_rate_params.util0,
            interest_rate_params.rate0,
            interest_rate_params.util1,
            interest_rate_params.rate1,
            interest_rate_params.max_rate,
        );
            bank.adjustment_factor = I80F48::from_num(interest_rate_params.adjustment_factor);
            bank.util0 = I80F48::from_num(interest_rate_params.util0);
            bank.rate0 = I80F48::from_num(interest_rate_params.rate0);
            bank.util1 = I80F48::from_num(interest_rate_params.util1);
            bank.rate1 = I80F48::from_num(interest_rate_params.rate1);
            bank.max_rate = I80F48::from_num(interest_rate_params.max_rate);
            require_group_admin = true;
        }

        if let Some(loan_origination_fee_rate) = loan_origination_fee_rate_opt {
            msg!(
                "Loan origination fee rate: old - {:?}, new - {:?}",
                bank.loan_origination_fee_rate,
                loan_origination_fee_rate
            );
            bank.loan_origination_fee_rate = I80F48::from_num(loan_origination_fee_rate);
            require_group_admin = true;
        }
        if let Some(loan_fee_rate) = loan_fee_rate_opt {
            msg!(
                "Loan fee fee rate: old - {:?}, new - {:?}",
                bank.loan_fee_rate,
                loan_fee_rate
            );
            bank.loan_fee_rate = I80F48::from_num(loan_fee_rate);
            require_group_admin = true;
        }

        if let Some(maint_asset_weight) = maint_asset_weight_opt {
            msg!(
                "Maint asset weight: old - {:?}, new - {:?}",
                bank.maint_asset_weight,
                maint_asset_weight
            );
            bank.maint_asset_weight = I80F48::from_num(maint_asset_weight);
            require_group_admin = true;
        }
        if let Some(init_asset_weight) = init_asset_weight_opt {
            msg!(
                "Init asset weight: old - {:?}, new - {:?}",
                bank.init_asset_weight,
                init_asset_weight
            );
            require_gte!(
                init_asset_weight,
                0.0,
                MangoError::InitAssetWeightCantBeNegative
            );

            bank.init_asset_weight = I80F48::from_num(init_asset_weight);

            // The security admin is allowed to decrease the init collateral weight to zero,
            // but all other changes need to go through the full group admin.
            if init_asset_weight != 0.0 {
                require_group_admin = true;
            }
        }
        if let Some(maint_liab_weight) = maint_liab_weight_opt {
            msg!(
                "Maint liab weight: old - {:?}, new - {:?}",
                bank.maint_liab_weight,
                maint_liab_weight
            );
            bank.maint_liab_weight = I80F48::from_num(maint_liab_weight);
            require_group_admin = true;
        }
        if let Some(init_liab_weight) = init_liab_weight_opt {
            msg!(
                "Init liab weight: old - {:?}, new - {:?}",
                bank.init_liab_weight,
                init_liab_weight
            );
            bank.init_liab_weight = I80F48::from_num(init_liab_weight);
            require_group_admin = true;
        }
        if let Some(liquidation_fee) = liquidation_fee_opt {
            msg!(
                "Liquidation fee: old - {:?}, new - {:?}",
                bank.liquidation_fee,
                liquidation_fee
            );
            bank.liquidation_fee = I80F48::from_num(liquidation_fee);
            require_group_admin = true;
        }

        if let Some(stable_price_delay_interval_seconds) = stable_price_delay_interval_seconds_opt {
            msg!(
                "Stable price delay interval seconds: old - {:?}, new - {:?}",
                bank.stable_price_model.delay_interval_seconds,
                stable_price_delay_interval_seconds
            );
            // Updating this makes the old delay values slightly inconsistent
            bank.stable_price_model.delay_interval_seconds = stable_price_delay_interval_seconds;
            require_group_admin = true;
        }
        if let Some(stable_price_delay_growth_limit) = stable_price_delay_growth_limit_opt {
            msg!(
                "Stable price delay growth limit: old - {:?}, new - {:?}",
                bank.stable_price_model.delay_growth_limit,
                stable_price_delay_growth_limit
            );
            bank.stable_price_model.delay_growth_limit = stable_price_delay_growth_limit;
            require_group_admin = true;
        }
        if let Some(stable_price_growth_limit) = stable_price_growth_limit_opt {
            msg!(
                "Stable price growth limit: old - {:?}, new - {:?}",
                bank.stable_price_model.stable_growth_limit,
                stable_price_growth_limit
            );
            bank.stable_price_model.stable_growth_limit = stable_price_growth_limit;
            require_group_admin = true;
        }

        if let Some(min_vault_to_deposits_ratio) = min_vault_to_deposits_ratio_opt {
            msg!(
                "Min vault to deposits ratio: old - {:?}, new - {:?}",
                bank.min_vault_to_deposits_ratio,
                min_vault_to_deposits_ratio
            );
            bank.min_vault_to_deposits_ratio = min_vault_to_deposits_ratio;
            require_group_admin = true;
        }
        if let Some(net_borrow_limit_per_window_quote) = net_borrow_limit_per_window_quote_opt {
            msg!(
                "Net borrow limit per window quote: old - {:?}, new - {:?}",
                bank.net_borrow_limit_per_window_quote,
                net_borrow_limit_per_window_quote
            );
            bank.net_borrow_limit_per_window_quote = net_borrow_limit_per_window_quote;
            require_group_admin = true;
        }
        if let Some(net_borrow_limit_window_size_ts) = net_borrow_limit_window_size_ts_opt {
            msg!(
                "Net borrow limit window size ts: old - {:?}, new - {:?}",
                bank.net_borrow_limit_window_size_ts,
                net_borrow_limit_window_size_ts
            );
            bank.net_borrow_limit_window_size_ts = net_borrow_limit_window_size_ts;
            require_group_admin = true;
        }
        if reset_net_borrow_limit {
            msg!("Net borrow limit reset");
            bank.net_borrows_in_window = 0;
            bank.last_net_borrows_window_start_ts = 0;
            require_group_admin = true;
        }

        if let Some(borrow_weight_scale_start_quote) = borrow_weight_scale_start_quote_opt {
            msg!(
                "Borrow weight scale start quote: old - {:?}, new - {:?}",
                bank.borrow_weight_scale_start_quote,
                borrow_weight_scale_start_quote
            );
            bank.borrow_weight_scale_start_quote = borrow_weight_scale_start_quote;
            require_group_admin = true;
        }
        if let Some(deposit_weight_scale_start_quote) = deposit_weight_scale_start_quote_opt {
            msg!(
                "Deposit weight scale start quote: old - {:?}, new - {:?}",
                bank.deposit_weight_scale_start_quote,
                deposit_weight_scale_start_quote
            );
            bank.deposit_weight_scale_start_quote = deposit_weight_scale_start_quote;
            require_group_admin = true;
        }

        if let Some(reduce_only) = reduce_only_opt {
            msg!(
                "Reduce only: old - {:?}, new - {:?}",
                bank.reduce_only,
                reduce_only
            );

            // security admin can only make it stricter
            // anything that makes it less strict, should require admin
            if reduce_only == 0 || (reduce_only == 2 && bank.reduce_only == 1) {
                require_group_admin = true;
            }
            bank.reduce_only = reduce_only;
        };

        if let Some(name) = name_opt.as_ref() {
            msg!("Name: old - {:?}, new - {:?}", bank.name, name);
            bank.name = fill_from_str(&name)?;
            require_group_admin = true;
        };

        if let Some(force_close) = force_close_opt {
            if force_close {
                require!(bank.reduce_only > 0, MangoError::SomeError);
            }
            msg!(
                "Force close: old - {:?}, new - {:?}",
                bank.force_close,
                u8::from(force_close)
            );
            bank.force_close = u8::from(force_close);
            require_group_admin = true;
        };

        if let Some(fee_rate) = token_conditional_swap_taker_fee_rate_opt {
            msg!(
                "Token conditional swap taker fee fraction old {:?}, new {:?}",
                bank.token_conditional_swap_taker_fee_rate,
                fee_rate
            );
            require_gte!(fee_rate, 0.0); // values <0 are not currently supported
            bank.token_conditional_swap_taker_fee_rate = fee_rate;
            require_group_admin = true;
        }
        if let Some(fee_rate) = token_conditional_swap_maker_fee_rate_opt {
            msg!(
                "Token conditional swap maker fee fraction old {:?}, new {:?}",
                bank.token_conditional_swap_maker_fee_rate,
                fee_rate
            );
            require_gte!(fee_rate, 0.0); // values <0 are not currently supported
            bank.token_conditional_swap_maker_fee_rate = fee_rate;
            require_group_admin = true;
        }

        if let Some(fee_rate) = flash_loan_swap_fee_rate_opt {
            msg!(
                "Flash loan swap fee fraction old {:?}, new {:?}",
                bank.flash_loan_swap_fee_rate,
                fee_rate
            );
            require_gte!(fee_rate, 0.0); // values <0 are not currently supported
            bank.flash_loan_swap_fee_rate = fee_rate;
            require_group_admin = true;
        }

        if let Some(interest_curve_scaling) = interest_curve_scaling_opt {
            msg!(
                "Interest curve scaling old {:?}, new {:?}",
                bank.interest_curve_scaling,
                interest_curve_scaling
            );
            require_gte!(interest_curve_scaling, 1.0);
            bank.interest_curve_scaling = interest_curve_scaling.into();
            require_group_admin = true;
        }
        if let Some(interest_target_utilization) = interest_target_utilization_opt {
            msg!(
                "Interest target utilization old {:?}, new {:?}",
                bank.interest_target_utilization,
                interest_target_utilization
            );
            require_gte!(interest_target_utilization, 0.0);
            bank.interest_target_utilization = interest_target_utilization;
            require_group_admin = true;
        }

        if maint_weight_shift_abort {
            let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
            let (maint_asset_weight, maint_liab_weight) = bank.maint_weights(now_ts);
            bank.maint_asset_weight = maint_asset_weight;
            bank.maint_liab_weight = maint_liab_weight;
            bank.maint_weight_shift_start = 0;
            bank.maint_weight_shift_end = 0;
            bank.maint_weight_shift_duration_inv = I80F48::ZERO;
            bank.maint_weight_shift_asset_target = I80F48::ZERO;
            bank.maint_weight_shift_liab_target = I80F48::ZERO;
            msg!(
                "Maint weight shift aborted, current maint weights asset {} liab {}",
                maint_asset_weight,
                maint_liab_weight,
            );
            // Allow execution by group admin or security admin
        }
        if let Some(maint_weight_shift_start) = maint_weight_shift_start_opt {
            msg!(
                "Maint weight shift start old {:?}, new {:?}",
                bank.maint_weight_shift_start,
                maint_weight_shift_start
            );
            bank.maint_weight_shift_start = maint_weight_shift_start;
            require_group_admin = true;
        }
        if let Some(maint_weight_shift_end) = maint_weight_shift_end_opt {
            msg!(
                "Maint weight shift end old {:?}, new {:?}",
                bank.maint_weight_shift_end,
                maint_weight_shift_end
            );
            bank.maint_weight_shift_end = maint_weight_shift_end;
            require_group_admin = true;
        }
        if let Some(maint_weight_shift_asset_target) = maint_weight_shift_asset_target_opt {
            msg!(
                "Maint weight shift asset target old {:?}, new {:?}",
                bank.maint_weight_shift_asset_target,
                maint_weight_shift_asset_target
            );
            bank.maint_weight_shift_asset_target =
                I80F48::from_num(maint_weight_shift_asset_target);
            require_group_admin = true;
        }
        if let Some(maint_weight_shift_liab_target) = maint_weight_shift_liab_target_opt {
            msg!(
                "Maint weight shift liab target old {:?}, new {:?}",
                bank.maint_weight_shift_liab_target,
                maint_weight_shift_liab_target
            );
            bank.maint_weight_shift_liab_target = I80F48::from_num(maint_weight_shift_liab_target);
            require_group_admin = true;
        }
        if maint_weight_shift_start_opt.is_some() || maint_weight_shift_end_opt.is_some() {
            let was_enabled = bank.maint_weight_shift_duration_inv.is_positive();
            if bank.maint_weight_shift_end <= bank.maint_weight_shift_start {
                bank.maint_weight_shift_duration_inv = I80F48::ZERO;
            } else {
                bank.maint_weight_shift_duration_inv = I80F48::ONE
                    / I80F48::from(bank.maint_weight_shift_end - bank.maint_weight_shift_start);
            }
            msg!(
                "Maint weight shift enabled old {}, new {}",
                was_enabled,
                bank.maint_weight_shift_duration_inv.is_positive(),
            );
        }

        if let Some(deposit_limit) = deposit_limit_opt {
            msg!(
                "Deposit limit old {:?}, new {:?}",
                bank.deposit_limit,
                deposit_limit
            );
            bank.deposit_limit = deposit_limit;
            require_group_admin = true;
        }

        if let Some(zero_util_rate) = zero_util_rate {
            msg!(
                "Zero utilization rate old {:?}, new {:?}",
                bank.zero_util_rate,
                zero_util_rate
            );
            bank.zero_util_rate = I80F48::from_num(zero_util_rate);
            require_group_admin = true;
        }

        if let Some(platform_liquidation_fee) = platform_liquidation_fee {
            msg!(
                "Platform liquidation fee old {:?}, new {:?}",
                bank.platform_liquidation_fee,
                platform_liquidation_fee
            );
            bank.platform_liquidation_fee = I80F48::from_num(platform_liquidation_fee);
            if platform_liquidation_fee != 0.0 {
                require_group_admin = true;
            }
        }

        if let Some(collateral_fee_per_day) = collateral_fee_per_day {
            msg!(
                "Collateral fee per day old {:?}, new {:?}",
                bank.collateral_fee_per_day,
                collateral_fee_per_day
            );
            bank.collateral_fee_per_day = collateral_fee_per_day;
            if collateral_fee_per_day != 0.0 {
                require_group_admin = true;
            }
        }

        if let Some(disable_asset_liquidation) = disable_asset_liquidation_opt {
            msg!(
                "Asset liquidation disabled old {:?}, new {:?}",
                bank.disable_asset_liquidation,
                disable_asset_liquidation
            );
            bank.disable_asset_liquidation = u8::from(disable_asset_liquidation);
            require_group_admin = true;
        }

        if let Some(force_withdraw) = force_withdraw_opt {
            msg!(
                "Force withdraw old {:?}, new {:?}",
                bank.force_withdraw,
                force_withdraw
            );
            bank.force_withdraw = u8::from(force_withdraw);
            require_group_admin = true;
        }
    }

    // account constraint #1
    if require_group_admin {
        require!(
            group.admin == ctx.accounts.admin.key(),
            MangoError::SomeError
        );
    } else {
        require!(
            group.admin == ctx.accounts.admin.key()
                || group.security_admin == ctx.accounts.admin.key(),
            MangoError::SomeError
        );
    }

    // Assumes that there is at least one bank
    let bank = ctx.remaining_accounts.first().unwrap().load_mut::<Bank>()?;
    bank.verify()?;

    emit_stack(TokenMetaDataLogV2 {
        mango_group: ctx.accounts.group.key(),
        mint: mint_info.mint.key(),
        token_index: bank.token_index,
        mint_decimals: bank.mint_decimals,
        oracle: mint_info.oracle.key(),
        fallback_oracle: ctx.accounts.fallback_oracle.key(),
        mint_info: ctx.accounts.mint_info.key(),
    });

    Ok(())
}
