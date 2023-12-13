use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::MangoError;
use crate::logs::{emit_stack, TokenConditionalSwapCancelLog};
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_conditional_swap_cancel(
    ctx: Context<TokenConditionalSwapCancel>,
    token_conditional_swap_index: usize,
    token_conditional_swap_id: u64,
) -> Result<()> {
    let mut buy_bank = ctx.accounts.buy_bank.load_mut()?;
    let mut sell_bank = ctx.accounts.sell_bank.load_mut()?;

    let mut account = ctx.accounts.account.load_full_mut()?;
    let tcs = account.token_conditional_swap_mut_by_index(token_conditional_swap_index)?;
    require_eq!(tcs.buy_token_index, buy_bank.token_index);
    require_eq!(tcs.sell_token_index, sell_bank.token_index);

    // If the tcs is already inactive, this just is a noop
    if !tcs.is_configured() {
        return Ok(());
    }

    require_eq!(
        tcs.id,
        token_conditional_swap_id,
        MangoError::TokenConditionalSwapIndexIdMismatch
    );
    *tcs = TokenConditionalSwap::default();

    emit_stack(TokenConditionalSwapCancelLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        id: token_conditional_swap_id,
    });

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    // Free up any locks on token positions, possibly dust and deactivate them.
    account.token_decrement_dust_deactivate(&mut buy_bank, now_ts, ctx.accounts.account.key())?;
    account.token_decrement_dust_deactivate(&mut sell_bank, now_ts, ctx.accounts.account.key())?;

    Ok(())
}
