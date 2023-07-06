use anchor_lang::prelude::*;
use solana_program::program_memory::sol_memmove;

use crate::accounts_ix::*;
use crate::state::*;

pub fn trigger_cancel(ctx: Context<TriggerCancel>, trigger_id: u64) -> Result<()> {
    // just to ensure the account is good
    ctx.accounts.triggers.load()?;

    let triggers_ai = ctx.accounts.triggers.as_ref();

    let trigger_offset;
    let freed_lamports;
    let new_account_size;
    {
        let mut bytes = triggers_ai.try_borrow_mut_data()?;
        trigger_offset = Triggers::find_trigger_offset_by_id(&bytes, trigger_id)?;
        let (triggers, trigger, condition, action) =
            Trigger::all_from_bytes(&bytes, trigger_offset)?;

        // Figure out the new size and incentive the triggerer gets
        let trigger_size = trigger.total_bytes as usize;
        new_account_size = bytes.len() - trigger_size;
        let previous_rent = Rent::get()?.minimum_balance(bytes.len());
        let new_rent = Rent::get()?.minimum_balance(new_account_size);
        let freed_up_rent = previous_rent - new_rent;
        freed_lamports = if trigger.condition_was_met == 0 {
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

    // Transfer unused reward lamports and rent back
    {
        let mut triggers_lamports = triggers_ai.try_borrow_mut_lamports()?;
        let destination_ai = ctx.accounts.lamport_destination.as_ref();
        let mut destination_lamports = destination_ai.try_borrow_mut_lamports()?;
        **destination_lamports += freed_lamports;
        **triggers_lamports -= freed_lamports;
    }

    Ok(())
}
