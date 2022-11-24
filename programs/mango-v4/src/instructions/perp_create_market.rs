use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::state::*;
use crate::util::fill_from_str;

use crate::logs::PerpMarketMetaDataLog;

#[derive(Accounts)]
#[instruction(perp_market_index: PerpMarketIndex)]
pub struct PerpCreateMarket<'info> {
    #[account(
        has_one = admin,
        constraint = group.load()?.perps_supported()
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [b"PerpMarket".as_ref(), group.key().as_ref(), perp_market_index.to_le_bytes().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<PerpMarket>(),
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    /// Accounts are initialised by client,
    /// anchor discriminator is set first when ix exits,
    #[account(zero)]
    pub orderbook: AccountLoader<'info, Orderbook>,
    #[account(zero)]
    pub event_queue: AccountLoader<'info, EventQueue>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[allow(clippy::too_many_arguments)]
pub fn perp_create_market(
    ctx: Context<PerpCreateMarket>,
    settle_token_index: TokenIndex,
    perp_market_index: PerpMarketIndex,
    name: String,
    oracle_config: OracleConfigParams,
    base_decimals: u8,
    quote_lot_size: i64,
    base_lot_size: i64,
    maint_asset_weight: f32,
    init_asset_weight: f32,
    maint_liab_weight: f32,
    init_liab_weight: f32,
    liquidation_fee: f32,
    maker_fee: f32,
    taker_fee: f32,
    min_funding: f32,
    max_funding: f32,
    impact_quantity: i64,
    group_insurance_fund: bool,
    trusted_market: bool,
    fee_penalty: f32,
    settle_fee_flat: f32,
    settle_fee_amount_threshold: f32,
    settle_fee_fraction_low_health: f32,
    settle_pnl_limit_factor: f32,
    settle_pnl_limit_factor_window_size_ts: u64,
) -> Result<()> {
    // Settlement tokens that aren't USDC aren't fully implemented, the main missing steps are:
    // - In health: the perp health needs to be adjusted by the settlement token weights.
    //   Otherwise settling perp pnl could decrease health.
    // - In settle pnl and settle fees: use the settle oracle to convert the pnl from USD to token.
    // - In perp bankruptcy: fix the assumption that the insurance fund has the same mint as
    //   the settlement token.
    require_msg!(
        settle_token_index == QUOTE_TOKEN_INDEX,
        "settlement tokens != USDC are not fully implemented"
    );

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let mut perp_market = ctx.accounts.perp_market.load_init()?;
    *perp_market = PerpMarket {
        group: ctx.accounts.group.key(),
        settle_token_index,
        perp_market_index,
        group_insurance_fund: if group_insurance_fund { 1 } else { 0 },
        trusted_market: if trusted_market { 1 } else { 0 },
        name: fill_from_str(&name)?,
        oracle: ctx.accounts.oracle.key(),
        oracle_config: oracle_config.to_oracle_config(),
        orderbook: ctx.accounts.orderbook.key(),
        event_queue: ctx.accounts.event_queue.key(),
        quote_lot_size,
        base_lot_size,
        maint_asset_weight: I80F48::from_num(maint_asset_weight),
        init_asset_weight: I80F48::from_num(init_asset_weight),
        maint_liab_weight: I80F48::from_num(maint_liab_weight),
        init_liab_weight: I80F48::from_num(init_liab_weight),
        liquidation_fee: I80F48::from_num(liquidation_fee),
        maker_fee: I80F48::from_num(maker_fee),
        taker_fee: I80F48::from_num(taker_fee),
        min_funding: I80F48::from_num(min_funding),
        max_funding: I80F48::from_num(max_funding),
        impact_quantity,
        long_funding: I80F48::ZERO,
        short_funding: I80F48::ZERO,
        funding_last_updated: now_ts,
        open_interest: 0,
        seq_num: 0,
        fees_accrued: I80F48::ZERO,
        fees_settled: I80F48::ZERO,
        bump: *ctx.bumps.get("perp_market").ok_or(MangoError::SomeError)?,
        base_decimals,
        registration_time: now_ts,
        padding1: Default::default(),
        padding2: Default::default(),
        fee_penalty,
        settle_fee_flat,
        settle_fee_amount_threshold,
        settle_fee_fraction_low_health,
        stable_price_model: StablePriceModel::default(),
        settle_pnl_limit_factor,
        settle_pnl_limit_factor_window_size_ts,
        reserved: [0; 1944],
    };

    let oracle_price =
        perp_market.oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?, None)?;
    perp_market
        .stable_price_model
        .reset_to_price(oracle_price.to_num(), now_ts);

    let mut orderbook = ctx.accounts.orderbook.load_init()?;
    orderbook.init();

    emit!(PerpMarketMetaDataLog {
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
