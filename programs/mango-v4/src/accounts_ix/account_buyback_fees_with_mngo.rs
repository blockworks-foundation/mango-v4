use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AccountBuybackFeesWithMngo<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AccountBuybackFeesWithMngo) @ MangoError::IxIsDisabled,
        constraint = group.load()?.buyback_fees() @ MangoError::SomeError
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
        address = group.load()?.buyback_fees_swap_mango_account
    )]
    pub dao_account: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        mut,
        has_one = group,
        constraint = mngo_bank.load()?.token_index == group.load()?.mngo_token_index,
        constraint = mngo_bank.load()?.token_index != 0, // should not be unset
    )]
    pub mngo_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = mngo_bank.load()?.oracle)]
    pub mngo_oracle: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = group,
        constraint = fees_bank.load()?.token_index == QUOTE_TOKEN_INDEX
    )]
    pub fees_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = fees_bank.load()?.oracle)]
    pub fees_oracle: UncheckedAccount<'info>,
}
