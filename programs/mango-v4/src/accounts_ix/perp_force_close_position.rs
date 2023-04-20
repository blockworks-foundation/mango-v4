use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PerpForceClosePosition<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpForceClosePosition) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = oracle,
        constraint = perp_market.load()?.is_force_close()
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    #[account(
        mut,
        has_one = group,
        constraint = account_a.load()?.is_operational() @ MangoError::AccountIsFrozen,
        constraint = account_a.key() != account_b.key()
    )]
    pub account_a: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        mut,
        has_one = group,
        constraint = account_b.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account_b: AccountLoader<'info, MangoAccountFixed>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,
}
