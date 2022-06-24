use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::state::*;

#[derive(Accounts)]
pub struct Serum3DeregisterMarket<'info> {
    #[account(
        mut,
        constraint = group.load()?.testing == 1,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        close = sol_destination
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn serum3_deregister_market(_ctx: Context<Serum3DeregisterMarket>) -> Result<()> {
    Ok(())
}
