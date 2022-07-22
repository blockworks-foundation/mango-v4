use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::{
    Book, BookSide, Group, MangoAccount2, MangoAccountAccMut, MangoAccountLoader, PerpMarket,
};

#[derive(Accounts)]
pub struct PerpCancelOrder<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut)]
    pub account: UncheckedAccount<'info>,
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
    let mut mal: MangoAccountLoader<MangoAccount2> =
        MangoAccountLoader::new(&ctx.accounts.account)?;
    let mut account: MangoAccountAccMut = mal.load_mut()?;
    require_keys_eq!(account.fixed.group, ctx.accounts.group.key());
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    require!(!account.fixed.is_bankrupt(), MangoError::IsBankrupt);

    let perp_market = ctx.accounts.perp_market.load_mut()?;
    let bids = ctx.accounts.bids.load_mut()?;
    let asks = ctx.accounts.asks.load_mut()?;
    let mut book = Book::new(bids, asks);

    let side = account
        .perp_find_order_side(perp_market.perp_market_index, order_id)
        .ok_or_else(|| error!(MangoError::SomeError))?; // InvalidOrderId

    let order = book.cancel_order(order_id, side)?;
    require!(
        order.owner == ctx.accounts.account.key(),
        MangoError::SomeError // InvalidOwner
    );

    account.perp_remove_order(order.owner_slot as usize, order.quantity)
}
