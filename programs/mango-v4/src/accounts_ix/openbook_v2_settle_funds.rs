use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::error::*;
use crate::state::*;
use openbook_v2::{program::OpenbookV2, state::Market};

#[derive(Accounts)]
pub struct OpenbookV2SettleFunds<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2SettleFunds) @ MangoError::IxIsDisabled,
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
    /// CHECK: Validated inline by checking against the pubkey stored in the account at #2
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = openbook_v2_program,
        has_one = openbook_v2_market_external,
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    pub openbook_v2_program: Program<'info, OpenbookV2>,

    #[account(mut)]
    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    #[account(
        mut,
        constraint = market_base_vault.mint == base_vault.mint,
    )]
    pub market_base_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = market_quote_vault.mint == quote_vault.mint,
    )]
    pub market_quote_vault: Box<Account<'info, TokenAccount>>,

    /// needed for the automatic settle_funds call
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

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked in the parent
    pub quote_oracle: UncheckedAccount<'info>,
    /// CHECK: The oracle can be one of several different account types and the pubkey is checked in the parent
    pub base_oracle: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
