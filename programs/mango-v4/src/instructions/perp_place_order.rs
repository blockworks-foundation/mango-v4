use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::{
    oracle_price, Book, EventQueueHeader, Group, MangoAccount, OrderType, PerpMarket, Queue, Side,
};

#[derive(Accounts)]
pub struct PerpPlaceOrder<'info> {
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
        has_one = asks,
        has_one = event_queue,
        has_one = oracle,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub asks: UncheckedAccount<'info>,
    #[account(mut)]
    pub bids: UncheckedAccount<'info>,
    #[account(mut)]
    pub event_queue: AccountLoader<'info, Queue<EventQueueHeader>>,

    pub oracle: UncheckedAccount<'info>,

    pub owner: Signer<'info>,
}

// TODO
#[allow(clippy::too_many_arguments)]
pub fn perp_place_order(
    ctx: Context<PerpPlaceOrder>,
    side: Side,
    price: i64,
    max_base_quantity: i64,
    max_quote_quantity: i64,
    client_order_id: u64,
    order_type: OrderType,
    expiry_timestamp: u64,
    limit: u8,
) -> Result<()> {
    let mut mango_account = ctx.accounts.account.load_mut()?;
    require!(mango_account.is_bankrupt == 0, MangoError::IsBankrupt);
    let mango_account_pk = ctx.accounts.account.key();

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let bids = &ctx.accounts.bids.to_account_info();
    let asks = &ctx.accounts.asks.to_account_info();
    let mut book = Book::load_mut(bids, asks, &perp_market)?;

    let mut event_queue = ctx.accounts.event_queue.load_mut()?;

    let oracle_price = oracle_price(&ctx.accounts.oracle.to_account_info())?;

    let now_ts = Clock::get()?.unix_timestamp as u64;
    let time_in_force = if expiry_timestamp != 0 {
        // If expiry is far in the future, clamp to 255 seconds
        let tif = expiry_timestamp.saturating_sub(now_ts).min(255);
        if tif == 0 {
            // If expiry is in the past, ignore the order
            msg!("Order is already expired");
            return Ok(());
        }
        tif as u8
    } else {
        // Never expire
        0
    };

    // TODO reduce_only based on event queue

    book.new_order(
        side,
        &mut perp_market,
        &mut event_queue,
        oracle_price,
        &mut mango_account,
        &mango_account_pk,
        price,
        max_base_quantity,
        max_quote_quantity,
        order_type,
        time_in_force,
        client_order_id,
        now_ts,
        limit,
    )?;

    Ok(())
}
