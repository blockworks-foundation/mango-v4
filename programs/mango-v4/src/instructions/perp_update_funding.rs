use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::accounts_zerocopy::*;
use crate::state::*;

pub fn perp_update_funding(ctx: Context<PerpUpdateFunding>) -> Result<()> {
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let book = Orderbook {
        bids: ctx.accounts.bids.load_mut()?,
        asks: ctx.accounts.asks.load_mut()?,
    };

    let now_slot = Clock::get()?.slot;
    let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
    let oracle_state =
        perp_market.oracle_state(&OracleAccountInfos::from_reader(oracle_ref), Some(now_slot))?;

    perp_market.update_funding_and_stable_price(&book, &oracle_state, now_ts)?;

    Ok(())
}
