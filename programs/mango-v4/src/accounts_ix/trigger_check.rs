use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TriggerCheck<'info> {
    #[account(
        // TODO: constraint = group.load()?.is_ix_enabled(IxGate::AccountCreate) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
    )]
    pub triggers: AccountLoader<'info, Triggers>,

    #[account(mut)]
    pub triggerer: Signer<'info>,

    pub system_program: Program<'info, System>,
    // Lots of remaining accounts for all the details
}
