use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TriggerCheck<'info> {
    // IxGate check needs to happen in the instruction, as this is shared among
    // TriggerCheck and TriggerCheckAndExecute
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
