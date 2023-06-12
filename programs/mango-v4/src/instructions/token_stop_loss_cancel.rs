use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_stop_loss_cancel(
    ctx: Context<AccountAndAuthority>,
    token_stop_loss_index: usize,
) -> Result<()> {
    require!(
        ctx.accounts
            .group
            .load()?
            .is_ix_enabled(IxGate::TokenStopLossCancel),
        MangoError::IxIsDisabled
    );

    let mut account = ctx.accounts.account.load_full_mut()?;
    let tsl = account.token_stop_loss_mut_by_index(token_stop_loss_index)?;
    // If the tsl is already inactive, this just is a noop
    *tsl = TokenStopLoss::default();

    Ok(())
}
