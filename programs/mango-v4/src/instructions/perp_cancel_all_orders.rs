use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::{AccountLoaderDynamic, Group, MangoAccount, OrderBook, PerpMarket};

#[derive(Accounts)]
pub struct PerpCancelAllOrders<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = orderbook,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub orderbook: AccountLoader<'info, OrderBook>,
}

pub fn perp_cancel_all_orders(ctx: Context<PerpCancelAllOrders>, limit: u8) -> Result<()> {
    let mut account = ctx.accounts.account.load_mut()?;
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = ctx.accounts.orderbook.load_mut()?;

    book.cancel_all_orders(&mut account.borrow_mut(), &mut perp_market, limit, None)?;

    Ok(())
}
