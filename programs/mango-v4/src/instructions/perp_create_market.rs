use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::error::MangoError;
use crate::state::*;

#[derive(Accounts)]
#[instruction(perp_market_index: PerpMarketIndex)]
pub struct PerpCreateMarket<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    pub oracle: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"PerpMarket".as_ref(), perp_market_index.to_le_bytes().as_ref()],
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
    base_token_index_opt: Option<TokenIndex>,
    quote_token_index: TokenIndex,
    quote_lot_size: i64,
    base_lot_size: i64,
    maint_asset_weight: f32,
    init_asset_weight: f32,
    maint_liab_weight: f32,
    init_liab_weight: f32,
    liquidation_fee: f32,
    maker_fee: f32,
    taker_fee: f32,
) -> Result<()> {
    let mut perp_market = ctx.accounts.perp_market.load_init()?;
    *perp_market = PerpMarket {
        group: ctx.accounts.group.key(),
        oracle: ctx.accounts.oracle.key(),
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
        open_interest: 0,
        seq_num: 0,
        fees_accrued: I80F48::ZERO,
        bump: *ctx.bumps.get("perp_market").ok_or(MangoError::SomeError)?,
        perp_market_index,
        base_token_index: base_token_index_opt.ok_or(TokenIndex::MAX).unwrap(),
        quote_token_index,
    };

    let mut bids = ctx.accounts.bids.load_init()?;
    bids.book_side_type = BookSideType::Bids;

    let mut asks = ctx.accounts.asks.load_init()?;
    asks.book_side_type = BookSideType::Asks;

    Ok(())
}
