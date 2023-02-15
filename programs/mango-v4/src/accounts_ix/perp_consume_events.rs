use crate::error::MangoError;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PerpConsumeEvents<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpConsumeEvents) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = event_queue,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    #[account(mut)]
    pub event_queue: AccountLoader<'info, EventQueue>,
}
