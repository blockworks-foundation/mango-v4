use anchor_lang::prelude::*;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::{new_fixed_order_account_retriever, new_health_cache};
use crate::state::IxGate;
use crate::state::Side;
use crate::state::{
    BookSide, EventQueue, Group, MangoAccountFixed, MangoAccountLoader, Order, Orderbook,
    PerpMarket,
};

#[derive(Accounts)]
pub struct PerpPlaceOrder<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpPlaceOrder) @ MangoError::IxIsDisabled,
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
        has_one = event_queue,
        has_one = oracle,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub bids: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub asks: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub event_queue: AccountLoader<'info, EventQueue>,

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked above
    pub oracle: UncheckedAccount<'info>,
}

// TODO
#[allow(clippy::too_many_arguments)]
pub fn perp_place_order(
    ctx: Context<PerpPlaceOrder>,
    mut order: Order,
    limit: u8,
) -> Result<Option<u128>> {
    require_gte!(order.max_base_lots, 0);
    require_gte!(order.max_quote_lots, 0);

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    let oracle_price;

    // Update funding if possible.
    //
    // Doing this automatically here makes it impossible for attackers to add orders to the orderbook
    // before triggering the funding computation.
    {
        let mut perp_market = ctx.accounts.perp_market.load_mut()?;
        let book = Orderbook {
            bids: ctx.accounts.bids.load_mut()?,
            asks: ctx.accounts.asks.load_mut()?,
        };

        oracle_price = perp_market.oracle_price(
            &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
            None, // staleness checked in health
        )?;

        perp_market.update_funding_and_stable_price(&book, oracle_price, now_ts)?;
    }

    let mut account = ctx.accounts.account.load_full_mut()?;
    // account constraint #1
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
        let pre_init_health = account.check_health_pre(&health_cache)?;
        Some((health_cache, pre_init_health))
    } else {
        None
    };

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = Orderbook {
        bids: ctx.accounts.bids.load_mut()?,
        asks: ctx.accounts.asks.load_mut()?,
    };

    let mut event_queue = ctx.accounts.event_queue.load_mut()?;

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let pp = account.perp_position(perp_market_index)?;
    let effective_pos = pp.effective_base_position_lots();
    let max_base_lots = if order.reduce_only || perp_market.is_reduce_only() {
        if (order.side == Side::Bid && effective_pos >= 0)
            || (order.side == Side::Ask && effective_pos <= 0)
        {
            0
        } else if order.side == Side::Bid {
            // ignores open asks
            (effective_pos + pp.bids_base_lots)
                .min(0)
                .abs()
                .min(order.max_base_lots)
        } else {
            // ignores open bids
            (effective_pos - pp.asks_base_lots)
                .max(0)
                .min(order.max_base_lots)
        }
    } else {
        order.max_base_lots
    };
    if perp_market.is_reduce_only() {
        require!(
            order.reduce_only || max_base_lots == order.max_base_lots,
            MangoError::MarketInReduceOnlyMode
        )
    };
    order.max_base_lots = max_base_lots;

    let order_id_opt = book.new_order(
        order,
        &mut perp_market,
        &mut event_queue,
        oracle_price,
        &mut account.borrow_mut(),
        &account_pk,
        now_ts,
        limit,
    )?;

    //
    // Health check
    //
    if let Some((mut health_cache, pre_init_health)) = pre_health_opt {
        let perp_position = account.perp_position(perp_market_index)?;
        health_cache.recompute_perp_info(perp_position, &perp_market)?;
        account.check_health_post(&health_cache, pre_init_health)?;
    }

    Ok(order_id_opt)
}
