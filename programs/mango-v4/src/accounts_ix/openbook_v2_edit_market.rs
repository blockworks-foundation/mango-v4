use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
#[instruction(market_index: OpenbookV2MarketIndex)]
pub struct OpenbookV2EditMarket<'info> {
    #[account(
        // group <-> admin relation is checked at #1
        constraint = group.load()?.openbook_v2_supported()
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub market: AccountLoader<'info, OpenbookV2Market>,
}
