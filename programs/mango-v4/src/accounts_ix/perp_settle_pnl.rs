use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PerpSettlePnl<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpSettlePnl) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = settler.load()?.is_operational() @ MangoError::AccountIsFrozen
        // settler_owner is checked at #1
    )]
    pub settler: AccountLoader<'info, MangoAccountFixed>,
    pub settler_owner: Signer<'info>,

    #[account(has_one = group, has_one = oracle)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    // This account MUST be profitable
    #[account(mut,
        has_one = group,
        constraint = account_a.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account_a: AccountLoader<'info, MangoAccountFixed>,
    // This account MUST have a loss
    #[account(
        mut,
        has_one = group,
        constraint = account_b.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account_b: AccountLoader<'info, MangoAccountFixed>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,

    // bank correctness is checked at #2
    #[account(mut, has_one = group)]
    pub settle_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,
}
