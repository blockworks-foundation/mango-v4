use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use openbook_v2::{
    program::OpenbookV2,
    state::{Market},
};

#[derive(Accounts)]
pub struct OpenbookV2PlaceTakerOrder<'info> {
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

    #[account(
        has_one = group,
        has_one = openbook_v2_program,
        has_one = openbook_v2_market_external,
    )]
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

    // These accounts are forwarded directly to the openbook_v2 cpi call
    // and are validated there.
    #[account(mut)]
    pub bids: AccountLoader<'info, ObV2BookSize>,

    #[account(mut)]
    /// CHECK: Validated by the openbook_v2 cpi call
    pub asks: AccountLoader<'info, ObV2BookSize>,

    #[account(mut)]
    /// CHECK: Validated by the openbook_v2 cpi call
    pub event_queue: AccountLoader<'info, ObV2EventQueue>,

    #[account(mut)]
    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_request_queue: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = market_base_vault.mint == payer_vault.mint,
    )]
    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_base_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    /// CHECK: Validated by the openbook_v2 cpi call
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
