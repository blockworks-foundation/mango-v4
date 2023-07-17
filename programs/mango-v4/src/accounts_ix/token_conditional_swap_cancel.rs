use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TokenConditionalSwapCancel<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::TokenConditionalSwapCancel) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen,
        constraint = account.load()?.is_owner_or_delegate(authority.key()),
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub authority: Signer<'info>,

    /// The bank's token_index is checked at #1
    #[account(
        mut,
        has_one = group,
    )]
    pub buy_bank: AccountLoader<'info, Bank>,
    #[account(
        mut,
        has_one = group,
    )]
    pub sell_bank: AccountLoader<'info, Bank>,
}
