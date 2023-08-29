use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AccountForceShrink<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AccountForceShrink) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    pub system_program: Program<'info, System>,
}
