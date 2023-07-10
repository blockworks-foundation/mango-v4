use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::state::*;

pub fn trigger_cancel(ctx: Context<TriggerCancel>, trigger_id: u64) -> Result<()> {
    // just to ensure the account is good
    ctx.accounts.triggers.load()?;

    let triggers_ai = ctx.accounts.triggers.as_ref();

    let (new_account_size, freed_lamports) = {
        let mut bytes = triggers_ai.try_borrow_mut_data()?;
        let trigger_offset = Triggers::find_trigger_offset_by_id(&bytes, trigger_id)?;
        let (triggers, trigger, condition, action) =
            Trigger::all_from_bytes(&bytes, trigger_offset)?;

        Triggers::remove_trigger(&mut bytes, trigger_id, trigger_offset)?
    };

    triggers_ai.realloc(new_account_size, false)?;
    Triggers::transfer_lamports(
        triggers_ai,
        &ctx.accounts.lamport_destination,
        freed_lamports,
    )?;

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
