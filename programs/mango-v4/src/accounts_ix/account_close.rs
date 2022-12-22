use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct AccountClose<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AccountClose) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen,
        close = sol_destination
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
