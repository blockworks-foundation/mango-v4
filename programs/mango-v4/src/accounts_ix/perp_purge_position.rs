use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
pub struct PerpPurgePosition<'info> {
    #[account()]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        // owner is not checked on purpose
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    #[account(has_one = group)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    #[account(mut, has_one = group)]
    pub settle_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,
}
