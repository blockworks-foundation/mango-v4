use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::{AccountLoaderDynamic, Group, MangoAccount, OrderBook, PerpMarket};

#[derive(Accounts)]
pub struct PerpCancelOrder<'info> {
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

pub fn perp_cancel_order(ctx: Context<PerpCancelOrder>, order_id: u128) -> Result<()> {
    let mut account = ctx.accounts.account.load_mut()?;
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = ctx.accounts.orderbook.load_mut()?;

    let oo = account
        .perp_find_order_with_order_id(perp_market.perp_market_index, order_id)
        .ok_or_else(|| {
            error_msg!("could not find perp order with id {order_id} in perp market orderbook")
        })?;
    let order_id = oo.id;
    let order_side_and_component = oo.side_and_component;
    drop(oo);

    book.cancel_order(
        &mut account.borrow_mut(),
        order_id,
        order_side_and_component,
        Some(ctx.accounts.account.key()),
    )?;

    Ok(())
}
