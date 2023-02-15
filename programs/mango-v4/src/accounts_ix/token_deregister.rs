use crate::{error::MangoError, state::*};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

/// In addition to these accounts, there must be remaining_accounts:
/// all n pairs of bank and its corresponding vault account for a token
#[derive(Accounts)]
pub struct TokenDeregister<'info> {
    #[account(
        has_one = admin,
        constraint = group.load()?.is_ix_enabled(IxGate::TokenDeregister) @ MangoError::IxIsDisabled,
        constraint = group.load()?.is_testing(),
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    // match mint info to bank
    #[account(
        mut,
        has_one = group,
        close = sol_destination
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    #[account(mut)]
    pub dust_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
