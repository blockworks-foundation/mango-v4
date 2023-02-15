use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AccountToggleFreeze<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AccountToggleFreeze) @ MangoError::IxIsDisabled,
        constraint = group.load()?.admin == admin.key()
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    pub admin: Signer<'info>,
}
