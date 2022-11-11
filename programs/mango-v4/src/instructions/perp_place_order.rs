use anchor_lang::prelude::*;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::state::MangoAccount;
use crate::state::{
    new_fixed_order_account_retriever, new_health_cache, AccountLoaderDynamic, EventQueue, Group,
    Order, OrderBook, PerpMarket,
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
pub fn perp_place_order(ctx: Context<PerpPlaceOrder>, order: Order, limit: u8) -> Result<()> {
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
        let book = ctx.accounts.orderbook.load_mut()?;

        oracle_price =
            perp_market.oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?)?;

        perp_market.update_funding(&book, oracle_price, now_ts)?;
    }

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

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    // TODO apply reduce_only flag to compute final base_lots, also process event queue
    require!(order.reduce_only == false, MangoError::SomeError);

    book.new_order(
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
    if let Some((mut health_cache, pre_health)) = pre_health_opt {
        let perp_position = account.perp_position(perp_market_index)?;
        health_cache.recompute_perp_info(perp_position, &perp_market)?;
        account.check_health_post(&health_cache, pre_health)?;
    }

    Ok(())
}
