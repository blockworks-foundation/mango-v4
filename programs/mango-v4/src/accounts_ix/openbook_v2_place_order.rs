use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use openbook_v2::{
    program::OpenbookV2,
    state::{Market, OpenOrdersAccount},
};

#[derive(Accounts)]
pub struct OpenbookV2PlaceOrder<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2PlaceOrder) @ MangoError::IxIsDisabled,
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

    #[account(mut)]
    pub open_orders: AccountLoader<'info, OpenOrdersAccount>,

    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    pub openbook_v2_program: Program<'info, OpenbookV2>,

    #[account(
        mut,
        has_one = bids,
        has_one = asks,
        has_one = event_heap,
    )]
    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    #[account(mut)]
    /// CHECK: bids will be checked by openbook_v2
    pub bids: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: asks will be checked by openbook_v2
    pub asks: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: event queue will be checked by openbook_v2
    pub event_heap: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: vault will be checked by openbook_v2
    pub market_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_vault_signer: UncheckedAccount<'info>,

    /// The bank that pays for the order. Bank oracle also expected in remaining_accounts
    // token_index and payer_bank.vault == payer_vault is validated inline at #3
    #[account(mut, has_one = group)]
    pub payer_bank: AccountLoader<'info, Bank>,
    /// The bank vault that pays for the order
    #[account(mut)]
    pub payer_vault: Box<Account<'info, TokenAccount>>,

    /// The bank that receives the funds upon settlement. Bank oracle also expected in remaining_accounts
    // token_index is validated inline at #3
    #[account(mut, has_one = group)]
    pub receiver_bank: AccountLoader<'info, Bank>,

    pub token_program: Program<'info, Token>,
}
