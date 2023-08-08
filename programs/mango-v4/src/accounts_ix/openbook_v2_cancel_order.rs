use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;
use openbook_v2::{program::OpenbookV2, state::Market};

#[derive(Accounts)]
pub struct OpenbookV2CancelOrder<'info> {
    /// CHECK: Instruction gate will be checked in instruction code as this is used by multiple instructions.
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // authority is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    pub authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account at #2
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = openbook_v2_program,
        has_one = openbook_v2_market_external,
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    pub openbook_v2_program: Program<'info, OpenbookV2>,

    #[account(
        has_one = bids,
        has_one = asks,
    )]
    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    #[account(mut)]
    /// CHECK: bids will be checked by openbook_v2
    pub bids: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: asks will be checked by openbook_v2
    pub asks: UncheckedAccount<'info>,
}
