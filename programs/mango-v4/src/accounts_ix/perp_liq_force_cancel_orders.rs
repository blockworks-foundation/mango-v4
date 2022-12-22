use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PerpLiqForceCancelOrders<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpLiqForceCancelOrders) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    // Allow force cancel even if account is frozen
    #[account(
        mut,
        has_one = group
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        mut,
        has_one = group,
        has_one = bids,
        has_one = asks,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub bids: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub asks: AccountLoader<'info, BookSide>,
}
