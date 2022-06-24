use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::{Book, BookSide, Group, MangoAccount, PerpMarket};

#[derive(Accounts)]
pub struct PerpCancelAllOrders<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    #[account(
        mut,
        has_one = group,
        has_one = bids,
        has_one = asks
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub asks: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub bids: AccountLoader<'info, BookSide>,

    pub owner: Signer<'info>,
}

pub fn perp_cancel_all_orders(ctx: Context<PerpCancelAllOrders>, limit: u8) -> Result<()> {
    let mut mango_account = ctx.accounts.account.load_mut()?;
    require!(mango_account.is_bankrupt == 0, MangoError::IsBankrupt);

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let bids = ctx.accounts.bids.load_mut()?;
    let asks = ctx.accounts.asks.load_mut()?;
    let mut book = Book::new(bids, asks);

    book.cancel_all_order(&mut mango_account, &mut perp_market, limit, None)?;

    Ok(())
}
