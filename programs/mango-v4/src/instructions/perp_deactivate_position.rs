use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn perp_deactivate_position(ctx: Context<PerpDeactivatePosition>) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;
    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let perp_market = ctx.accounts.perp_market.load()?;
    let perp_position = account.perp_position_mut(perp_market.perp_market_index)?;

    // Is the perp position closable?
    perp_position.settle_funding(&perp_market);
    require_msg!(
        perp_position.base_position_lots() == 0,
        "perp position still has base lots"
    );
    // No dusting needed because we're able to use settle_pnl to get this to 0.
    require_msg!(
        perp_position.quote_position_native() == 0,
        "perp position still has quote position"
    );
    require_msg!(
        perp_position.bids_base_lots == 0 && perp_position.asks_base_lots == 0,
        "perp position still has open orders"
    );
    require_msg!(
        perp_position.taker_base_lots == 0 && perp_position.taker_quote_lots == 0,
        "perp position still has events on event queue"
    );

    account.deactivate_perp_position_and_log(
        perp_market.perp_market_index,
        perp_market.settle_token_index,
        ctx.accounts.account.key(),
    )?;

    Ok(())
}
