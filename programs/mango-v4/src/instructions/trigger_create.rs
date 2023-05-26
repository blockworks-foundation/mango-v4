use anchor_lang::prelude::*;
use solana_program::program_memory::sol_memcpy;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn trigger_create(
    ctx: Context<TriggerCreate>,
    trigger_num: u64,
    condition: Vec<u8>,
    action: Vec<u8>,
) -> Result<()> {
    // TODO: amount?
    let incentive_lamports = 100_000;

    {
        let mut trigger = ctx.accounts.trigger.load_init()?;
        trigger.group = ctx.accounts.group.key();
        trigger.account = ctx.accounts.account.key();
        trigger.owner = ctx.accounts.owner.key();
        trigger.trigger_num = trigger_num;
        trigger.expiry_slot = u64::MAX; // TODO: pass in expiry info
        trigger.condition_was_met = 0;

        // TODO: copy out condition and action type for fixed-offset access
        trigger.condition_bytes = condition.len().try_into().unwrap();
        trigger.action_bytes = action.len().try_into().unwrap();

        trigger.incentive_lamports = incentive_lamports;
    }

    // Transfer extra lamports into the trigger account as incentive
    **ctx
        .accounts
        .trigger
        .as_ref()
        .try_borrow_mut_lamports()
        .unwrap() += 2 * incentive_lamports;
    **ctx
        .accounts
        .payer
        .as_ref()
        .try_borrow_mut_lamports()
        .unwrap() -= 2 * incentive_lamports;

    // TODO: It's better API if setting up the condition and the action are separate instructions,
    // instead of passing opaque blobs of condition and action information here.

    {
        let mut bytes = ctx.accounts.trigger.as_ref().try_borrow_mut_data()?;
        let fixed_struct_end = 8 + std::mem::size_of::<Trigger>();

        // Copy the condition and action bytes into the space after the Trigger struct
        let condition_bytes = &mut bytes[fixed_struct_end..fixed_struct_end + condition.len()];
        sol_memcpy(condition_bytes, &condition, condition.len());

        let condition_end = fixed_struct_end + condition.len();
        let action_bytes = &mut bytes[condition_end..condition_end + action.len()];
        sol_memcpy(action_bytes, &action, action.len());
    }

    // Verify the condition and action are valid
    let trigger_bytes = ctx.accounts.trigger.as_ref().try_borrow_data()?;
    let (fixed, condition, action) = Trigger::from_account_bytes(&trigger_bytes)?;

    // TODO: remove logging?
    msg!("cond {:#?}", condition);
    msg!("act {:#?}", action);

    Ok(())
}
