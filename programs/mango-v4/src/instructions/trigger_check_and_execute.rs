use anchor_lang::prelude::*;
use solana_program::program_memory::sol_memmove;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn trigger_check_and_execute<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, TriggerCheck<'info>>,
    trigger_id: u64,
    num_condition_accounts: u8,
) -> Result<()> {
    let num_condition_accounts: usize = num_condition_accounts.into();

    {
        // just to ensure it's good?!
        ctx.accounts.triggers.load()?;
    }

    let triggers_ai = ctx.accounts.triggers.as_ref();

    let trigger_offset;
    let incentive_lamports;
    let new_account_size;
    {
        let mut bytes = triggers_ai.try_borrow_mut_data()?;
        trigger_offset = Triggers::find_trigger_offset_by_id(&bytes, trigger_id)?;
        let (triggers, trigger, condition, action) =
            Trigger::all_from_bytes(&bytes, trigger_offset)?;

        let now_slot = Clock::get()?.slot;
        require_gt!(trigger.expiry_slot, now_slot);

        let condition_accounts = &ctx.remaining_accounts[..num_condition_accounts];
        condition.check(condition_accounts)?;

        let action_accounts = &ctx.remaining_accounts[num_condition_accounts..];
        action.execute(triggers, ctx.accounts, action_accounts)?;

        // Figure out the new size and incentive the triggerer gets
        let trigger_size = trigger.total_bytes as usize;
        new_account_size = bytes.len() - trigger_size;
        let previous_rent = Rent::get()?.minimum_balance(bytes.len());
        let new_rent = Rent::get()?.minimum_balance(new_account_size);
        let freed_up_rent = previous_rent - new_rent;
        incentive_lamports = if trigger.condition_was_met == 0 {
            2 * trigger.incentive_lamports + freed_up_rent
        } else {
            trigger.incentive_lamports + freed_up_rent
        };

        // Move all trailing triggers forward, overwriting the closed one
        let trigger_end_offset = trigger_offset + trigger_size;
        if trigger_end_offset < bytes.len() {
            unsafe {
                sol_memmove(
                    &mut bytes[trigger_offset],
                    &mut bytes[trigger_end_offset],
                    bytes.len() - trigger_end_offset,
                );
            }
        }
    }

    triggers_ai.realloc(new_account_size, false)?;

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
