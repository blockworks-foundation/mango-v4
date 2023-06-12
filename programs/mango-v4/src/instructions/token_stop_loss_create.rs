use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_stop_loss_create(
    ctx: Context<AccountAndAuthority>,
    token_stop_loss: TokenStopLoss,
) -> Result<()> {
    require!(
        ctx.accounts
            .group
            .load()?
            .is_ix_enabled(IxGate::TokenStopLossCreate),
        MangoError::IxIsDisabled
    );

    let mut account = ctx.accounts.account.load_full_mut()?;

    let id = account.fixed.next_stop_loss_id;
    account.fixed.next_stop_loss_id = account.fixed.next_stop_loss_id.wrapping_add(1);

    let tsl = account.add_token_stop_loss()?;
    *tsl = token_stop_loss;
    tsl.id = id;

    Ok(())
}
