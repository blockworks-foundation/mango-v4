use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TokenConditionalSwapCreate<'info> {
    // The ix gate is checked in individual instructions
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen,
        constraint = account.load()?.is_owner_or_delegate(authority.key()),
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub authority: Signer<'info>,

    #[account(
        has_one = group,
    )]
    pub buy_bank: AccountLoader<'info, Bank>,
    #[account(
        has_one = group,
    )]
    pub sell_bank: AccountLoader<'info, Bank>,
}
