use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_stop_loss_create(
    ctx: Context<TokenStopLossCreate>,
    // TODO: args
) -> Result<()> {
    Ok(())
}
