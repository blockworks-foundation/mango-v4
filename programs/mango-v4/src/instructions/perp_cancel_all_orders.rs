use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::{
    Book, BookSide, Group, MangoAccount2, MangoAccountAccMut, MangoAccountLoader, PerpMarket,
};

#[derive(Accounts)]
pub struct PerpCancelAllOrders<'info> {
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

pub fn perp_cancel_all_orders(ctx: Context<PerpCancelAllOrders>, limit: u8) -> Result<()> {
    let mut mal: MangoAccountLoader<MangoAccount2> =
        MangoAccountLoader::new(&ctx.accounts.account)?;
    let mut account: MangoAccountAccMut = mal.load_mut()?;
    require_keys_eq!(account.fixed.group, ctx.accounts.group.key());
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    require!(!account.fixed.is_bankrupt(), MangoError::IsBankrupt);

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let bids = ctx.accounts.bids.load_mut()?;
    let asks = ctx.accounts.asks.load_mut()?;
    let mut book = Book::new(bids, asks);

    book.cancel_all_order(&mut account, &mut perp_market, limit, None)?;

    Ok(())
}
