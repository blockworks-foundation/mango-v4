use crate::{accounts_zerocopy::AccountInfoRef, error::MangoError, state::*};
use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::logs::PerpMarketMetaDataLog;

#[derive(Accounts)]
pub struct PerpEditMarket<'info> {
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

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
    liquidation_fee_opt: Option<f32>,
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
) -> Result<()> {
    let group = ctx.accounts.group.load()?;

    if oracle_opt.is_none()
        && oracle_config_opt.is_none()
        && base_decimals_opt.is_none()
        && maint_base_asset_weight_opt.is_none()
        && init_base_asset_weight_opt.is_none()
        && maint_base_liab_weight_opt.is_none()
        && init_base_liab_weight_opt.is_none()
        && maint_pnl_asset_weight_opt.is_none()
        && init_pnl_asset_weight_opt.is_none()
        && liquidation_fee_opt.is_none()
        && maker_fee_opt.is_none()
        && taker_fee_opt.is_none()
        && min_funding_opt.is_none()
        && max_funding_opt.is_none()
        && impact_quantity_opt.is_none()
        && group_insurance_fund_opt.is_none()
        && fee_penalty_opt.is_none()
        && settle_fee_flat_opt.is_none()
        && settle_fee_amount_threshold_opt.is_none()
        && settle_fee_fraction_low_health_opt.is_none()
        && stable_price_delay_interval_seconds_opt.is_none()
        && stable_price_delay_growth_limit_opt.is_none()
        && stable_price_growth_limit_opt.is_none()
        && settle_pnl_limit_factor_opt.is_none()
        && settle_pnl_limit_window_size_ts_opt.is_none()
        // security admin can bring to reduce only mode
        && reduce_only_opt.is_some()
    {
        require!(
            group.admin == ctx.accounts.admin.key()
                || group.security_admin == ctx.accounts.admin.key(),
            MangoError::SomeError
        );
    } else {
        require!(
            group.admin == ctx.accounts.admin.key(),
            MangoError::SomeError
        );
    }

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    // note: unchanged fields are inline, and match exact definition in perp_register_market
    // please maintain, and don't remove, makes it easy to reason about which support admin modification

    // unchanged -
    // name
    // group

    if let Some(oracle_config) = oracle_config_opt {
        perp_market.oracle_config = oracle_config.to_oracle_config();
    };
    if let Some(oracle) = oracle_opt {
        perp_market.oracle = oracle;

        require_keys_eq!(oracle, ctx.accounts.oracle.key());
        let oracle_price = perp_market
            .oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?, None)?;
        perp_market.stable_price_model.reset_to_price(
            oracle_price.to_num(),
            Clock::get()?.unix_timestamp.try_into().unwrap(),
        );
    }

    // unchanged -
    // bids
    // asks
    // event_queue
    // quote_lot_size
    // base_lot_size

    if let Some(maint_base_asset_weight) = maint_base_asset_weight_opt {
        perp_market.maint_base_asset_weight = I80F48::from_num(maint_base_asset_weight);
    }
    if let Some(init_base_asset_weight) = init_base_asset_weight_opt {
        perp_market.init_base_asset_weight = I80F48::from_num(init_base_asset_weight);
    }
    if let Some(maint_base_liab_weight) = maint_base_liab_weight_opt {
        perp_market.maint_base_liab_weight = I80F48::from_num(maint_base_liab_weight);
    }
    if let Some(init_base_liab_weight) = init_base_liab_weight_opt {
        perp_market.init_base_liab_weight = I80F48::from_num(init_base_liab_weight);
    }
    if let Some(maint_pnl_asset_weight) = maint_pnl_asset_weight_opt {
        perp_market.maint_pnl_asset_weight = I80F48::from_num(maint_pnl_asset_weight);
    }
    if let Some(init_pnl_asset_weight) = init_pnl_asset_weight_opt {
        perp_market.init_pnl_asset_weight = I80F48::from_num(init_pnl_asset_weight);
    }
    if let Some(liquidation_fee) = liquidation_fee_opt {
        perp_market.liquidation_fee = I80F48::from_num(liquidation_fee);
    }

    if let Some(maker_fee) = maker_fee_opt {
        perp_market.maker_fee = I80F48::from_num(maker_fee);
    }
    if let Some(taker_fee) = taker_fee_opt {
        perp_market.taker_fee = I80F48::from_num(taker_fee);
    }

    if let Some(min_funding) = min_funding_opt {
        perp_market.min_funding = I80F48::from_num(min_funding);
    }
    if let Some(max_funding) = max_funding_opt {
        perp_market.max_funding = I80F48::from_num(max_funding);
    }
    if let Some(impact_quantity) = impact_quantity_opt {
        perp_market.impact_quantity = impact_quantity;
    }
    if let Some(fee_penalty) = fee_penalty_opt {
        perp_market.fee_penalty = fee_penalty;
    }

    // unchanged -
    // long_funding
    // short_funding
    // funding_last_updated
    // open_interest
    // seq_num
    // fees_accrued
    // bump

    if let Some(base_decimals) = base_decimals_opt {
        perp_market.base_decimals = base_decimals;
    }

    // unchanged -
    // perp_market_index

    // unchanged -
    // registration_time

    if let Some(group_insurance_fund) = group_insurance_fund_opt {
        perp_market.set_elligible_for_group_insurance_fund(group_insurance_fund);
    }

    if let Some(settle_fee_flat) = settle_fee_flat_opt {
        perp_market.settle_fee_flat = settle_fee_flat;
    }
    if let Some(settle_fee_amount_threshold) = settle_fee_amount_threshold_opt {
        perp_market.settle_fee_amount_threshold = settle_fee_amount_threshold;
    }
    if let Some(settle_fee_fraction_low_health) = settle_fee_fraction_low_health_opt {
        perp_market.settle_fee_fraction_low_health = settle_fee_fraction_low_health;
    }

    if let Some(stable_price_delay_interval_seconds) = stable_price_delay_interval_seconds_opt {
        // Updating this makes the old delay values slightly inconsistent
        perp_market.stable_price_model.delay_interval_seconds = stable_price_delay_interval_seconds;
    }
    if let Some(stable_price_delay_growth_limit) = stable_price_delay_growth_limit_opt {
        perp_market.stable_price_model.delay_growth_limit = stable_price_delay_growth_limit;
    }
    if let Some(stable_price_growth_limit) = stable_price_growth_limit_opt {
        perp_market.stable_price_model.stable_growth_limit = stable_price_growth_limit;
    }

    if let Some(settle_pnl_limit_factor_opt) = settle_pnl_limit_factor_opt {
        perp_market.settle_pnl_limit_factor = settle_pnl_limit_factor_opt;
    }
    if let Some(settle_pnl_limit_window_size_ts) = settle_pnl_limit_window_size_ts_opt {
        perp_market.settle_pnl_limit_window_size_ts = settle_pnl_limit_window_size_ts;
    }

    if let Some(reduce_only) = reduce_only_opt {
        perp_market.reduce_only = u8::from(reduce_only);
    };

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
