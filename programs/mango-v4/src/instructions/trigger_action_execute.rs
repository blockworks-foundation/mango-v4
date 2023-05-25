use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn trigger_action_execute(
    ctx: Context<TriggerActionExecute>,
    num_condition_accounts: u8,
) -> Result<()> {
    let num_condition_accounts: usize = num_condition_accounts.into();

    let trigger_action_bytes = ctx.accounts.trigger_action.as_ref().try_borrow_data()?;
    let (trigger, condition, action) = TriggerAction::from_account_bytes(&trigger_action_bytes)?;

    let condition_accounts = &ctx.remaining_accounts[..num_condition_accounts];
    condition.check(condition_accounts)?;

    let action_accounts = &ctx.remaining_accounts[num_condition_accounts..];
    action.execute(trigger, ctx.accounts, action_accounts)?;

    // TODO: incentivize triggerer
    // TODO: close trigger action account

    Ok(())
}
