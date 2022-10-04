use anchor_lang::prelude::*;

use crate::accounts_zerocopy::*;
use crate::state::{Book2, BookSide, Group, PerpMarket};

#[derive(Accounts)]
pub struct PerpUpdateFunding<'info> {
    pub group: AccountLoader<'info, Group>, // Required for group metadata parsing

    #[account(
        mut,
        has_one = bids_direct,
        has_one = asks_direct,
        has_one = bids_oracle_pegged,
        has_one = asks_oracle_pegged,
        has_one = oracle,
        constraint = perp_market.load()?.group.key() == group.key(),
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub asks_direct: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub bids_direct: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub asks_oracle_pegged: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub bids_oracle_pegged: AccountLoader<'info, BookSide>,

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked above
    pub oracle: UncheckedAccount<'info>,
}
pub fn perp_update_funding(ctx: Context<PerpUpdateFunding>) -> Result<()> {
    // TODO: should we enforce a minimum window between 2 update_funding ix calls?
    let now_ts = Clock::get()?.unix_timestamp;

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = Book2::load_mut(
        &ctx.accounts.bids_direct,
        &ctx.accounts.asks_direct,
        &ctx.accounts.bids_oracle_pegged,
        &ctx.accounts.asks_oracle_pegged,
    )?;

    let oracle_price =
        perp_market.oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?)?;

    perp_market.update_funding(&book, oracle_price, now_ts as u64)?;

    Ok(())
}
