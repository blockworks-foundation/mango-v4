use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn trigger_action_execute<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, TriggerActionExecute<'info>>,
    num_condition_accounts: u8,
) -> Result<()> {
    let num_condition_accounts: usize = num_condition_accounts.into();

    {
        let trigger_action_bytes = ctx.accounts.trigger_action.as_ref().try_borrow_data()?;
        let (trigger, condition, action) =
            TriggerAction::from_account_bytes(&trigger_action_bytes)?;

        let condition_accounts = &ctx.remaining_accounts[..num_condition_accounts];
        condition.check(condition_accounts)?;

        let action_accounts = &ctx.remaining_accounts[num_condition_accounts..];
        action.execute(trigger, ctx.accounts, action_accounts)?;
    }

    // Transfer all the lamports on the trigger action account to the triggerer
    let trigger_action_ai = ctx.accounts.trigger_action.as_ref();
    let mut acc_lamports = trigger_action_ai.try_borrow_mut_lamports().unwrap();
    **ctx
        .accounts
        .triggerer
        .as_ref()
        .try_borrow_mut_lamports()
        .unwrap() += **acc_lamports;
    **acc_lamports = 0;

    // Close trigger action account
    trigger_action_ai.assign(&solana_program::system_program::ID);
    trigger_action_ai.realloc(0, false)?;

    Ok(())
}
