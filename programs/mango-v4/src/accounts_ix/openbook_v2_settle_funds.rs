use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use openbook_v2::state::OpenOrdersAccount;

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

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub open_orders: AccountLoader<'info, OpenOrdersAccount>,

    #[account(
        has_one = group,
        has_one = openbook_v2_program,
        has_one = openbook_v2_market_external,
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    pub openbook_v2_program: Program<'info, OpenbookV2>,

    #[account(
        mut,
        has_one = market_base_vault,
        has_one = market_quote_vault,
    )]
    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    #[account(mut)]
    pub market_base_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
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

    /// CHECK: validated against banks at #4
    pub quote_oracle: UncheckedAccount<'info>,
    /// CHECK: validated against banks at #4
    pub base_oracle: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
