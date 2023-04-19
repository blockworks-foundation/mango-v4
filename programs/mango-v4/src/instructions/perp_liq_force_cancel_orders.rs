use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::state::*;

pub fn perp_liq_force_cancel_orders(
    ctx: Context<PerpLiqForceCancelOrders>,
    limit: u8,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    //
    // Early return if if liquidation is not allowed or if market is not in force close
    //
    let mut health_cache = {
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
        let health_cache =
            new_health_cache(&account.borrow(), &retriever).context("create health cache")?;

        {
            let check_liquidatable_result = account.check_liquidatable(&health_cache)?;
            if account.fixed.is_operational()
                && (check_liquidatable_result != CheckLiquidatable::Liquidatable)
                && !perp_market.is_force_close()
            {
                return Ok(());
            }
        }

        health_cache
    };

    //
    // Cancel orders
    //
    {
        let mut book = Orderbook {
            bids: ctx.accounts.bids.load_mut()?,
            asks: ctx.accounts.asks.load_mut()?,
        };

        book.cancel_all_orders(&mut account.borrow_mut(), &mut perp_market, limit, None)?;

        let perp_position = account.perp_position(perp_market.perp_market_index)?;
        health_cache.recompute_perp_info(perp_position, &perp_market)?;
    }

    //
    // Health check at the end
    //
    let init_health = health_cache.health(HealthType::LiquidationEnd);
    account
        .fixed
        .maybe_recover_from_being_liquidated(init_health);

    Ok(())
}
