use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TriggerCancel<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::TriggerCancel) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        has_one = group,
        // authority is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = account,
    )]
    pub triggers: AccountLoader<'info, Triggers>,

    /// CHECK: This account receives the freed up lamports
    #[account(mut)]
    pub lamport_destination: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}
