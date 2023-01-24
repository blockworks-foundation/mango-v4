use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::{
    BookSide, Group, IxGate, MangoAccountFixed, MangoAccountLoader, Orderbook, PerpMarket,
};

#[derive(Accounts)]
pub struct PerpCancelOrder<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpCancelOrder) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // owner is checked at #1
    )]
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

pub fn perp_cancel_order(ctx: Context<PerpCancelOrder>, order_id: u128) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;
    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = Orderbook {
        bids: ctx.accounts.bids.load_mut()?,
        asks: ctx.accounts.asks.load_mut()?,
    };

    let oo = account
        .perp_find_order_with_order_id(perp_market.perp_market_index, order_id)
        .ok_or_else(|| {
            error_msg!("could not find perp order with id {order_id} in perp market orderbook")
        })?;
    let order_id = oo.id;
    let order_side_and_tree = oo.side_and_tree();

    book.cancel_order(
        &mut account.borrow_mut(),
        order_id,
        order_side_and_tree,
        Some(ctx.accounts.account.key()),
    )?;

    Ok(())
}
