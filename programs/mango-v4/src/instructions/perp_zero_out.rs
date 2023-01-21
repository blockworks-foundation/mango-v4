use anchor_lang::prelude::*;
use bytemuck::Zeroable;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct PerpZeroOutForMarket<'info> {
    #[account(
        constraint = group.load()?.is_operational() @ MangoError::GroupIsHalted
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
}

pub fn perp_zero_out_for_market(ctx: Context<PerpZeroOutForMarket>) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;

    let perp_market = ctx.accounts.perp_market.load()?;
    let mut perp_position = account.perp_position_mut(perp_market.perp_market_index)?;
    *perp_position = PerpPosition::zeroed();
    perp_position.market_index = PerpMarketIndex::MAX;

    for i in 0..account.header.perp_oo_count() {
        let mut oo = account.perp_order_by_raw_index(i);
        if !oo.is_active_for_market(perp_market.perp_market_index) {
            continue;
        }
        *oo.market = FREE_ORDER_SLOT;
        *oo.side_and_tree = SideAndOrderTree::BidFixed.into();
        *oo.id = 0;
        *oo.client_id = 0;
    }

    Ok(())
}
