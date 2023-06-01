use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn trigger_check<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, TriggerCheck<'info>>,
    trigger_id: u64,
) -> Result<()> {
    {
        // just to ensure it's good?!
        ctx.accounts.triggers.load()?;
    }

    let triggers_ai = ctx.accounts.triggers.as_ref();
    let now_slot = Clock::get()?.slot;

    let trigger_offset;
    {
        let bytes = triggers_ai.try_borrow_data()?;
        trigger_offset = Triggers::find_trigger_offset_by_id(&bytes, trigger_id)?;
        let (_triggers, trigger, condition, _action) =
            Trigger::all_from_bytes(&bytes, trigger_offset)?;

        require!(trigger.condition_was_met == 0, MangoError::SomeError);
        require_gt!(trigger.expiry_slot, now_slot);

        condition.check(ctx.remaining_accounts)?;
    }

    let incentive_lamports;
    {
        let mut bytes = triggers_ai.try_borrow_mut_data()?;
        let trigger = Trigger::from_bytes_mut(&mut bytes[trigger_offset..])?;

        trigger.condition_was_met = 1;
        // TODO: how far in the future does a trigger expire once the condition is met?
        trigger.expiry_slot = now_slot + 1000;

        incentive_lamports = trigger.incentive_lamports;
    }

    // Transfer reward lamports
    {
        let mut triggers_lamports = triggers_ai.try_borrow_mut_lamports()?;
        let triggerer_ai = ctx.accounts.triggerer.as_ref();
        let mut triggerer_lamports = triggerer_ai.try_borrow_mut_lamports()?;
        **triggerer_lamports += incentive_lamports;
        **triggers_lamports -= incentive_lamports;
    }

    Ok(())
}
