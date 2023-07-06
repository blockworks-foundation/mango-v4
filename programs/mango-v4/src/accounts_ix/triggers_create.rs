use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction()]
pub struct TriggersCreate<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::TriggersCreate) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        has_one = group,
        // authority is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub authority: Signer<'info>,

    #[account(
        init,
        seeds = [b"Triggers".as_ref(), account.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Triggers>(),
    )]
    pub triggers: AccountLoader<'info, Triggers>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
