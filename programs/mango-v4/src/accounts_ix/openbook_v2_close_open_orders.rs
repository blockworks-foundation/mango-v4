use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::error::MangoError;
use crate::state::*;
use openbook_v2::{
    program::OpenbookV2,
    state::{Market, OpenOrdersIndexer},
};

#[derive(Accounts)]
pub struct OpenbookV2CloseOpenOrders<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2CloseOpenOrders) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // authority is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    pub authority: Signer<'info>,

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
    /// can't zerocopy this unfortunately
    pub open_orders_indexer: Box<Account<'info, OpenOrdersIndexer>>,

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account at #2
    pub open_orders_account: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    // token_index is validated inline at #3
    #[account(mut, has_one = group)]
    pub base_bank: AccountLoader<'info, Bank>,

    // token_index is validated inline at #3
    #[account(mut, has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
