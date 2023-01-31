use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::{
    BookSide, Group, IxGate, MangoAccountFixed, MangoAccountLoader, Orderbook, PerpMarket, Side,
};

#[derive(Accounts)]
pub struct PerpCancelAllOrdersBySide<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpCancelAllOrdersBySide) @ MangoError::IxIsDisabled,
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

pub fn perp_cancel_all_orders_by_side(
    ctx: Context<PerpCancelAllOrdersBySide>,
    side_option: Option<Side>,
    limit: u8,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;
    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = Orderbook {
        bids: ctx.accounts.bids.load_mut()?,
        asks: ctx.accounts.asks.load_mut()?,
    };

    book.cancel_all_orders(
        &mut account.borrow_mut(),
        &mut perp_market,
        limit,
        side_option,
    )?;

    Ok(())
}
