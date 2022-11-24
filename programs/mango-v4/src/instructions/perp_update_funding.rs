use anchor_lang::prelude::*;

use crate::accounts_zerocopy::*;
use crate::state::{Group, Orderbook, PerpMarket};

#[derive(Accounts)]
pub struct PerpUpdateFunding<'info> {
    pub group: AccountLoader<'info, Group>, // Required for group metadata parsing

    #[account(
        mut,
        has_one = orderbook,
        has_one = oracle,
        constraint = perp_market.load()?.group.key() == group.key(),
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub orderbook: AccountLoader<'info, Orderbook>,

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked above
    pub oracle: UncheckedAccount<'info>,
}
pub fn perp_update_funding(ctx: Context<PerpUpdateFunding>) -> Result<()> {
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let book = ctx.accounts.orderbook.load_mut()?;

    let now_slot = Clock::get()?.slot;
    let oracle_price = perp_market.oracle_price(
        &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
        Some(now_slot),
    )?;

    perp_market.update_funding_and_stable_price(&book, oracle_price, now_ts)?;

    Ok(())
}
