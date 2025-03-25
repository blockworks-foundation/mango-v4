use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::MangoError;
use crate::state::*;

pub fn perp_purge_orders(ctx: Context<PerpPurgeOrders>, limit: u8) -> Result<()> {
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    // only allow pruning orders when market is in force-close
    require!(perp_market.is_force_close(), MangoError::SomeError);

    let mut account = ctx.accounts.account.load_full_mut()?;
    let mut book = Orderbook {
        bids: ctx.accounts.bids.load_mut()?,
        asks: ctx.accounts.asks.load_mut()?,
    };

    book.cancel_all_orders(
        &mut account.borrow_mut(),
        ctx.accounts.account.as_ref().key,
        &mut perp_market,
        limit,
        None,
    )
}
