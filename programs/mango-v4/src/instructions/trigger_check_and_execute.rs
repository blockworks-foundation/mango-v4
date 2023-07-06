use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn trigger_check_and_execute<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, TriggerCheck<'info>>,
    trigger_id: u64,
    num_condition_accounts: u8,
) -> Result<()> {
    let num_condition_accounts: usize = num_condition_accounts.into();

    require!(
        ctx.accounts
            .group
            .load()?
            .is_ix_enabled(IxGate::TriggerCheckAndExecute),
        MangoError::IxIsDisabled
    );

    // just to ensure the account is good
    ctx.accounts.triggers.load()?;

    let triggers_ai = ctx.accounts.triggers.as_ref();

    let (new_account_size, incentive_lamports) = {
        let mut bytes = triggers_ai.try_borrow_mut_data()?;
        let trigger_offset = Triggers::find_trigger_offset_by_id(&bytes, trigger_id)?;
        let (triggers, trigger, condition, action) =
            Trigger::all_from_bytes(&bytes, trigger_offset)?;

        let now_slot = Clock::get()?.slot;
        require_gt!(trigger.expiry_slot, now_slot);

        let condition_accounts = &ctx.remaining_accounts[..num_condition_accounts];
        condition.check(condition_accounts)?;

        let action_accounts = &ctx.remaining_accounts[num_condition_accounts..];
        action.execute(triggers, ctx.accounts, action_accounts)?;

        Triggers::remove_trigger(&mut bytes, trigger_id, trigger_offset)?
    };

    triggers_ai.realloc(new_account_size, false)?;
    Triggers::transfer_lamports(triggers_ai, &ctx.accounts.triggerer, incentive_lamports)?;

    Ok(())
}
