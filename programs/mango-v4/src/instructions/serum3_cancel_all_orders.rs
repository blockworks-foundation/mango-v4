use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::logs::{emit_stack, Serum3OpenOrdersBalanceLogV2};
use crate::serum3_cpi::{load_open_orders_ref, OpenOrdersAmounts, OpenOrdersSlim};
use crate::state::*;

pub fn serum3_cancel_all_orders(ctx: Context<Serum3CancelAllOrders>, limit: u8) -> Result<()> {
    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load_full()?;
        // account constraint #1
        require!(
            account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
            MangoError::SomeError
        );

        let serum_market = ctx.accounts.serum_market.load()?;

        // Validate open_orders #2
        require!(
            account
                .serum3_orders(serum_market.market_index)?
                .open_orders
                == ctx.accounts.open_orders.key(),
            MangoError::SomeError
        );
    }

    //
    // Cancel
    //
    cpi_cancel_all_orders(ctx.accounts, limit)?;

    let serum_market = ctx.accounts.serum_market.load()?;
    let oo_ai = &ctx.accounts.open_orders.as_ref();
    let open_orders = load_open_orders_ref(oo_ai)?;
    let after_oo = OpenOrdersSlim::from_oo(&open_orders);
    emit_stack(Serum3OpenOrdersBalanceLogV2 {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        market_index: serum_market.market_index,
        base_token_index: serum_market.base_token_index,
        quote_token_index: serum_market.quote_token_index,
        base_total: after_oo.native_base_total(),
        base_free: after_oo.native_base_free(),
        quote_total: after_oo.native_quote_total(),
        quote_free: after_oo.native_quote_free(),
        referrer_rebates_accrued: after_oo.native_rebates(),
    });

    Ok(())
}

fn cpi_cancel_all_orders(ctx: &Serum3CancelAllOrders, limit: u8) -> Result<()> {
    use crate::serum3_cpi;
    let group = ctx.group.load()?;
    serum3_cpi::CancelOrder {
        program: ctx.serum_program.to_account_info(),
        market: ctx.serum_market_external.to_account_info(),
        bids: ctx.market_bids.to_account_info(),
        asks: ctx.market_asks.to_account_info(),
        event_queue: ctx.market_event_queue.to_account_info(),

        open_orders: ctx.open_orders.to_account_info(),
        open_orders_authority: ctx.group.to_account_info(),
    }
    .cancel_all(&group, limit)
}
