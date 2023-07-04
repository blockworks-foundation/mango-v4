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
        // owner is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account at #2
    pub open_orders: AccountLoader<'info, OpenOrdersAccount>,

    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    pub openbook_v2_program: Program<'info, OpenbookV2>,

    #[account(
        mut,
        has_one = bids,
        has_one = asks,
        has_one = event_queue,
        constraint = openbook_v2_market_external.load()?.base_vault == market_base_vault.key(),
        constraint = openbook_v2_market_external.load()?.quote_vault == market_quote_vault.key(),
    )]
    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    #[account(mut)]
    /// CHECK: bids will be checked by openbook_v2
    pub bids: AccountLoader<'info, ObV2BookSize>,

    #[account(mut)]
    /// CHECK: asks will be checked by openbook_v2
    pub asks: AccountLoader<'info, ObV2BookSize>,

    #[account(mut)]
    /// CHECK: event queue will be checked by openbook_v2
    pub event_queue: AccountLoader<'info, ObV2EventQueue>,

    #[account(mut)]
    /// CHECK: base vault will be checked by openbook_v2
    pub market_base_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    /// CHECK: quote vault will be checked by openbook_v2
    pub market_quote_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_vault_signer: UncheckedAccount<'info>,

    /// The bank that pays for the order, if necessary
    // token_index and payer_bank.vault == payer_vault is validated inline at #3
    #[account(mut, has_one = group)]
    pub payer_bank: AccountLoader<'info, Bank>,
    /// The bank vault that pays for the order, if necessary
    #[account(mut)]
    pub payer_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: The oracle can be one of several different account types
    #[account(address = payer_bank.load()?.oracle)]
    pub payer_oracle: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}