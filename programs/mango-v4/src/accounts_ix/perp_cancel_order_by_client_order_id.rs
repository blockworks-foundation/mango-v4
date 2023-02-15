use crate::error::*;
use crate::state::{BookSide, Group, IxGate, MangoAccountFixed, PerpMarket};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PerpCancelOrderByClientOrderId<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpCancelOrderByClientOrderId) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        // owner is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

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
