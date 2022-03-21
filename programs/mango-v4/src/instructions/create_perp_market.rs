use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::*;

#[derive(Accounts)]
#[instruction(perp_market_index: PerpMarketIndex)]
pub struct CreatePerpMarket<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    pub oracle: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"PerpMarket".as_ref(), &perp_market_index.to_le_bytes().as_ref()],
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

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_perp_market(
    ctx: Context<CreatePerpMarket>,
    perp_market_index: PerpMarketIndex,
    base_token_index_opt: Option<TokenIndex>,
    quote_token_index: TokenIndex,
    quote_lot_size: i64,
    base_lot_size: i64,
) -> Result<()> {
    let mut perp_market = ctx.accounts.perp_market.load_init()?;
    *perp_market = PerpMarket {
        group: ctx.accounts.group.key(),
        oracle: ctx.accounts.oracle.key(),
        bids: ctx.accounts.bids.key(),
        asks: ctx.accounts.asks.key(),
        quote_lot_size: quote_lot_size,
        base_lot_size: base_lot_size,
        seq_num: 0,
        perp_market_index,
        base_token_index: base_token_index_opt.ok_or(TokenIndex::MAX).unwrap(),
        quote_token_index,
        bump: *ctx.bumps.get("perp_market").ok_or(MangoError::SomeError)?,
    };

    let mut bids = ctx.accounts.bids.load_init()?;
    bids.book_side_type = BookSideType::Bids;

    let mut asks = ctx.accounts.asks.load_init()?;
    asks.book_side_type = BookSideType::Asks;

    Ok(())
}
