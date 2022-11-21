use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::state::*;

#[derive(Accounts)]
pub struct PerpCloseMarket<'info> {
    #[account(
        constraint = group.load()?.is_testing(),
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = orderbook,
        has_one = event_queue,
        close = sol_destination
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    #[account(
        mut,
        close = sol_destination
    )]
    pub orderbook: AccountLoader<'info, Orderbook>,

    #[account(
        mut,
        close = sol_destination
    )]
    pub event_queue: AccountLoader<'info, EventQueue>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[allow(clippy::too_many_arguments)]
pub fn perp_close_market(_ctx: Context<PerpCloseMarket>) -> Result<()> {
    Ok(())
}
