use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use openbook_v2::{program::OpenbookV2, state::Market};

#[derive(Accounts)]
pub struct OpenbookV2CreateOpenOrders<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2CreateOpenOrders) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // authority is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        has_one = group,
        has_one = openbook_v2_program,
        has_one = openbook_v2_market_external,
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    pub openbook_v2_program: Program<'info, OpenbookV2>,

    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    #[account(mut)]
    /// CHECK: Will be checked against seeds and will be initiated by openbook v2
    pub open_orders_indexer: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Will be checked against seeds and will be initiated by openbook v2
    pub open_orders_account: UncheckedAccount<'info>,

    pub authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
