use anchor_lang::prelude::*;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::state::MangoAccount;
use crate::state::{
    new_fixed_order_account_retriever, new_health_cache, AccountLoaderDynamic, Book2, BookSide,
    EventQueue, Group, OrderType, PerpMarket, SideAndComponent, QUOTE_TOKEN_INDEX,
};

#[derive(Accounts)]
pub struct PerpPlaceOrder<'info> {
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
        has_one = event_queue,
        has_one = oracle,
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
    #[account(mut)]
    pub event_queue: AccountLoader<'info, EventQueue>,

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked above
    pub oracle: UncheckedAccount<'info>,
}

// TODO
#[allow(clippy::too_many_arguments)]
pub fn perp_place_order(
    ctx: Context<PerpPlaceOrder>,
    side_and_component: SideAndComponent,

    // Price information, effect is based on order type and component.
    //
    // For Direct orders it's a literal price in lots
    // - fill orders on the book up to this price or
    // - place an order on the book at this price.
    // - ignored for Market orders and potentially adjusted for PostOnlySlide orders.
    //
    // For OraclePegged orders its the adjustment from the oracle price, and
    // - orders on the book may be filled at oracle + adjustment (depends on order type)
    // - if an order is placed on the book, it'll be in the oracle-pegged book
    price_data: i64,

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
    let mut account = ctx.accounts.account.load_mut()?;
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let account_pk = ctx.accounts.account.key();

    let perp_market_index = ctx.accounts.perp_market.load()?.perp_market_index;

    //
    // Create the perp position if needed
    //
    account.ensure_perp_position(perp_market_index, QUOTE_TOKEN_INDEX)?;

    //
    // Pre-health computation, _after_ perp position is created
    //
    let pre_health_opt = if !account.fixed.is_in_health_region() {
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
        let health_cache =
            new_health_cache(&account.borrow(), &retriever).context("pre-withdraw init health")?;
        let pre_health = account.check_health_pre(&health_cache)?;
        Some((health_cache, pre_health))
    } else {
        None
    };

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = Book2::load_mut(
        &ctx.accounts.bids_direct,
        &ctx.accounts.asks_direct,
        &ctx.accounts.bids_oracle_pegged,
        &ctx.accounts.asks_oracle_pegged,
    )?;

    let mut event_queue = ctx.accounts.event_queue.load_mut()?;

    let oracle_price =
        perp_market.oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?)?;

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
        side_and_component,
        &mut perp_market,
        &mut event_queue,
        oracle_price,
        &mut account.borrow_mut(),
        &account_pk,
        price_data,
        max_base_lots,
        max_quote_lots,
        order_type,
        time_in_force,
        client_order_id,
        now_ts,
        limit,
    )?;

    //
    // Health check
    //
    if let Some((mut health_cache, pre_health)) = pre_health_opt {
        let perp_position = account.perp_position(perp_market_index)?;
        health_cache.recompute_perp_info(perp_position, &perp_market)?;
        account.check_health_post(&health_cache, pre_health)?;
    }

    Ok(())
}
