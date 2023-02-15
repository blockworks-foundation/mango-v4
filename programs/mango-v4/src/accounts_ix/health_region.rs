use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;

/// Sets up for a health region
///
/// The same transaction must have the corresponding HealthRegionEnd call.
///
/// remaining_accounts: health accounts for account
#[derive(Accounts)]
pub struct HealthRegionBegin<'info> {
    /// Instructions Sysvar for instruction introspection
    /// CHECK: fixed instructions sysvar account
    #[account(address = tx_instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::HealthRegion) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
}

/// Ends a health region.
///
/// remaining_accounts: health accounts for account
#[derive(Accounts)]
pub struct HealthRegionEnd<'info> {
    #[account(
        mut,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
}
