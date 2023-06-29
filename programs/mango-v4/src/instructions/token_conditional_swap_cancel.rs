use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::logs::TokenConditionalSwapCancelLog;
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_conditional_swap_cancel(
    ctx: Context<AccountAndAuthority>,
    token_conditional_swap_index: usize,
    token_conditional_swap_id: u64,
) -> Result<()> {
    require!(
        ctx.accounts
            .group
            .load()?
            .is_ix_enabled(IxGate::TokenConditionalSwapCancel),
        MangoError::IxIsDisabled
    );

    let mut account = ctx.accounts.account.load_full_mut()?;
    let tcs = account.token_conditional_swap_mut_by_index(token_conditional_swap_index)?;

    // If the tcs is already inactive, this just is a noop
    if !tcs.has_data() {
        return Ok(());
    }

    require_eq!(tcs.id, token_conditional_swap_id);
    *tcs = TokenConditionalSwap::default();

    emit!(TokenConditionalSwapCancelLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        id: token_conditional_swap_id,
    });

    Ok(())
}
