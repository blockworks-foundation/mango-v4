use crate::state::{BookSide, Group, MangoAccountFixed, PerpMarket};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PerpPurgeOrders<'info> {
    #[account()]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        // owner is not checked on purpose
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
