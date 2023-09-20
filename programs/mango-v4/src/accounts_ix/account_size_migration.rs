use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AccountSizeMigration<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AccountSizeMigration) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
