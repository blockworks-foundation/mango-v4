use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(trigger_num: u64, condition: Vec<u8>, action: Vec<u8>)]
pub struct TriggerCreate<'info> {
    #[account(
        // TODO: constraint = group.load()?.is_ix_enabled(IxGate::TriggerCreate) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        init,
        seeds = [b"Trigger".as_ref(), group.key().as_ref(), account.key().as_ref(), &trigger_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Trigger>() + condition.len() + action.len(),
    )]
    pub trigger: AccountLoader<'info, Trigger>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
