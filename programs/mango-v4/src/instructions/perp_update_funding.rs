use anchor_lang::prelude::*;

use crate::accounts_zerocopy::*;
use crate::error::MangoError;
use crate::state::{BookSide, Group, IxGate, Orderbook, PerpMarket};

#[derive(Accounts)]
pub struct PerpUpdateFunding<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpUpdateFunding) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>, // Required for group metadata parsing

    #[account(
        mut,
        has_one = group,
        has_one = bids,
        has_one = asks,
        has_one = oracle,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub bids: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub asks: AccountLoader<'info, BookSide>,

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked above
    pub oracle: UncheckedAccount<'info>,
}
pub fn perp_update_funding(ctx: Context<PerpUpdateFunding>) -> Result<()> {
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let book = Orderbook {
        bids: ctx.accounts.bids.load_mut()?,
        asks: ctx.accounts.asks.load_mut()?,
    };

    let now_slot = Clock::get()?.slot;
    let oracle_price = perp_market.oracle_price(
        &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
        Some(now_slot),
    )?;

    perp_market.update_funding_and_stable_price(&book, oracle_price, now_ts)?;

    Ok(())
}
