use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn trigger_check_and_execute<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, TriggerCheck<'info>>,
    num_condition_accounts: u8,
) -> Result<()> {
    let num_condition_accounts: usize = num_condition_accounts.into();

    {
        let trigger_bytes = ctx.accounts.trigger.as_ref().try_borrow_data()?;
        let (trigger, condition, action) = Trigger::from_account_bytes(&trigger_bytes)?;

        let now_slot = Clock::get()?.slot;
        require_gt!(trigger.expiry_slot, now_slot);

        let condition_accounts = &ctx.remaining_accounts[..num_condition_accounts];
        condition.check(condition_accounts)?;

        let action_accounts = &ctx.remaining_accounts[num_condition_accounts..];
        action.execute(trigger, ctx.accounts, action_accounts)?;
    }

    // Transfer all the lamports on the trigger account to the triggerer
    // and close the trigger account.
    {
        let trigger_ai = ctx.accounts.trigger.as_ref();
        let mut trigger_lamports = trigger_ai.try_borrow_mut_lamports().unwrap();
        let triggerer_ai = ctx.accounts.triggerer.as_ref();
        let mut triggerer_lamports = triggerer_ai.try_borrow_mut_lamports().unwrap();
        **triggerer_lamports += **trigger_lamports;
        **trigger_lamports = 0;

        trigger_ai.assign(&solana_program::system_program::ID);
        trigger_ai.realloc(0, false)?;
    }

    Ok(())
}
