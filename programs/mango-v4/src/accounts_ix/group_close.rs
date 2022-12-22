use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

#[derive(Accounts)]
pub struct GroupClose<'info> {
    #[account(
        mut,
        has_one = admin,
        has_one = insurance_vault,
        constraint = group.load()?.is_testing(),
        constraint = group.load()?.is_ix_enabled(IxGate::GroupClose) @ MangoError::IxIsDisabled,
        close = sol_destination
    )]
    pub group: AccountLoader<'info, Group>,

    pub admin: Signer<'info>,

    #[account(mut)]
    pub insurance_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
