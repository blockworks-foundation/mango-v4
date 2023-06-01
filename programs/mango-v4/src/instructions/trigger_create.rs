use anchor_lang::prelude::*;
use anchor_lang::system_program;
use solana_program::program_memory::sol_memcpy;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

fn next_8_byte_aligned_size(size: usize) -> usize {
    if size & 7 == 0 {
        size
    } else {
        (size & !7) + 8
    }
}

pub fn trigger_create(
    ctx: Context<TriggerCreate>,
    condition: Vec<u8>,
    action: Vec<u8>,
) -> Result<()> {
    // TODO: amount?
    let incentive_lamports = 100_000;

    // Sanity check: the first few bytes are the type
    require_gte!(condition.len(), 4);
    require_gte!(action.len(), 4);

    {
        let account = ctx.accounts.account.load()?;
        // account constraint #1
        require!(
            account.is_owner_or_delegate(ctx.accounts.authority.key()),
            MangoError::SomeError
        );
    }

    let trigger_id;
    {
        let mut triggers = ctx.accounts.triggers.load_mut()?;
        trigger_id = triggers.next_trigger_id;
        triggers.next_trigger_id = triggers.next_trigger_id.wrapping_add(1);
    }

    // Ensure the account has enough space for storing the trigger data
    let triggers_ai = ctx.accounts.triggers.as_ref();
    let prev_bytes_end = triggers_ai.try_data_len()?;
    let action_bytes_aligned = next_8_byte_aligned_size(action.len());
    let new_bytes_needed = std::mem::size_of::<Trigger>() + condition.len() + action_bytes_aligned;
    {
        let new_bytes_end = prev_bytes_end + new_bytes_needed;
        triggers_ai.realloc(new_bytes_end, false)?;

        // Fund the account for the new size
        // Beware, the account's balance may already be much higher because of incentives
        let added_rent = Rent::get()?.minimum_balance(new_bytes_end)
            - Rent::get()?.minimum_balance(prev_bytes_end);
        let transfer_ctx = system_program::Transfer {
            from: ctx.accounts.payer.to_account_info(),
            to: triggers_ai.clone(),
        };
        system_program::transfer(
            CpiContext::new(ctx.accounts.system_program.to_account_info(), transfer_ctx),
            added_rent,
        )?;
    }

    {
        let bytes = &mut triggers_ai.try_borrow_mut_data()?[prev_bytes_end..];
        let trigger = Trigger::from_bytes_mut(bytes)?;
        trigger.total_bytes = new_bytes_needed.try_into().unwrap();
        trigger.version = 0;
        trigger.condition_bytes = condition.len().try_into().unwrap();
        trigger.action_bytes = action.len().try_into().unwrap();
        trigger.id = trigger_id;
        trigger.expiry_slot = u64::MAX; // TODO: pass in expiry info
        trigger.incentive_lamports = incentive_lamports;
        trigger.condition_was_met = 0;

        let trigger_end = std::mem::size_of::<Trigger>();
        let condition_end = trigger_end + condition.len();

        // Copy the condition and action bytes into the space after the Trigger struct
        let condition_bytes = &mut bytes[trigger_end..condition_end];
        sol_memcpy(condition_bytes, &condition, condition.len());

        let action_bytes = &mut bytes[condition_end..];
        sol_memcpy(action_bytes, &action, action.len());
    }

    // Transfer extra lamports into the trigger account as incentive
    let transfer_ctx = system_program::Transfer {
        from: ctx.accounts.payer.to_account_info(),
        to: ctx.accounts.triggers.to_account_info(),
    };
    system_program::transfer(
        CpiContext::new(ctx.accounts.system_program.to_account_info(), transfer_ctx),
        2 * incentive_lamports,
    )?;

    // TODO: It's better API if setting up the condition and the action are separate instructions,
    // instead of passing opaque blobs of condition and action information here.

    // Verify the condition and action are valid
    let bytes = &triggers_ai.try_borrow_data()?;
    require_eq!(
        Triggers::find_trigger_offset_by_id(bytes, trigger_id)?,
        prev_bytes_end
    );
    let (_triggers, _trigger, condition, action) = Trigger::all_from_bytes(bytes, prev_bytes_end)?;
    action.check()?;

    // TODO: remove logging?
    msg!("cond {:#?}", condition);
    msg!("act {:#?}", action);

    Ok(())
}
