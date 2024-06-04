use anchor_lang::prelude::*;

use openbook_v2::cpi::accounts::CancelOrder;

use crate::error::*;
use crate::logs::{emit_stack, OpenbookV2OpenOrdersBalanceLog};
use crate::serum3_cpi::OpenOrdersAmounts;
use crate::serum3_cpi::OpenOrdersSlim;
use crate::state::*;

use crate::accounts_ix::*;

use openbook_v2::state::Side as OpenbookV2Side;

// Very similar to openbook_v2_cancel_order except this uses a u64 and calls the
// cpi for cancelling by client order id. Uses the same accounts.
pub fn openbook_v2_cancel_order_by_client_order_id(
    ctx: Context<OpenbookV2CancelOrder>,
    side: OpenbookV2Side,
    client_order_id: u64,
) -> Result<()> {
    // Check instruction gate. Guarded by the same as cancelling by exchange id.
    let group = ctx.accounts.group.load()?;
    require!(
        group.is_ix_enabled(IxGate::OpenbookV2CancelOrder),
        MangoError::IxIsDisabled
    );

    let openbook_market = ctx.accounts.openbook_v2_market.load()?;

    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load_full()?;
        // account constraint #1
        require!(
            account
                .fixed
                .is_owner_or_delegate(ctx.accounts.authority.key()),
            MangoError::SomeError
        );

        // Validate open_orders #2
        require!(
            account
                .openbook_v2_orders(openbook_market.market_index)?
                .open_orders
                == ctx.accounts.open_orders.key(),
            MangoError::SomeError
        );
    }

    //
    // Cancel cpi
    //
    let account = ctx.accounts.account.load()?;
    let account_seeds = mango_account_seeds!(account);
    cpi_cancel_order_by_client_order_id(ctx.accounts, &[account_seeds], client_order_id)?;

    let open_orders = ctx.accounts.open_orders.load()?;
    let openbook_market_external = ctx.accounts.openbook_v2_market_external.load()?;
    let after_oo = OpenOrdersSlim::from_oo_v2(
        &open_orders,
        openbook_market_external.base_lot_size.try_into().unwrap(),
        openbook_market_external.quote_lot_size.try_into().unwrap(),
    );

    emit_stack(OpenbookV2OpenOrdersBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        market_index: openbook_market.market_index,
        base_token_index: openbook_market.base_token_index,
        quote_token_index: openbook_market.quote_token_index,
        base_total: after_oo.native_base_total(),
        base_free: after_oo.native_base_free(),
        quote_total: after_oo.native_quote_total(),
        quote_free: after_oo.native_quote_free(),
        referrer_rebates_accrued: after_oo.native_rebates(),
    });

    Ok(())
}

fn cpi_cancel_order_by_client_order_id(ctx: &OpenbookV2CancelOrder, seeds: &[&[&[u8]]], client_order_id: u64) -> Result<()> {
    let cpi_accounts = CancelOrder {
        signer: ctx.account.to_account_info(),
        open_orders_account: ctx.open_orders.to_account_info(),
        market: ctx.openbook_v2_market_external.to_account_info(),
        bids: ctx.bids.to_account_info(),
        asks: ctx.asks.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.openbook_v2_program.to_account_info(),
        cpi_accounts,
        seeds,
    );

    let _total_quantity_cancelled = openbook_v2::cpi::cancel_order_by_client_order_id(
        cpi_ctx,
        client_order_id
    )?;

    Ok(())
}
