use anchor_lang::prelude::*;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::state::BookSideOrderTree;
use crate::state::MangoAccount;
use crate::state::OrderParams;
use crate::state::{
    new_fixed_order_account_retriever, new_health_cache, AccountLoaderDynamic, EventQueue, Group,
    Order, OrderBook, PerpMarket, PlaceOrderType, SideAndOrderTree,
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
        has_one = orderbook,
        has_one = event_queue,
        has_one = oracle,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub orderbook: AccountLoader<'info, OrderBook>,
    #[account(mut)]
    pub event_queue: AccountLoader<'info, EventQueue>,

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked above
    pub oracle: UncheckedAccount<'info>,
}

// TODO
#[allow(clippy::too_many_arguments)]
pub fn perp_place_order(
    ctx: Context<PerpPlaceOrder>,
    side_and_tree: SideAndOrderTree,

    // Price information, effect is based on order type and component.
    //
    // For Fixed orders it's a literal price in lots (quote lots per base lots)
    // - fill orders on the book up to this price or
    // - place an order on the book at this price.
    // - ignored for Market orders and potentially adjusted for PostOnlySlide orders.
    //
    // For OraclePegged orders its the adjustment from the oracle price, and
    // - orders on the book may be filled at oracle + adjustment (depends on order type)
    // - if an order is placed on the book, it'll be in the oracle-pegged book
    // - the unit is lots (quote lots per base lots)
    price_data: i64,

    // For OraclePegged orders only: the limit at which the pegged order shall expire.
    // May be -1 to denote no peg limit.
    //
    // Example: An bid pegged to -20 with peg_limit 100 would expire if the oracle hits 121.
    peg_limit: i64,

    // Max base lots to buy/sell.
    max_base_lots: i64,

    // Max quote lots to pay/receive (not taking fees into account).
    max_quote_lots: i64,

    // Arbitrary user-controlled order id.
    client_order_id: u64,

    order_type: PlaceOrderType,

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

    let (perp_market_index, settle_token_index) = {
        let perp_market = ctx.accounts.perp_market.load()?;
        (
            perp_market.perp_market_index,
            perp_market.settle_token_index,
        )
    };

    //
    // Create the perp position if needed
    //
    account.ensure_perp_position(perp_market_index, settle_token_index)?;

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
    let mut book = ctx.accounts.orderbook.load_mut()?;

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

    require_gte!(peg_limit, -1);

    let order = Order {
        side: side_and_tree.side(),
        params: match order_type {
            PlaceOrderType::Market => OrderParams::Market,
            PlaceOrderType::ImmediateOrCancel => OrderParams::ImmediateOrCancel {
                price_lots: price_data,
            },
            _ => match side_and_tree.order_tree() {
                BookSideOrderTree::Fixed => OrderParams::Fixed {
                    price_lots: price_data,
                    order_type: order_type.to_post_order_type()?,
                },
                BookSideOrderTree::OraclePegged => OrderParams::OraclePegged {
                    price_offset_lots: price_data,
                    order_type: order_type.to_post_order_type()?,
                    peg_limit,
                },
            },
        },
    };

    book.new_order(
        order,
        &mut perp_market,
        &mut event_queue,
        oracle_price,
        &mut account.borrow_mut(),
        &account_pk,
        max_base_lots,
        max_quote_lots,
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
