use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AccountSettleFeesWithMngo<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AccountSettleFeesWithMngo) @ MangoError::IxIsDisabled,
        constraint = group.load()?.fees_pay_with_mngo() @ MangoError::SomeError
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
        constraint = group.load()?.fees_swap_mango_account == dao_account.key()
    )]
    pub dao_account: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        mut,
        has_one = group,
        constraint = mngo_bank.load()?.token_index == group.load()?.fees_mngo_token_index,
        constraint = mngo_bank.load()?.token_index != 0, // should not be unset
    )]
    pub mngo_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = mngo_bank.load()?.oracle)]
    pub mngo_oracle: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = group,
        constraint = settle_bank.load()?.token_index == QUOTE_TOKEN_INDEX
    )]
    pub settle_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,
}
