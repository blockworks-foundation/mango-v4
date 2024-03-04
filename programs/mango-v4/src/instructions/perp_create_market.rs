use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::state::*;
use crate::util::fill_from_str;

use crate::accounts_ix::*;
use crate::logs::{emit_stack, PerpMarketMetaDataLog};

#[allow(clippy::too_many_arguments)]
pub fn perp_create_market(
    ctx: Context<PerpCreateMarket>,
    perp_market_index: PerpMarketIndex,
    settle_token_index: TokenIndex,
    name: String,
    oracle_config: OracleConfigParams,
    base_decimals: u8,
    quote_lot_size: i64,
    base_lot_size: i64,
    maint_base_asset_weight: f32,
    init_base_asset_weight: f32,
    maint_base_liab_weight: f32,
    init_base_liab_weight: f32,
    maint_overall_asset_weight: f32,
    init_overall_asset_weight: f32,
    base_liquidation_fee: f32,
    maker_fee: f32,
    taker_fee: f32,
    min_funding: f32,
    max_funding: f32,
    impact_quantity: i64,
    group_insurance_fund: bool,
    fee_penalty: f32,
    settle_fee_flat: f32,
    settle_fee_amount_threshold: f32,
    settle_fee_fraction_low_health: f32,
    settle_pnl_limit_factor: f32,
    settle_pnl_limit_window_size_ts: u64,
    positive_pnl_liquidation_fee: f32,
    platform_liquidation_fee: f32,
) -> Result<()> {
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let mut perp_market = ctx.accounts.perp_market.load_init()?;
    *perp_market = PerpMarket {
        group: ctx.accounts.group.key(),
        settle_token_index,
        perp_market_index,
        blocked1: 0,
        group_insurance_fund: u8::from(group_insurance_fund),
        bump: *ctx.bumps.get("perp_market").ok_or(MangoError::SomeError)?,
        base_decimals,
        name: fill_from_str(&name)?,
        bids: ctx.accounts.bids.key(),
        asks: ctx.accounts.asks.key(),
        event_queue: ctx.accounts.event_queue.key(),
        oracle: ctx.accounts.oracle.key(),
        oracle_config: oracle_config.to_oracle_config(),
        stable_price_model: StablePriceModel::default(),
        quote_lot_size,
        base_lot_size,
        maint_base_asset_weight: I80F48::from_num(maint_base_asset_weight),
        init_base_asset_weight: I80F48::from_num(init_base_asset_weight),
        maint_base_liab_weight: I80F48::from_num(maint_base_liab_weight),
        init_base_liab_weight: I80F48::from_num(init_base_liab_weight),
        open_interest: 0,
        seq_num: 0,
        registration_time: now_ts,
        min_funding: I80F48::from_num(min_funding),
        max_funding: I80F48::from_num(max_funding),
        impact_quantity,
        long_funding: I80F48::ZERO,
        short_funding: I80F48::ZERO,
        funding_last_updated: now_ts,
        base_liquidation_fee: I80F48::from_num(base_liquidation_fee),
        maker_fee: I80F48::from_num(maker_fee),
        taker_fee: I80F48::from_num(taker_fee),
        fees_accrued: I80F48::ZERO,
        fees_settled: I80F48::ZERO,
        fee_penalty,
        settle_fee_flat,
        settle_fee_amount_threshold,
        settle_fee_fraction_low_health,
        settle_pnl_limit_factor,
        padding3: Default::default(),
        settle_pnl_limit_window_size_ts,
        reduce_only: 0,
        force_close: 0,
        padding4: Default::default(),
        maint_overall_asset_weight: I80F48::from_num(maint_overall_asset_weight),
        init_overall_asset_weight: I80F48::from_num(init_overall_asset_weight),
        positive_pnl_liquidation_fee: I80F48::from_num(positive_pnl_liquidation_fee),
        fees_withdrawn: 0,
        platform_liquidation_fee: I80F48::from_num(platform_liquidation_fee),
        accrued_liquidation_fees: I80F48::ZERO,
        reserved: [0; 1848],
    };

    let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
    if let Ok(oracle_price) =
        perp_market.oracle_price(&OracleAccountInfos::from_reader(oracle_ref), None)
    {
        perp_market
            .stable_price_model
            .reset_to_price(oracle_price.to_num(), now_ts);
    } else {
        perp_market.stable_price_model.reset_on_nonzero_price = 1;
    }

    let mut orderbook = Orderbook {
        bids: ctx.accounts.bids.load_init()?,
        asks: ctx.accounts.asks.load_init()?,
    };
    orderbook.init();

    emit_stack(PerpMarketMetaDataLog {
        mango_group: ctx.accounts.group.key(),
        perp_market: ctx.accounts.perp_market.key(),
        perp_market_index,
        base_decimals,
        base_lot_size,
        quote_lot_size,
        oracle: ctx.accounts.oracle.key(),
    });

    Ok(())
}
