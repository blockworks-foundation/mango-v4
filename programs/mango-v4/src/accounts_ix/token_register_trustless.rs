use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

const FIRST_BANK_NUM: u32 = 0;

#[derive(Accounts)]
#[instruction(token_index: TokenIndex)]
pub struct TokenRegisterTrustless<'info> {
    #[account(
        mut,
        constraint = group.load()?.admin == admin.key() || group.load()?.fast_listing_admin == admin.key(),
        constraint = group.load()?.is_ix_enabled(IxGate::TokenRegisterTrustless) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        // using the token_index in this seed guards against reusing it
        seeds = [b"Bank".as_ref(), group.key().as_ref(), &token_index.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Bank>(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        init,
        seeds = [b"Vault".as_ref(), group.key().as_ref(), &token_index.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
        bump,
        token::authority = group,
        token::mint = mint,
        payer = payer
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        init,
        // using the mint in this seed guards against registering the same mint twice
        seeds = [b"MintInfo".as_ref(), group.key().as_ref(), mint.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<MintInfo>(),
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    /// CHECK: The oracle can be one of several different account types
    pub fallback_oracle: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
