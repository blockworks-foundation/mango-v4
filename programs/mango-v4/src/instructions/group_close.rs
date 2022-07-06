use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

#[derive(Accounts)]
pub struct CloseGroup<'info> {
    #[account(
        mut,
        constraint = group.load()?.testing == 1,
        has_one = admin,
        close = sol_destination
    )]
    pub group: AccountLoader<'info, Group>,

    pub admin: Signer<'info>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn group_close(_ctx: Context<CloseGroup>) -> Result<()> {
    // TODO: checks
    Ok(())
}
