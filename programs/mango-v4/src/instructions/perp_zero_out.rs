use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct PerpZeroOutForMarket<'info> {
    #[account(
        has_one = admin,
        constraint = group.load()?.is_operational() @ MangoError::GroupIsHalted,
        constraint = group.load()?.is_testing()
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        has_one = group,
        constraint = perp_market.load()?.perp_market_index == 1
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    pub admin: Signer<'info>,
}

pub fn perp_zero_out_for_market(ctx: Context<PerpZeroOutForMarket>) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;

    let perp_market = ctx.accounts.perp_market.load()?;

    let perp_position = account.perp_position_mut(perp_market.perp_market_index)?;
    *perp_position = PerpPosition::default();

    for i in 0..account.header.perp_oo_count() {
        let oo = account.perp_order_mut_by_raw_index(i);
        if !oo.is_active_for_market(perp_market.perp_market_index) {
            continue;
        }
        *oo = PerpOpenOrder::default();
    }

    Ok(())
}
