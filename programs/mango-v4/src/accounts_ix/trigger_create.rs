use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TriggerCreate<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::TriggerCreate) @ MangoError::IxIsDisabled,
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

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
