use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

#[derive(Accounts)]
pub struct CloseGroup<'info> {
    #[account(
        mut,
        constraint = group.load()?.admin == admin.key(),
        close = sol_destination
    )]
    pub group: AccountLoader<'info, Group>,

    pub admin: Signer<'info>,

    #[account(mut)]
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn close_group(_ctx: Context<CloseGroup>) -> Result<()> {
    Ok(())
}
