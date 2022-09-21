use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct PerpDeactivatePosition<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group
        // owner is checked at #1
    )]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(has_one = group)]
    pub perp_market: AccountLoader<'info, PerpMarket>,
}

pub fn perp_deactivate_position(ctx: Context<PerpDeactivatePosition>) -> Result<()> {
    let mut account = ctx.accounts.account.load_mut()?;
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

    account.deactivate_perp_position(perp_market.perp_market_index, QUOTE_TOKEN_INDEX)?;

    Ok(())
}
