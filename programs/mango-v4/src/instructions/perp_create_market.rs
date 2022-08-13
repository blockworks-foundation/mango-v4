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
    pub bids: AccountLoader<'info, BookSide>,
    #[account(zero)]
    pub asks: AccountLoader<'info, BookSide>,
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
    base_token_index_opt: Option<TokenIndex>,
    base_token_decimals: u8,
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
) -> Result<()> {
    let mut perp_market = ctx.accounts.perp_market.load_init()?;
    *perp_market = PerpMarket {
        name: fill_from_str(&name)?,
        group: ctx.accounts.group.key(),
        oracle: ctx.accounts.oracle.key(),
        oracle_config,
        bids: ctx.accounts.bids.key(),
        asks: ctx.accounts.asks.key(),
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
        // Why optional - Perp could be based purely on an oracle
        bump: *ctx.bumps.get("perp_market").ok_or(MangoError::SomeError)?,
        base_token_decimals,
        perp_market_index,
        base_token_index: base_token_index_opt.ok_or(TokenIndex::MAX).unwrap(),
        registration_time: Clock::get()?.unix_timestamp,
        padding1: Default::default(),
        padding2: Default::default(),
        reserved: [0; 128],
    };

    let mut bids = ctx.accounts.bids.load_init()?;
    bids.book_side_type = BookSideType::Bids;

    let mut asks = ctx.accounts.asks.load_init()?;
    asks.book_side_type = BookSideType::Asks;

    Ok(())
}
