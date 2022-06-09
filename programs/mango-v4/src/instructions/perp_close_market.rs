use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::state::*;

#[derive(Accounts)]
pub struct PerpCloseMarket<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = perp_market.load()?.group == group.key(),
        close = sol_destination
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    #[account(
        mut,
        constraint = bids.key() == perp_market.load()?.bids,
        close = sol_destination
    )]
    pub bids: AccountLoader<'info, BookSide>,

    #[account(
        mut,
        constraint = asks.key() == perp_market.load()?.asks,
        close = sol_destination
    )]
    pub asks: AccountLoader<'info, BookSide>,

    #[account(
        mut,
        constraint = event_queue.key() == perp_market.load()?.event_queue,
        close = sol_destination
    )]
    pub event_queue: AccountLoader<'info, EventQueue>,

    #[account(mut)]
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[allow(clippy::too_many_arguments)]
pub fn perp_close_market(_ctx: Context<PerpCloseMarket>) -> Result<()> {
    Ok(())
}
