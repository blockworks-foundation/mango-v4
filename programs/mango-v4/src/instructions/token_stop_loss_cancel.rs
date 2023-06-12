use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_stop_loss_cancel(
    ctx: Context<AccountAndAuthority>,
    token_stop_loss_index: usize,
    token_stop_loss_id: u64,
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
    if !tsl.is_active() {
        return Ok(());
    }

    require_eq!(tsl.id, token_stop_loss_id);
    *tsl = TokenStopLoss::default();

    Ok(())
}
