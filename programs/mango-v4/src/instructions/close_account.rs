use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::state::*;

#[derive(Accounts)]
pub struct CloseAccount<'info> {
    #[account(
        mut,
        has_one = owner,
        close = sol_destination
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(mut)]
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn close_account(_ctx: Context<CloseAccount>) -> Result<()> {
    // CRITICAL: currently can close any account, even one with bad health
    // TODO: Implement
    Ok(())
}
