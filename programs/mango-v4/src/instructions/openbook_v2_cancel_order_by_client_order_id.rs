use anchor_lang::prelude::*;

use openbook_v2::cpi::accounts::CancelOrder;

use crate::error::*;
use crate::instructions::{emit_openbook_v2_balance_log, validate_openbook_v2_cancel_order};
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

    validate_openbook_v2_cancel_order(&ctx, &openbook_market)?;

    //
    // Cancel cpi
    //
    let account = ctx.accounts.account.load()?;
    let account_seeds = mango_account_seeds!(account);
    cpi_cancel_order_by_client_order_id(ctx.accounts, &[account_seeds], client_order_id)?;

    emit_openbook_v2_balance_log(&ctx, &openbook_market)?;
    Ok(())
}

fn cpi_cancel_order_by_client_order_id(
    ctx: &OpenbookV2CancelOrder,
    seeds: &[&[&[u8]]],
    client_order_id: u64,
) -> Result<()> {
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

    let _total_quantity_cancelled =
        openbook_v2::cpi::cancel_order_by_client_order_id(cpi_ctx, client_order_id)?;

    Ok(())
}
