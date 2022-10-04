use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::{AccountLoaderDynamic, Book2, BookSide, Group, MangoAccount, PerpMarket};

#[derive(Accounts)]
pub struct PerpCancelOrderByClientOrderId<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = bids_direct,
        has_one = asks_direct,
        has_one = bids_oracle_pegged,
        has_one = asks_oracle_pegged,
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
}

pub fn perp_cancel_order_by_client_order_id(
    ctx: Context<PerpCancelOrderByClientOrderId>,
    client_order_id: u64,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_mut()?;
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = Book2::load_mut(
        &ctx.accounts.bids_direct,
        &ctx.accounts.asks_direct,
        &ctx.accounts.bids_oracle_pegged,
        &ctx.accounts.asks_oracle_pegged,
    )?;

    let oo = account
        .perp_find_order_with_client_order_id(perp_market.perp_market_index, client_order_id)
        .ok_or_else(|| error_msg!("could not find perp order with client order id {client_order_id} in perp order books"))?;

    book.cancel_order(
        &mut account.borrow_mut(),
        oo.order_id,
        oo.order_side,
        oo.book_component,
        Some(ctx.accounts.account.key()),
    )?;

    Ok(())
}
