use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;
use openbook_v2::{
    program::OpenbookV2,
    state::{BookSide, Market, OpenOrdersAccount},
};

#[derive(Accounts)]
pub struct OpenbookV2CancelOrder<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2CancelOrder) @ MangoError::IxIsDisabled,
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

    #[account(mut)]
    pub open_orders: AccountLoader<'info, OpenOrdersAccount>,

    #[account(
        has_one = group,
        has_one = openbook_v2_program,
        has_one = openbook_v2_market_external,
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    pub openbook_v2_program: Program<'info, OpenbookV2>,

    #[account(mut)]
    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    #[account(mut)]
    pub market_bids: AccountLoader<'info, BookSide>,

    #[account(mut)]
    pub market_asks: AccountLoader<'info, BookSide>,

    #[account(mut)]
    pub market_event_queue: AccountLoader<'info, EventQueue>,
}
