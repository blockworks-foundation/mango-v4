use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PerpSettleFees<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpSettleFees) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group, has_one = oracle)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    // This account MUST have a loss
    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,

    // bank correctness is checked at #2
    #[account(mut, has_one = group)]
    pub settle_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,
}
