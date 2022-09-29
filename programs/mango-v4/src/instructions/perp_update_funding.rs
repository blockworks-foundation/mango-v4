use anchor_lang::prelude::*;

use crate::accounts_zerocopy::*;
use crate::state::{Book, BookSide, Group, PerpMarket};

use crate::logs::PerpUpdateFundingLog;

#[derive(Accounts)]
pub struct PerpUpdateFunding<'info> {
    pub group: AccountLoader<'info, Group>, // Required for group metadata parsing

    #[account(
        mut,
        has_one = bids,
        has_one = asks,
        has_one = oracle,
        constraint = perp_market.load()?.group.key() == group.key(),
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub asks: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub bids: AccountLoader<'info, BookSide>,

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked above
    pub oracle: UncheckedAccount<'info>,
}
pub fn perp_update_funding(ctx: Context<PerpUpdateFunding>) -> Result<()> {
    // TODO: should we enforce a minimum window between 2 update_funding ix calls?
    let now_ts = Clock::get()?.unix_timestamp;

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let bids = ctx.accounts.bids.load_mut()?;
    let asks = ctx.accounts.asks.load_mut()?;
    let book = Book::new(bids, asks);

    let oracle_price =
        perp_market.oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?)?;

    perp_market.update_funding(&book, oracle_price, now_ts as u64)?;

    emit!(PerpUpdateFundingLog {
        mango_group: ctx.accounts.group.key(),
        market_index: perp_market.perp_market_index,
        long_funding: perp_market.long_funding.to_bits(),
        short_funding: perp_market.long_funding.to_bits(),
        price: oracle_price.to_bits()
    });

    Ok(())
}
