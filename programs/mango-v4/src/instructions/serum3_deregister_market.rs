use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::state::*;

#[derive(Accounts)]
pub struct Serum3DeregisterMarket<'info> {
    #[account(
        mut,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = serum_market.load()?.group == group.key(),
        close = sol_destination
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,

    #[account(mut)]
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn serum3_deregister_market(_ctx: Context<Serum3DeregisterMarket>) -> Result<()> {
    Ok(())
}
