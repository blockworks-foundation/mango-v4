use crate::accounts_ix::*;
use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::logs::emit_perp_balances;
use crate::require_msg;
use crate::state::{MangoAccountLoader, OracleAccountInfos};
use anchor_lang::prelude::*;
use fixed::types::I80F48;

pub fn perp_force_close_unmatched(ctx: Context<PerpForceCloseUnmatched>) -> Result<()> {
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    require_msg!(
        perp_market.open_interest > 0,
        "Perp market must have open interest"
    );

    let mut account = ctx.accounts.account.load_full_mut()?;
    let perp_market_index = perp_market.perp_market_index;
    let position = account.perp_position_mut(perp_market_index)?;
    require_msg!(
        position.base_position_lots() > 0,
        "User must have a long position"
    );

    let clock = Clock::get()?;
    let (now_ts, now_slot) = (clock.unix_timestamp as u64, clock.slot);
    let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
    let oracle_price = perp_market.oracle_price(
        &OracleAccountInfos::from_reader(oracle_ref),
        Some((now_ts, now_slot)),
    )?;

    let base_transfer = 1; // we know base_position_lots is 1
    let quote_transfer = I80F48::from(base_transfer * perp_market.base_lot_size) * oracle_price;
    position.record_trade(&mut perp_market, -base_transfer, quote_transfer);

    // Normally here we would have an opposing trade. But in this BTC-PERP market there is no short to close against
    // Also note, no funding is applied before record_trade just to keep it consistent with PerpForceClosePosition

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.account.key(),
        position,
        &perp_market,
    );

    Ok(())
}
