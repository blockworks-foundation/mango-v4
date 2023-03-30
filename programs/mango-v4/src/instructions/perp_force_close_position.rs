use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::accounts_zerocopy::*;

use crate::error::MangoError;
use crate::state::*;

pub fn perp_force_close_position(ctx: Context<PerpForceClosePosition>) -> Result<()> {
    let mut account_a = ctx.accounts.account_a.load_full_mut()?;
    let mut account_b = ctx.accounts.account_b.load_full_mut()?;

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let perp_market_index = perp_market.perp_market_index;

    let account_a_perp_position = account_a.perp_position_mut(perp_market_index)?;
    let account_b_perp_position = account_b.perp_position_mut(perp_market_index)?;

    require!(
        account_a_perp_position.base_position_lots().is_negative(),
        MangoError::SomeError
    );
    require!(
        account_b_perp_position.base_position_lots().is_positive(),
        MangoError::SomeError
    );

    let base_transfer = account_a_perp_position
        .base_position_lots()
        .abs()
        .min(account_b_perp_position.base_position_lots());
    let quote_transfer = base_transfer
        * perp_market.base_lot_size
        * perp_market.oracle_price(&ctx.accounts.oracle.as_ref(), perp_market.staleness_slot);

    account_a_perp_position.record_trade(&mut perp_market, base_transfer, quote_transfer);
    account_b_perp_position.record_trade(&mut perp_market, -base_transfer, -quote_transfer);

    Ok(())
}
