use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::{AccountLoaderDynamic, Book2, BookSide, Group, MangoAccount, PerpMarket, Side};

#[derive(Accounts)]
pub struct PerpCancelAllOrdersBySide<'info> {
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

pub fn perp_cancel_all_orders_by_side(
    ctx: Context<PerpCancelAllOrdersBySide>,
    side_option: Option<Side>,
    limit: u8,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_mut()?;
    require_keys_eq!(account.fixed.group, ctx.accounts.group.key());
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = Book2::load_mut(
        &ctx.accounts.bids_direct,
        &ctx.accounts.asks_direct,
        &ctx.accounts.bids_oracle_pegged,
        &ctx.accounts.asks_oracle_pegged,
    )?;

    book.cancel_all_orders(
        &mut account.borrow_mut(),
        &mut perp_market,
        limit,
        side_option,
    )?;

    Ok(())
}
