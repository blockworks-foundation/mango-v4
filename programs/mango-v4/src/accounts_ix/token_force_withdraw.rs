use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct TokenForceWithdraw<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::TokenForceWithdraw) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen,
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        mut,
        has_one = group,
        has_one = vault,
        has_one = oracle,
        // the mints of bank/vault/token_accounts are implicitly the same because
        // spl::token::transfer succeeds between token_account and vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    #[account(
        mut,
        address = get_associated_token_address(&account.load()?.owner, &vault.mint),
        // NOTE: the owner may have been changed (before immutable owner was a thing)
    )]
    pub owner_ata_token_account: Box<Account<'info, TokenAccount>>,

    /// Only for the unusual case where the owner_ata account is not owned by account.owner
    #[account(
        mut,
        constraint = alternate_owner_token_account.owner == account.load()?.owner,
    )]
    pub alternate_owner_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}
