use anchor_lang::prelude::*;
use fixed::types::I80F48;
use openbook_v2::cpi::Return;
use openbook_v2::state::{Order as OpenbookV2Order, Side as OpenbookV2Side, PlaceOrderType as OpenbookV2OrderType};
use crate::error::MangoError;
use crate::state::*;
use crate::util::fill_from_str;

use crate::accounts_ix::*;

pub fn openbook_v2_place_order(
    ctx: Context<OpenbookV2PlaceOrder>,
    order: OpenbookV2Order,
    limit: u8,
) -> Result<()> {
    Ok(())
}

fn cpi_place_order(
    ctx: &OpenbookV2PlaceOrder,
    seeds: &[&[&[u8]]],
    order: OpenbookV2Order,
    limit: u8,
) -> Result<Return<Option<u128>>> {
    let cpi_accounts = openbook_v2::cpi::accounts::PlaceOrder {
        signer: ctx.account.to_account_info(),
        open_orders_account: ctx.open_orders.to_account_info(),
        open_orders_admin: None,
        user_token_account: ctx.vault.to_account_info(),
        market: ctx.openbook_v2_market_external.to_account_info(),
        bids: ctx.bids.to_account_info(),
        asks: ctx.asks.to_account_info(),
        event_heap: ctx.event_heap.to_account_info(),
        market_vault: ctx.market_vault.to_account_info(),
        oracle_a: None, // todo-pan: how do oracle work
        oracle_b: None,
        token_program: ctx.token_program.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.openbook_v2_program.to_account_info(),
        cpi_accounts,
        seeds,
    );

    let price_lots = {
        let market = ctx.openbook_v2_market_external.load()?;
        market.native_price_to_lot(I80F48::from(1000)).unwrap()
    };

    let expiry_timestamp: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();

    let args = openbook_v2::PlaceOrderArgs {
        side: order.side,
        price_lots,
        max_base_lots: order.max_base_lots,
        max_quote_lots_including_fees: order.max_quote_lots_including_fees,
        client_order_id: order.client_order_id,
        order_type: OpenbookV2OrderType::Limit,
        expiry_timestamp,
        self_trade_behavior: order.self_trade_behavior,
        limit,
    };
    openbook_v2::cpi::place_order(cpi_ctx, args)
}