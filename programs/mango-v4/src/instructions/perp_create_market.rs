use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::error::MangoError;

use crate::state::*;
use crate::util::fill_from_str;

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
    pub bids_direct: AccountLoader<'info, BookSide>,
    #[account(zero)]
    pub asks_direct: AccountLoader<'info, BookSide>,
    #[account(zero)]
    pub bids_oracle_pegged: AccountLoader<'info, BookSide>,
    #[account(zero)]
    pub asks_oracle_pegged: AccountLoader<'info, BookSide>,
    #[account(zero)]
    pub event_queue: AccountLoader<'info, EventQueue>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[allow(clippy::too_many_arguments)]
pub fn perp_create_market(
    ctx: Context<PerpCreateMarket>,
    perp_market_index: PerpMarketIndex,
    name: String,
    oracle_config: OracleConfig,
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
) -> Result<()> {
    let mut perp_market = ctx.accounts.perp_market.load_init()?;
    *perp_market = PerpMarket {
        name: fill_from_str(&name)?,
        group: ctx.accounts.group.key(),
        oracle: ctx.accounts.oracle.key(),
        oracle_config,
        bids_direct: ctx.accounts.bids_direct.key(),
        asks_direct: ctx.accounts.asks_direct.key(),
        bids_oracle_pegged: ctx.accounts.bids_oracle_pegged.key(),
        asks_oracle_pegged: ctx.accounts.asks_oracle_pegged.key(),
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
        funding_last_updated: Clock::get()?.unix_timestamp,
        open_interest: 0,
        seq_num: 0,
        fees_accrued: I80F48::ZERO,
        fees_settled: I80F48::ZERO,
        bump: *ctx.bumps.get("perp_market").ok_or(MangoError::SomeError)?,
        base_decimals,
        perp_market_index,
        registration_time: Clock::get()?.unix_timestamp,
        group_insurance_fund: if group_insurance_fund { 1 } else { 0 },
        trusted_market: if trusted_market { 1 } else { 0 },
        padding0: Default::default(),
        padding1: Default::default(),
        padding2: Default::default(),
        fee_penalty,
        settle_fee_flat,
        settle_fee_amount_threshold,
        settle_fee_fraction_low_health,
        reserved: [0; 28],
    };

    let mut bids_direct = ctx.accounts.bids_direct.load_init()?;
    bids_direct.book_side_type = BookSideType::Bids;

    let mut asks_direct = ctx.accounts.asks_direct.load_init()?;
    asks_direct.book_side_type = BookSideType::Asks;

    let mut bids_oracle_pegged = ctx.accounts.bids_oracle_pegged.load_init()?;
    bids_oracle_pegged.book_side_type = BookSideType::Bids;

    let mut asks_oracle_pegged = ctx.accounts.asks_oracle_pegged.load_init()?;
    asks_oracle_pegged.book_side_type = BookSideType::Asks;

    Ok(())
}
