use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::{Book, Group, MangoAccount, PerpMarket};

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
    pub asks: UncheckedAccount<'info>,
    #[account(mut)]
    pub bids: UncheckedAccount<'info>,

    pub owner: Signer<'info>,
}

pub fn perp_cancel_all_orders(ctx: Context<PerpCancelAllOrders>, limit: u8) -> Result<()> {
    let mut mango_account = ctx.accounts.account.load_mut()?;
    require!(mango_account.is_bankrupt == 0, MangoError::IsBankrupt);

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let bids = &ctx.accounts.bids.to_account_info();
    let asks = &ctx.accounts.asks.to_account_info();
    let mut book = Book::load_mut(bids, asks, &perp_market)?;

    book.cancel_all_order(&mut mango_account, &mut perp_market, limit, None)?;

    Ok(())
}
