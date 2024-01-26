use crate::util::fill_from_str;
use crate::{accounts_zerocopy::AccountInfoRef, error::MangoError, state::*};
use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::logs::{emit_stack, PerpMarketMetaDataLog};

#[allow(clippy::too_many_arguments)]
pub fn perp_edit_market(
    ctx: Context<PerpEditMarket>,
    oracle_opt: Option<Pubkey>,
    oracle_config_opt: Option<OracleConfigParams>,
    base_decimals_opt: Option<u8>,
    maint_base_asset_weight_opt: Option<f32>,
    init_base_asset_weight_opt: Option<f32>,
    maint_base_liab_weight_opt: Option<f32>,
    init_base_liab_weight_opt: Option<f32>,
    maint_overall_asset_weight_opt: Option<f32>,
    init_overall_asset_weight_opt: Option<f32>,
    base_liquidation_fee_opt: Option<f32>,
    maker_fee_opt: Option<f32>,
    taker_fee_opt: Option<f32>,
    min_funding_opt: Option<f32>,
    max_funding_opt: Option<f32>,
    impact_quantity_opt: Option<i64>,
    group_insurance_fund_opt: Option<bool>,
    fee_penalty_opt: Option<f32>,
    settle_fee_flat_opt: Option<f32>,
    settle_fee_amount_threshold_opt: Option<f32>,
    settle_fee_fraction_low_health_opt: Option<f32>,
    stable_price_delay_interval_seconds_opt: Option<u32>,
    stable_price_delay_growth_limit_opt: Option<f32>,
    stable_price_growth_limit_opt: Option<f32>,
    settle_pnl_limit_factor_opt: Option<f32>,
    settle_pnl_limit_window_size_ts_opt: Option<u64>,
    reduce_only_opt: Option<bool>,
    reset_stable_price: bool,
    positive_pnl_liquidation_fee_opt: Option<f32>,
    name_opt: Option<String>,
    force_close_opt: Option<bool>,
    platform_liquidation_fee_opt: Option<f32>,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    let mut require_group_admin = false;

    if let Some(oracle_config) = oracle_config_opt {
        msg!(
        "Oracle config: old - conf_filter {:?}, max_staleness_slots {:?},  new - conf_filter {:?}, max_staleness_slots {:?}",
        perp_market.oracle_config.conf_filter,
        perp_market.oracle_config.max_staleness_slots,
        oracle_config.conf_filter,
        oracle_config.max_staleness_slots
    );
        perp_market.oracle_config = oracle_config.to_oracle_config();
        require_group_admin = true;
    };
    if let Some(oracle) = oracle_opt {
        msg!("Oracle: old - {:?}, new - {:?}", perp_market.oracle, oracle);
        perp_market.oracle = oracle;
        require_group_admin = true;
    }
    if reset_stable_price {
        msg!("Stable price reset");
        require_keys_eq!(perp_market.oracle, ctx.accounts.oracle.key());
        let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
        let oracle_price =
            perp_market.oracle_price(&OracleAccountInfos::from_reader(oracle_ref), None)?;
        perp_market.stable_price_model.reset_to_price(
            oracle_price.to_num(),
            Clock::get()?.unix_timestamp.try_into().unwrap(),
        );
        require_group_admin = true;
    }

    if let Some(maint_base_asset_weight) = maint_base_asset_weight_opt {
        msg!(
            "Maint base asset weight: old - {:?}, new - {:?}",
            perp_market.maint_base_asset_weight,
            maint_base_asset_weight
        );
        perp_market.maint_base_asset_weight = I80F48::from_num(maint_base_asset_weight);
        require_group_admin = true;
    }
    if let Some(init_base_asset_weight) = init_base_asset_weight_opt {
        msg!(
            "Init base asset weight: old - {:?}, new - {:?}",
            perp_market.init_base_asset_weight,
            init_base_asset_weight
        );
        require_gte!(
            init_base_asset_weight,
            0.0,
            MangoError::InitAssetWeightCantBeNegative
        );

        perp_market.init_base_asset_weight = I80F48::from_num(init_base_asset_weight);
        require_group_admin = true;
    }
    if let Some(maint_base_liab_weight) = maint_base_liab_weight_opt {
        msg!(
            "Maint base liab weight: old - {:?}, new - {:?}",
            perp_market.maint_base_liab_weight,
            maint_base_liab_weight
        );
        perp_market.maint_base_liab_weight = I80F48::from_num(maint_base_liab_weight);
        require_group_admin = true;
    }
    if let Some(init_base_liab_weight) = init_base_liab_weight_opt {
        msg!(
            "Init base liab weight: old - {:?}, new - {:?}",
            perp_market.init_base_liab_weight,
            init_base_liab_weight
        );
        perp_market.init_base_liab_weight = I80F48::from_num(init_base_liab_weight);
        require_group_admin = true;
    }
    if let Some(maint_overall_asset_weight) = maint_overall_asset_weight_opt {
        msg!(
            "Maint pnl asset weight: old - {:?}, new - {:?}",
            perp_market.maint_overall_asset_weight,
            maint_overall_asset_weight
        );
        perp_market.maint_overall_asset_weight = I80F48::from_num(maint_overall_asset_weight);
        require_group_admin = true;
    }
    if let Some(init_overall_asset_weight) = init_overall_asset_weight_opt {
        msg!(
            "Init pnl asset weight: old - {:?}, new - {:?}",
            perp_market.init_overall_asset_weight,
            init_overall_asset_weight
        );
        perp_market.init_overall_asset_weight = I80F48::from_num(init_overall_asset_weight);

        // The security admin is allowed to disable init collateral contributions,
        // but all other changes need to go through the full group admin.
        if init_overall_asset_weight != 0.0 {
            require_group_admin = true;
        }
    }
    if let Some(base_liquidation_fee) = base_liquidation_fee_opt {
        msg!(
            "Base liquidation fee: old - {:?}, new - {:?}",
            perp_market.base_liquidation_fee,
            base_liquidation_fee
        );
        perp_market.base_liquidation_fee = I80F48::from_num(base_liquidation_fee);
        require_group_admin = true;
    }

    if let Some(maker_fee) = maker_fee_opt {
        msg!(
            "Maker fee: old - {:?}, new - {:?}",
            perp_market.maker_fee,
            maker_fee
        );
        perp_market.maker_fee = I80F48::from_num(maker_fee);
        require_group_admin = true;
    }
    if let Some(taker_fee) = taker_fee_opt {
        msg!(
            "Taker fee: old - {:?}, new - {:?}",
            perp_market.taker_fee,
            taker_fee
        );
        perp_market.taker_fee = I80F48::from_num(taker_fee);
        require_group_admin = true;
    }

    if let Some(min_funding) = min_funding_opt {
        msg!(
            "Min funding: old - {:?}, new - {:?}",
            perp_market.min_funding,
            min_funding
        );
        perp_market.min_funding = I80F48::from_num(min_funding);
        require_group_admin = true;
    }
    if let Some(max_funding) = max_funding_opt {
        msg!(
            "Max funding: old - {:?}, new - {:?}",
            perp_market.max_funding,
            max_funding
        );
        perp_market.max_funding = I80F48::from_num(max_funding);
        require_group_admin = true;
    }
    if let Some(impact_quantity) = impact_quantity_opt {
        msg!(
            "Impact quantity: old - {:?}, new - {:?}",
            perp_market.impact_quantity,
            impact_quantity
        );
        perp_market.impact_quantity = impact_quantity;
        require_group_admin = true;
    }
    if let Some(fee_penalty) = fee_penalty_opt {
        msg!(
            "Fee penalty: old - {:?}, new - {:?}",
            perp_market.fee_penalty,
            fee_penalty
        );
        perp_market.fee_penalty = fee_penalty;
        require_group_admin = true;
    }

    if let Some(base_decimals) = base_decimals_opt {
        msg!(
            "Base decimals: old - {:?}, new - {:?}",
            perp_market.base_decimals,
            base_decimals
        );
        perp_market.base_decimals = base_decimals;
        require_group_admin = true;
    }

    if let Some(group_insurance_fund) = group_insurance_fund_opt {
        msg!(
            "Group insurance fund: old - {:?}, new - {:?}",
            perp_market.group_insurance_fund,
            group_insurance_fund
        );
        perp_market.set_elligible_for_group_insurance_fund(group_insurance_fund);
        require_group_admin = true;
    }

    if let Some(settle_fee_flat) = settle_fee_flat_opt {
        msg!(
            "Settle fee flat: old - {:?}, new - {:?}",
            perp_market.settle_fee_flat,
            settle_fee_flat
        );
        perp_market.settle_fee_flat = settle_fee_flat;
        require_group_admin = true;
    }
    if let Some(settle_fee_amount_threshold) = settle_fee_amount_threshold_opt {
        msg!(
            "Settle fee amount threshold: old - {:?}, new - {:?}",
            perp_market.settle_fee_amount_threshold,
            settle_fee_amount_threshold
        );
        perp_market.settle_fee_amount_threshold = settle_fee_amount_threshold;
        require_group_admin = true;
    }
    if let Some(settle_fee_fraction_low_health) = settle_fee_fraction_low_health_opt {
        msg!(
            "Settle fee fraction low health: old - {:?}, new - {:?}",
            perp_market.settle_fee_fraction_low_health,
            settle_fee_fraction_low_health
        );
        perp_market.settle_fee_fraction_low_health = settle_fee_fraction_low_health;
        require_group_admin = true;
    }

    if let Some(stable_price_delay_interval_seconds) = stable_price_delay_interval_seconds_opt {
        // Updating this makes the old delay values slightly inconsistent
        msg!(
            "Stable price delay interval seconds: old - {:?}, new - {:?}",
            perp_market.stable_price_model.delay_interval_seconds,
            stable_price_delay_interval_seconds
        );
        perp_market.stable_price_model.delay_interval_seconds = stable_price_delay_interval_seconds;
        require_group_admin = true;
    }
    if let Some(stable_price_delay_growth_limit) = stable_price_delay_growth_limit_opt {
        msg!(
            "Stable price delay growth limit: old - {:?}, new - {:?}",
            perp_market.stable_price_model.delay_growth_limit,
            stable_price_delay_growth_limit
        );
        perp_market.stable_price_model.delay_growth_limit = stable_price_delay_growth_limit;
        require_group_admin = true;
    }
    if let Some(stable_price_growth_limit) = stable_price_growth_limit_opt {
        msg!(
            "Stable price growth limit: old - {:?}, new - {:?}",
            perp_market.stable_price_model.stable_growth_limit,
            stable_price_growth_limit
        );
        perp_market.stable_price_model.stable_growth_limit = stable_price_growth_limit;
        require_group_admin = true;
    }

    if let Some(settle_pnl_limit_factor) = settle_pnl_limit_factor_opt {
        msg!(
            "Settle pnl limit factor: old - {:?}, new - {:?}",
            perp_market.settle_pnl_limit_factor,
            settle_pnl_limit_factor
        );
        perp_market.settle_pnl_limit_factor = settle_pnl_limit_factor;
        require_group_admin = true;
    }
    if let Some(settle_pnl_limit_window_size_ts) = settle_pnl_limit_window_size_ts_opt {
        msg!(
            "Settle pnl limit window size ts: old - {:?}, new - {:?}",
            perp_market.settle_pnl_limit_window_size_ts,
            settle_pnl_limit_window_size_ts
        );
        perp_market.settle_pnl_limit_window_size_ts = settle_pnl_limit_window_size_ts;
        require_group_admin = true;
    }

    if let Some(reduce_only) = reduce_only_opt {
        msg!(
            "Reduce only: old - {:?}, new - {:?}",
            perp_market.reduce_only,
            u8::from(reduce_only)
        );
        perp_market.reduce_only = u8::from(reduce_only);

        // security admin can only enable reduce_only
        if !reduce_only {
            require_group_admin = true;
        }
    };

    if let Some(positive_pnl_liquidation_fee) = positive_pnl_liquidation_fee_opt {
        msg!(
            "Positive pnl liquidation fee: old - {:?}, new - {:?}",
            perp_market.positive_pnl_liquidation_fee,
            positive_pnl_liquidation_fee
        );
        perp_market.positive_pnl_liquidation_fee = I80F48::from_num(positive_pnl_liquidation_fee);
        require_group_admin = true;
    }

    if let Some(name) = name_opt.as_ref() {
        msg!("Name: old - {:?}, new - {:?}", perp_market.name, name);
        perp_market.name = fill_from_str(&name)?;
        require_group_admin = true;
    };

    if let Some(force_close) = force_close_opt {
        if force_close {
            require!(perp_market.reduce_only > 0, MangoError::SomeError);
        }
        msg!(
            "Force close: old - {:?}, new - {:?}",
            perp_market.force_close,
            u8::from(force_close)
        );
        perp_market.force_close = u8::from(force_close);
        require_group_admin = true;
    };

    if let Some(platform_liquidation_fee) = platform_liquidation_fee_opt {
        msg!(
            "Platform liquidation fee: old - {:?}, new - {:?}",
            perp_market.platform_liquidation_fee,
            platform_liquidation_fee
        );
        perp_market.platform_liquidation_fee = I80F48::from_num(platform_liquidation_fee);
        require_group_admin = true;
    };

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

    emit_stack(PerpMarketMetaDataLog {
        mango_group: ctx.accounts.group.key(),
        perp_market: ctx.accounts.perp_market.key(),
        perp_market_index: perp_market.perp_market_index,
        base_decimals: perp_market.base_decimals,
        base_lot_size: perp_market.base_lot_size,
        quote_lot_size: perp_market.quote_lot_size,
        oracle: perp_market.oracle.key(),
    });

    Ok(())
}
