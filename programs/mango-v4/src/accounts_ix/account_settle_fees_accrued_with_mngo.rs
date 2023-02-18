use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AccountSettleFeesAccruedWithMngo<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AccountSettleFeesAccruedWithMngo) @ MangoError::IxIsDisabled,
        constraint = group.load()?.pay_fees_with_mngo() @ MangoError::SomeError
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // owner is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen,
        constraint = group.load()?.dao_mango_account == dao_account.key()
    )]
    pub dao_account: AccountLoader<'info, MangoAccountFixed>,

    #[account(mut, has_one = group)]
    pub mngo_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = mngo_bank.load()?.oracle)]
    pub mngo_oracle: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = group,
        constraint = settle_bank.load()?.token_index == 0
    )]
    pub settle_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,
}
