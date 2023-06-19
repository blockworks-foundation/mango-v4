use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_conditional_swap_create(
    ctx: Context<AccountAndAuthority>,
    token_conditional_swap: TokenConditionalSwap,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    require!(
        group.is_ix_enabled(IxGate::TokenConditionalSwapCreate),
        MangoError::IxIsDisabled
    );

    let mut account = ctx.accounts.account.load_full_mut()?;

    let id = account.fixed.next_conditional_swap_id;
    account.fixed.next_conditional_swap_id = account.fixed.next_conditional_swap_id.wrapping_add(1);

    let tcs = account.add_token_conditional_swap()?;
    *tcs = token_conditional_swap;
    tcs.id = id;
    tcs.taker_fee_bps = group.token_conditional_swap_taker_fee_bps;
    tcs.maker_fee_bps = group.token_conditional_swap_maker_fee_bps;

    Ok(())
}
