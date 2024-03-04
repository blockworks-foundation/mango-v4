use anchor_lang::prelude::*;

use crate::accounts_ix::*;

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::MangoError;
use crate::logs::{emit_perp_balances, emit_stack, PerpForceClosePositionLog};
use crate::state::*;
use fixed::types::I80F48;

pub fn perp_force_close_position(ctx: Context<PerpForceClosePosition>) -> Result<()> {
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let perp_market_index = perp_market.perp_market_index;

    let mut account_a = ctx.accounts.account_a.load_full_mut()?;
    let mut account_b = ctx.accounts.account_b.load_full_mut()?;

    let account_a_perp_position = account_a.perp_position_mut(perp_market_index)?;
    let account_b_perp_position = account_b.perp_position_mut(perp_market_index)?;

    require_gt!(
        account_a_perp_position.base_position_lots(),
        0,
        MangoError::SomeError
    );
    require_gt!(
        0,
        account_b_perp_position.base_position_lots(),
        MangoError::SomeError
    );

    let base_transfer = account_a_perp_position
        .base_position_lots()
        .min(account_b_perp_position.base_position_lots().abs())
        .max(0);
    let now_slot = Clock::get()?.slot;
    let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
    let oracle_price =
        perp_market.oracle_price(&OracleAccountInfos::from_reader(oracle_ref), Some(now_slot))?;
    let quote_transfer = I80F48::from(base_transfer * perp_market.base_lot_size) * oracle_price;

    account_a_perp_position.record_trade(&mut perp_market, -base_transfer, quote_transfer);
    account_b_perp_position.record_trade(&mut perp_market, base_transfer, -quote_transfer);

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.account_a.key(),
        account_a_perp_position,
        &perp_market,
    );
    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.account_b.key(),
        &account_b_perp_position,
        &perp_market,
    );

    emit_stack(PerpForceClosePositionLog {
        mango_group: ctx.accounts.group.key(),
        perp_market_index: perp_market.perp_market_index,
        account_a: ctx.accounts.account_a.key(),
        account_b: ctx.accounts.account_b.key(),
        base_transfer: base_transfer,
        quote_transfer: quote_transfer.to_bits(),
        price: oracle_price.to_bits(),
    });

    Ok(())
}
