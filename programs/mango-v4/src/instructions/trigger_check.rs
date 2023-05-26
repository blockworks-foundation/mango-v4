use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn trigger_check<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, TriggerCheck<'info>>,
) -> Result<()> {
    {
        let trigger_bytes = ctx.accounts.trigger.as_ref().try_borrow_data()?;
        let (_trigger, condition, _action) = Trigger::from_account_bytes(&trigger_bytes)?;

        condition.check(ctx.remaining_accounts)?;
    }

    let mut trigger = ctx.accounts.trigger.load_mut()?;

    require!(trigger.condition_was_met == 0, MangoError::SomeError);

    let now_slot = Clock::get()?.slot;
    require_gt!(trigger.expiry_slot, now_slot);

    trigger.condition_was_met = 1;
    // TODO: how far in the future does a trigger expire once the condition is met?
    trigger.expiry_slot = now_slot + 1000;

    // Transfer reward lamports
    {
        let trigger_ai = ctx.accounts.trigger.as_ref();
        let mut trigger_lamports = trigger_ai.try_borrow_mut_lamports().unwrap();
        let triggerer_ai = ctx.accounts.triggerer.as_ref();
        let mut triggerer_lamports = triggerer_ai.try_borrow_mut_lamports().unwrap();
        **triggerer_lamports += trigger.incentive_lamports;
        **trigger_lamports -= trigger.incentive_lamports;
    }

    Ok(())
}
