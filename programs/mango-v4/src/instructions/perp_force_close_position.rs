use anchor_lang::prelude::*;

use crate::accounts_ix::*;

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::MangoError;
use crate::state::*;
use fixed::types::I80F48;

pub fn perp_force_close_position(ctx: Context<PerpForceClosePosition>) -> Result<()> {
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let perp_market_index = perp_market.perp_market_index;

    let mut account_a = ctx.accounts.account_a.load_full_mut()?;
    let mut account_b = ctx.accounts.account_b.load_full_mut()?;

    let account_a_perp_position = account_a.perp_position_mut(perp_market_index)?;
    let account_b_perp_position = account_b.perp_position_mut(perp_market_index)?;

    require!(
        account_a_perp_position.base_position_lots().signum()
            != account_b_perp_position.base_position_lots().signum(),
        MangoError::SomeError
    );

    let base_transfer = account_a_perp_position
        .base_position_lots()
        .abs()
        .min(account_b_perp_position.base_position_lots().abs())
        .max(0);

    let staleness_slot = perp_market
        .oracle_config
        .max_staleness_slots
        .try_into()
        .unwrap();
    let oracle_price = perp_market
        .oracle_price(
            &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
            Some(staleness_slot),
        )
        .unwrap();
    let quote_transfer = I80F48::from(base_transfer * perp_market.base_lot_size) * oracle_price;

    account_a_perp_position.record_trade(&mut perp_market, base_transfer, quote_transfer);
    account_b_perp_position.record_trade(&mut perp_market, -base_transfer, -quote_transfer);

    Ok(())
}
