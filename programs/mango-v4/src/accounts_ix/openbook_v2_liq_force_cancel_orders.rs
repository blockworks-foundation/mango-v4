use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::error::*;
use crate::state::*;
use openbook_v2::{
    program::OpenbookV2,
    state::{BookSide, Market, OpenOrdersAccount},
};

#[derive(Accounts)]
pub struct OpenbookV2LiqForceCancelOrders<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2LiqForceCancelOrders) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    // Allow force cancel even if account is frozen
    #[account(
        mut,
        has_one = group
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    #[account(mut)]
    pub open_orders: AccountLoader<'info, OpenOrdersAccount>,

    #[account(
        has_one = group,
        has_one = openbook_v2_program,
        has_one = openbook_v2_market_external,
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    pub openbook_v2_program: Program<'info, OpenbookV2>,

    #[account(mut)]
    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    #[account(mut)]
    pub market_bids: AccountLoader<'info, BookSide>,

    #[account(mut)]
    pub market_asks: AccountLoader<'info, BookSide>,

    #[account(mut)]
    pub market_event_queue: AccountLoader<'info, EventQueue>,

    #[account(mut)]
    pub market_base_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub market_quote_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_vault_signer: UncheckedAccount<'info>,

    // token_index and bank.vault == vault is validated inline at #3
    #[account(mut, has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub quote_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, has_one = group)]
    pub base_bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub base_vault: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}
