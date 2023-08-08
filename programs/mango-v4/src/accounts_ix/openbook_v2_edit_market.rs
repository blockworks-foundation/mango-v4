use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
#[instruction(market_index: OpenbookV2MarketIndex)]
pub struct OpenbookV2EditMarket<'info> {
    #[account(
        constraint = group.load()?.openbook_v2_supported(),
        constraint = group.load()?.admin == admin.key(),
    )]
    pub group: AccountLoader<'info, Group>,

    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub market: AccountLoader<'info, OpenbookV2Market>,
}
