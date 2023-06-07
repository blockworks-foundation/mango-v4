use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::{new_fixed_order_account_retriever, new_health_cache};
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_stop_loss_trigger(ctx: Context<TokenStopLossTrigger>) -> Result<()> {
    Ok(())
}
