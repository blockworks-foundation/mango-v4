use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::{BookSide, Group, MangoAccountFixed, MangoAccountLoader, Orderbook, PerpMarket};

#[derive(Accounts)]
pub struct PerpCancelAllOrders<'info> {
    #[account(
        constraint = group.load()?.is_operational()
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = bids,
        has_one = asks,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub bids: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub asks: AccountLoader<'info, BookSide>,
}

pub fn perp_cancel_all_orders(ctx: Context<PerpCancelAllOrders>, limit: u8) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = Orderbook {
        bids: ctx.accounts.bids.load_mut()?,
        asks: ctx.accounts.asks.load_mut()?,
    };

    book.cancel_all_orders(&mut account.borrow_mut(), &mut perp_market, limit, None)?;

    Ok(())
}
