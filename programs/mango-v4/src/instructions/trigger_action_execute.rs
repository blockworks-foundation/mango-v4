use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn trigger_action_execute(ctx: Context<TriggerActionExecute>) -> Result<()> {
    let trigger_action_bytes = ctx.accounts.trigger_action.as_ref().try_borrow_data()?;
    let (trigger, condition, action) = TriggerAction::from_account_bytes(&trigger_action_bytes)?;

    match condition {
        ConditionRef::OraclePrice(c) => {
            // TODO: check condition
        }
    };

    match action {
        ActionRef::PerpPlaceOrder(a) => {
            // TODO: actual execution
        }
    }

    Ok(())
}
