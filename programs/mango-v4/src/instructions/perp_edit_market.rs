use crate::{accounts_zerocopy::AccountInfoRef, error::MangoError, state::*};
use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::logs::PerpMarketMetaDataLog;

#[derive(Accounts)]
pub struct PerpEditMarket<'info> {
    pub group: AccountLoader<'info, Group>,
    // group <-> admin relation is checked at #1
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    /// The oracle account is optional and only used when reset_stable_price is set.
    ///
    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,
}

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
    maint_pnl_asset_weight_opt: Option<f32>,
    init_pnl_asset_weight_opt: Option<f32>,
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
) -> Result<()> {
    let group = ctx.accounts.group.load()?;

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    let mut require_group_admin = false;

    if let Some(oracle_config) = oracle_config_opt {
        perp_market.oracle_config = oracle_config.to_oracle_config();
        require_group_admin = true;
    };
    if let Some(oracle) = oracle_opt {
        perp_market.oracle = oracle;
        require_group_admin = true;
    }
    if reset_stable_price {
        require_keys_eq!(perp_market.oracle, ctx.accounts.oracle.key());
        let oracle_price = perp_market
            .oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?, None)?;
        perp_market.stable_price_model.reset_to_price(
            oracle_price.to_num(),
            Clock::get()?.unix_timestamp.try_into().unwrap(),
        );
        require_group_admin = true;
    }

    if let Some(maint_base_asset_weight) = maint_base_asset_weight_opt {
        perp_market.maint_base_asset_weight = I80F48::from_num(maint_base_asset_weight);
        require_group_admin = true;
    }
    if let Some(init_base_asset_weight) = init_base_asset_weight_opt {
        require_gte!(
            init_base_asset_weight,
            0.0,
            MangoError::InitAssetWeightCantBeNegative
        );

        let old_init_base_asset_weight = perp_market.init_base_asset_weight;
        perp_market.init_base_asset_weight = I80F48::from_num(init_base_asset_weight);

        // security admin can only reduce init_base_asset_weight
        if old_init_base_asset_weight < perp_market.init_base_asset_weight {
            require_group_admin = true;
        }
    }
    if let Some(maint_base_liab_weight) = maint_base_liab_weight_opt {
        perp_market.maint_base_liab_weight = I80F48::from_num(maint_base_liab_weight);
        require_group_admin = true;
    }
    if let Some(init_base_liab_weight) = init_base_liab_weight_opt {
        perp_market.init_base_liab_weight = I80F48::from_num(init_base_liab_weight);
        require_group_admin = true;
    }
    if let Some(maint_pnl_asset_weight) = maint_pnl_asset_weight_opt {
        perp_market.maint_pnl_asset_weight = I80F48::from_num(maint_pnl_asset_weight);
        require_group_admin = true;
    }
    if let Some(init_pnl_asset_weight) = init_pnl_asset_weight_opt {
        perp_market.init_pnl_asset_weight = I80F48::from_num(init_pnl_asset_weight);
        require_group_admin = true;
    }
    if let Some(base_liquidation_fee) = base_liquidation_fee_opt {
        perp_market.base_liquidation_fee = I80F48::from_num(base_liquidation_fee);
        require_group_admin = true;
    }

    if let Some(maker_fee) = maker_fee_opt {
        perp_market.maker_fee = I80F48::from_num(maker_fee);
        require_group_admin = true;
    }
    if let Some(taker_fee) = taker_fee_opt {
        perp_market.taker_fee = I80F48::from_num(taker_fee);
        require_group_admin = true;
    }

    if let Some(min_funding) = min_funding_opt {
        perp_market.min_funding = I80F48::from_num(min_funding);
        require_group_admin = true;
    }
    if let Some(max_funding) = max_funding_opt {
        perp_market.max_funding = I80F48::from_num(max_funding);
        require_group_admin = true;
    }
    if let Some(impact_quantity) = impact_quantity_opt {
        perp_market.impact_quantity = impact_quantity;
        require_group_admin = true;
    }
    if let Some(fee_penalty) = fee_penalty_opt {
        perp_market.fee_penalty = fee_penalty;
        require_group_admin = true;
    }

    if let Some(base_decimals) = base_decimals_opt {
        perp_market.base_decimals = base_decimals;
        require_group_admin = true;
    }

    if let Some(group_insurance_fund) = group_insurance_fund_opt {
        perp_market.set_elligible_for_group_insurance_fund(group_insurance_fund);
        require_group_admin = true;
    }

    if let Some(settle_fee_flat) = settle_fee_flat_opt {
        perp_market.settle_fee_flat = settle_fee_flat;
        require_group_admin = true;
    }
    if let Some(settle_fee_amount_threshold) = settle_fee_amount_threshold_opt {
        perp_market.settle_fee_amount_threshold = settle_fee_amount_threshold;
        require_group_admin = true;
    }
    if let Some(settle_fee_fraction_low_health) = settle_fee_fraction_low_health_opt {
        perp_market.settle_fee_fraction_low_health = settle_fee_fraction_low_health;
        require_group_admin = true;
    }

    if let Some(stable_price_delay_interval_seconds) = stable_price_delay_interval_seconds_opt {
        // Updating this makes the old delay values slightly inconsistent
        perp_market.stable_price_model.delay_interval_seconds = stable_price_delay_interval_seconds;
        require_group_admin = true;
    }
    if let Some(stable_price_delay_growth_limit) = stable_price_delay_growth_limit_opt {
        perp_market.stable_price_model.delay_growth_limit = stable_price_delay_growth_limit;
        require_group_admin = true;
    }
    if let Some(stable_price_growth_limit) = stable_price_growth_limit_opt {
        perp_market.stable_price_model.stable_growth_limit = stable_price_growth_limit;
        require_group_admin = true;
    }

    if let Some(settle_pnl_limit_factor_opt) = settle_pnl_limit_factor_opt {
        perp_market.settle_pnl_limit_factor = settle_pnl_limit_factor_opt;
        require_group_admin = true;
    }
    if let Some(settle_pnl_limit_window_size_ts) = settle_pnl_limit_window_size_ts_opt {
        perp_market.settle_pnl_limit_window_size_ts = settle_pnl_limit_window_size_ts;
        require_group_admin = true;
    }

    if let Some(reduce_only) = reduce_only_opt {
        perp_market.reduce_only = u8::from(reduce_only);

        // security admin can only enable reduce_only
        if !reduce_only {
            require_group_admin = true;
        }
    };

    if let Some(positive_pnl_liquidation_fee) = positive_pnl_liquidation_fee_opt {
        perp_market.positive_pnl_liquidation_fee = I80F48::from_num(positive_pnl_liquidation_fee);
        require_group_admin = true;
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

    emit!(PerpMarketMetaDataLog {
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
