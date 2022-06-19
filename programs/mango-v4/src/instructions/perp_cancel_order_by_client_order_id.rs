use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::{Book, Group, MangoAccount, PerpMarket};

#[derive(Accounts)]
pub struct PerpCancelOrderByClientOrderId<'info> {
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

pub fn perp_cancel_order_by_client_order_id(
    ctx: Context<PerpCancelOrderByClientOrderId>,
    client_order_id: u64,
) -> Result<()> {
    let mut mango_account = ctx.accounts.account.load_mut()?;
    require!(mango_account.is_bankrupt == 0, MangoError::IsBankrupt);

    let perp_market = ctx.accounts.perp_market.load_mut()?;
    let bids = ctx.accounts.bids.as_ref();
    let asks = ctx.accounts.asks.as_ref();
    let mut book = Book::load_mut(bids, asks, &perp_market)?;

    let (order_id, side) = mango_account
        .perps
        .find_order_with_client_order_id(perp_market.perp_market_index, client_order_id)
        .ok_or_else(|| error!(MangoError::SomeError))?;

    let order = book.cancel_order(order_id, side)?;
    require!(
        order.owner == ctx.accounts.account.key(),
        MangoError::SomeError // InvalidOwner
    );

    mango_account
        .perps
        .remove_order(order.owner_slot as usize, order.quantity)
}
