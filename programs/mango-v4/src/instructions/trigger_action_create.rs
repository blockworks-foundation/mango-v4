use anchor_lang::prelude::*;
use solana_program::program_memory::sol_memcpy;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn trigger_action_create(
    ctx: Context<TriggerActionCreate>,
    trigger_num: u64,
    condition: Vec<u8>,
    action: Vec<u8>,
) -> Result<()> {
    {
        let mut trigger_action = ctx.accounts.trigger_action.load_init()?;
        trigger_action.group = ctx.accounts.group.key();
        trigger_action.account = ctx.accounts.account.key();
        trigger_action.owner = ctx.accounts.owner.key();
        trigger_action.trigger_num = trigger_num;

        // TODO: copy out condition and action type for fixed-offset access
        trigger_action.condition_bytes = condition.len().try_into().unwrap();
        trigger_action.action_bytes = action.len().try_into().unwrap();
    }

    // TODO: It's better API if setting up the condition and the action are separate instructions,
    // instead of passing opaque blobs of condition and action information here.

    {
        let mut bytes = ctx.accounts.trigger_action.as_ref().try_borrow_mut_data()?;
        let fixed_struct_end = 8 + std::mem::size_of::<TriggerAction>();

        // Copy the condition and action bytes into the space after the TriggerAction struct
        let condition_bytes = &mut bytes[fixed_struct_end..fixed_struct_end + condition.len()];
        sol_memcpy(condition_bytes, &condition, condition.len());

        let condition_end = fixed_struct_end + condition.len();
        let action_bytes = &mut bytes[condition_end..condition_end + action.len()];
        sol_memcpy(action_bytes, &action, action.len());
    }

    // Verify the condition and action are valid
    let trigger_action_bytes = ctx.accounts.trigger_action.as_ref().try_borrow_data()?;
    let (fixed, condition, action) = TriggerAction::from_account_bytes(&trigger_action_bytes)?;

    // TODO: remove logging?
    msg!("cond {:#?}", condition);
    msg!("act {:#?}", action);

    Ok(())
}
