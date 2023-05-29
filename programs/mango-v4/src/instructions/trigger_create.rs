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
        let account = ctx.accounts.account.load()?;
        // account constraint #1
        require!(
            account.is_owner_or_delegate(ctx.accounts.authority.key()),
            MangoError::SomeError
        );
    }

    {
        let mut trigger = ctx.accounts.trigger.load_init()?;
        trigger.group = ctx.accounts.group.key();
        trigger.account = ctx.accounts.account.key();
        trigger.trigger_num = trigger_num;
        trigger.expiry_slot = u64::MAX; // TODO: pass in expiry info
        trigger.condition_was_met = 0;

        require_gte!(condition.len(), 4);
        require_gte!(action.len(), 4);
        trigger.condition_bytes = condition.len().try_into().unwrap();
        trigger.action_bytes = action.len().try_into().unwrap();

        trigger.incentive_lamports = incentive_lamports;
    }
    // ensure the discriminator is written
    ctx.accounts.trigger.exit(ctx.program_id)?;

    // Transfer extra lamports into the trigger account as incentive
    use anchor_lang::system_program;
    let transfer_ctx = system_program::Transfer {
        from: ctx.accounts.payer.to_account_info(),
        to: ctx.accounts.trigger.to_account_info(),
    };
    system_program::transfer(
        CpiContext::new(ctx.accounts.system_program.to_account_info(), transfer_ctx),
        2 * incentive_lamports,
    )?;

    // TODO: It's better API if setting up the condition and the action are separate instructions,
    // instead of passing opaque blobs of condition and action information here.

    // Copy the condition and action information into the account.
    // The types are read _after_ copying the bytes because previously, there was no guarantee of alignment!
    let condition_type;
    let action_type;
    {
        let mut bytes = ctx.accounts.trigger.as_ref().try_borrow_mut_data()?;
        let fixed_struct_end = 8 + std::mem::size_of::<Trigger>();

        // Copy the condition and action bytes into the space after the Trigger struct
        let condition_bytes = &mut bytes[fixed_struct_end..fixed_struct_end + condition.len()];
        sol_memcpy(condition_bytes, &condition, condition.len());
        condition_type = *bytemuck::from_bytes(&condition_bytes[..4]);

        let condition_end = fixed_struct_end + condition.len();
        let action_bytes = &mut bytes[condition_end..condition_end + action.len()];
        sol_memcpy(action_bytes, &action, action.len());
        action_type = *bytemuck::from_bytes(&action_bytes[..4]);
    }

    {
        let mut trigger = ctx.accounts.trigger.load_mut()?;
        trigger.condition_type = condition_type;
        trigger.action_type = action_type;
    }

    // Verify the condition and action are valid
    let trigger_bytes = ctx.accounts.trigger.as_ref().try_borrow_data()?;
    let (fixed, condition, action) = Trigger::from_account_bytes(&trigger_bytes)?;
    action.check()?;

    // TODO: remove logging?
    msg!("cond {:#?}", condition);
    msg!("act {:#?}", action);

    Ok(())
}
