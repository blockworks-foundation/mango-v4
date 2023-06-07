use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::{new_fixed_order_account_retriever, new_health_cache};
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_stop_loss_trigger(ctx: Context<TokenStopLossTrigger>) -> Result<()> {
    let liqor = ctx.accounts.liqor.load()?;
    // account constraint #1
    require!(
        liqor.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    // load stop loss by index
    // check that banks match
    // check price condition
    // check amount
    // token_liq-like transfer (max amount based on config and health)
    // record amount, maybe remove tsl

    Ok(())
}
