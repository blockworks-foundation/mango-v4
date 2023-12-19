use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::{new_fixed_order_account_retriever, new_health_cache};
use crate::state::*;

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

        let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
        let oracle_state = perp_market.oracle_state(
            &OracleAccountInfos::from_reader(oracle_ref),
            None, // staleness checked in health
        )?;
        oracle_price = oracle_state.price;

        perp_market.update_funding_and_stable_price(&book, &oracle_state, now_ts)?;
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
        let health_cache = new_health_cache(&account.borrow(), &retriever, now_ts)
            .context("pre-withdraw init health")?;
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
    let group = ctx.accounts.group.load()?;

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    account
        .fixed
        .expire_buyback_fees(now_ts, group.buyback_fees_expiry_interval);

    let pp = account.perp_position(perp_market_index)?;
    let effective_pos = pp.effective_base_position_lots();
    let max_base_lots = if order.reduce_only || perp_market.is_reduce_only() {
        reduce_only_max_base_lots(pp, &order, perp_market.is_reduce_only())
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

fn reduce_only_max_base_lots(pp: &PerpPosition, order: &Order, market_reduce_only: bool) -> i64 {
    let effective_pos = pp.effective_base_position_lots();
    msg!(
        "reduce only: current effective position: {} lots",
        effective_pos
    );
    let allowed_base_lots = if (order.side == Side::Bid && effective_pos >= 0)
        || (order.side == Side::Ask && effective_pos <= 0)
    {
        msg!("reduce only: cannot increase magnitude of effective position");
        0
    } else if market_reduce_only {
        // If the market is in reduce-only mode, we are stricter and pretend
        // all open orders that go into the same direction as the new order
        // execute.
        if order.side == Side::Bid {
            msg!(
                "reduce only: effective base position incl open bids is {} lots",
                effective_pos + pp.bids_base_lots
            );
            (effective_pos + pp.bids_base_lots).min(0).abs()
        } else {
            msg!(
                "reduce only: effective base position incl open asks is {} lots",
                effective_pos - pp.asks_base_lots
            );
            (effective_pos - pp.asks_base_lots).max(0)
        }
    } else {
        effective_pos.abs()
    };
    msg!(
        "reduce only: max allowed {:?}: {} base lots",
        order.side,
        allowed_base_lots
    );
    allowed_base_lots.min(order.max_base_lots)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perp_reduce_only() {
        let test_cases = vec![
            ("null", true, 0, (0, 0), (Side::Bid, 0), 0),
            ("ok bid", true, -5, (0, 0), (Side::Bid, 1), 1),
            ("limited bid", true, -5, (0, 0), (Side::Bid, 10), 5),
            ("limited bid2", true, -5, (1, 10), (Side::Bid, 10), 4),
            ("limited bid3", false, -5, (1, 10), (Side::Bid, 10), 5),
            ("no bid", true, 5, (0, 0), (Side::Bid, 1), 0),
            ("ok ask", true, 5, (0, 0), (Side::Ask, 1), 1),
            ("limited ask", true, 5, (0, 0), (Side::Ask, 10), 5),
            ("limited ask2", true, 5, (10, 1), (Side::Ask, 10), 4),
            ("limited ask3", false, 5, (10, 1), (Side::Ask, 10), 5),
            ("no ask", true, -5, (0, 0), (Side::Ask, 1), 0),
        ];

        for (
            name,
            market_reduce_only,
            base_lots,
            (open_bids, open_asks),
            (side, amount),
            expected,
        ) in test_cases
        {
            println!("test: {name}");

            let pp = PerpPosition {
                base_position_lots: base_lots,
                bids_base_lots: open_bids,
                asks_base_lots: open_asks,
                ..PerpPosition::default()
            };
            let order = Order {
                side,
                max_base_lots: amount,
                max_quote_lots: 0,
                client_order_id: 0,
                reduce_only: true,
                time_in_force: 0,
                self_trade_behavior: SelfTradeBehavior::DecrementTake,
                params: OrderParams::Market {},
            };

            let result = reduce_only_max_base_lots(&pp, &order, market_reduce_only);
            assert_eq!(result, expected);
        }
    }
}
