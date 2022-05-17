use anchor_lang::prelude::*;

use crate::state::{oracle_price, Book, PerpMarket};

#[derive(Accounts)]
pub struct PerpUpdateFunding<'info> {
    #[account(
        mut,
        has_one = bids,
        has_one = asks,
        has_one = oracle,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub asks: UncheckedAccount<'info>,
    #[account(mut)]
    pub bids: UncheckedAccount<'info>,

    pub oracle: UncheckedAccount<'info>,
}
pub fn perp_update_funding(ctx: Context<PerpUpdateFunding>) -> Result<()> {
    // TODO: should we enforce a minimum window between 2 update_funding ix calls?
    let now_ts = Clock::get()?.unix_timestamp;

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let bids = &ctx.accounts.bids.to_account_info();
    let asks = &ctx.accounts.asks.to_account_info();
    let book = Book::load_mut(bids, asks, &perp_market)?;

    let oracle_price = oracle_price(&ctx.accounts.oracle.to_account_info())?;

    perp_market.update_funding(&book, oracle_price, now_ts as u64)?;

    Ok(())
}
