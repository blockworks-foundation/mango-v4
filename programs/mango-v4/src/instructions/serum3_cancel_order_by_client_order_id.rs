use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

use crate::accounts_ix::*;
use crate::logs::{emit_stack, Serum3OpenOrdersBalanceLogV2};
use crate::serum3_cpi::{load_open_orders_ref, OpenOrdersAmounts, OpenOrdersSlim};

use super::update_bank_potential_tokens;
use super::update_order_tracking;

pub fn serum3_cancel_order_by_client_order_id(
    ctx: Context<Serum3CancelOrderV2>,
    client_order_id: u64,
) -> Result<()> {
    let accounts = &ctx.accounts.v1;
    let v2 = &ctx.accounts.v2;

    let ix_gate = IxGate::Serum3CancelOrderByClientOrderId;
    let group = accounts.group.load()?;
    require!(group.is_ix_enabled(ix_gate), MangoError::IxIsDisabled);

    let serum_market = accounts.serum_market.load()?;

    //
    // Validation
    //
    {
        let account = accounts.account.load_full()?;
        // account constraint #1
        require!(
            account.fixed.is_owner_or_delegate(accounts.owner.key()),
            MangoError::SomeError
        );

        // Validate open_orders #2
        require!(
            account
                .serum3_orders(serum_market.market_index)?
                .open_orders
                == accounts.open_orders.key(),
            MangoError::SomeError
        );
    }

    //
    // Cancel
    //
    cpi_cancel_order_by_client_order_id(accounts, client_order_id)?;

    let oo_ai = &accounts.open_orders.as_ref();
    let open_orders = load_open_orders_ref(oo_ai)?;
    let after_oo = OpenOrdersSlim::from_oo(&open_orders);

    let mut account = accounts.account.load_full_mut()?;
    let serum_orders = account.serum3_orders_mut(serum_market.market_index)?;

    update_order_tracking(serum_orders, &after_oo);

    let mut base_bank = v2.base_bank.load_mut()?;
    let mut quote_bank = v2.quote_bank.load_mut()?;
    update_bank_potential_tokens(serum_orders, &mut base_bank, &mut quote_bank, &after_oo);

    emit_stack(Serum3OpenOrdersBalanceLogV2 {
        mango_group: accounts.group.key(),
        mango_account: accounts.account.key(),
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

fn cpi_cancel_order_by_client_order_id(
    ctx: &Serum3CancelOrder,
    client_order_id: u64,
) -> Result<()> {
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
    .cancel_one_by_client_order_id(&group, client_order_id)
}
