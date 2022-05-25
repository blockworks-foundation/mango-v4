use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::{
    compute_health_from_fixed_accounts, oracle_price, Book, EventQueue, Group, HealthType,
    MangoAccount, OrderType, PerpMarket, Side,
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
    pub event_queue: AccountLoader<'info, EventQueue>,

    pub oracle: UncheckedAccount<'info>,

    pub owner: Signer<'info>,
}

// TODO
#[allow(clippy::too_many_arguments)]
pub fn perp_place_order(
    ctx: Context<PerpPlaceOrder>,
    side: Side,

    // Price in quote lots per base lots.
    //
    // Effect is based on order type, it's usually
    // - fill orders on the book up to this price or
    // - place an order on the book at this price.
    //
    // Ignored for Market orders and potentially adjusted for PostOnlySlide orders.
    price_lots: i64,

    // Max base lots to buy/sell.
    max_base_lots: i64,

    // Max quote lots to pay/receive (not taking fees into account).
    max_quote_lots: i64,

    // Arbitrary user-controlled order id.
    client_order_id: u64,

    order_type: OrderType,

    // Timestamp of when order expires
    //
    // Send 0 if you want the order to never expire.
    // Timestamps in the past mean the instruction is skipped.
    // Timestamps in the future are reduced to now + 255s.
    expiry_timestamp: u64,

    // Maximum number of orders from the book to fill.
    //
    // Use this to limit compute used during order matching.
    // When the limit is reached, processing stops and the instruction succeeds.
    limit: u8,
) -> Result<()> {
    let mut mango_account = ctx.accounts.account.load_mut()?;
    require!(mango_account.is_bankrupt == 0, MangoError::IsBankrupt);
    let mango_account_pk = ctx.accounts.account.key();

    {
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
            &mut mango_account.perps,
            &mango_account_pk,
            price_lots,
            max_base_lots,
            max_quote_lots,
            order_type,
            time_in_force,
            client_order_id,
            now_ts,
            limit,
        )?;
    }

    let health = compute_health_from_fixed_accounts(
        &mango_account,
        HealthType::Init,
        ctx.remaining_accounts,
    )?;
    msg!("health: {}", health);
    require!(health >= 0, MangoError::HealthMustBePositive);

    Ok(())
}
