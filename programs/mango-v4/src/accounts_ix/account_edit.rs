use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AccountEdit<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AccountEdit) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,
}
