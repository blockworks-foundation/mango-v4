use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TokenConditionalSwapStart<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::TokenConditionalSwapStart) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        mut,
        has_one = group,
        constraint = caller.load()?.is_operational() @ MangoError::AccountIsFrozen,
        constraint = caller.load()?.is_owner_or_delegate(caller_authority.key()),
        constraint = caller.key() != account.key(),
    )]
    pub caller: AccountLoader<'info, MangoAccountFixed>,
    pub caller_authority: Signer<'info>,
}
