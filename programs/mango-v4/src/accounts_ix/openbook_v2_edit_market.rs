use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
#[instruction(market_index: OpenbookV2MarketIndex)]
pub struct OpenbookV2EditMarket<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,

    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub market: AccountLoader<'info, OpenbookV2Market>,
}
