use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::{AccountLoaderDynamic, Book, BookSide, Group, MangoAccount, PerpMarket};

#[derive(Accounts)]
pub struct PerpCancelOrder<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,
    pub owner: Signer<'info>,

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
}

pub fn perp_cancel_order(ctx: Context<PerpCancelOrder>, order_id: i128) -> Result<()> {
    let mut account = ctx.accounts.account.load_mut()?;
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let perp_market = ctx.accounts.perp_market.load_mut()?;
    let bids = ctx.accounts.bids.load_mut()?;
    let asks = ctx.accounts.asks.load_mut()?;
    let mut book = Book::new(bids, asks);

    let side = account
        .perp_find_order_side(perp_market.perp_market_index, order_id)
        .ok_or_else(|| {
            error_msg!("could not find perp order with id {order_id} in perp market orderbook")
        })?;

    book.cancel_order(
        &mut account.borrow_mut(),
        order_id,
        side,
        Some(ctx.accounts.account.key()),
    )?;

    Ok(())
}
