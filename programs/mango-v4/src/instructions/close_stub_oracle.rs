use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::state::*;

#[derive(Accounts)]
pub struct CloseStubOracle<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    // match stub oracle to group
    #[account(
        mut,
        constraint = oracle.load()?.group == group.key(),
        close = sol_destination
    )]
    pub oracle: AccountLoader<'info, StubOracle>,

    #[account(mut)]
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[allow(clippy::too_many_arguments)]
pub fn close_stub_oracle(_ctx: Context<CloseStubOracle>) -> Result<()> {
    Ok(())
}
