use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::Mint;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct RegisterToken<'info> {
    #[account(
        has_one = owner,
    )]
    pub group: AccountLoader<'info, MangoGroup>,
    pub owner: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"tokenbank".as_ref(), mint.key().as_ref()],
        bump,
        payer = payer,
    )]
    pub bank: AccountLoader<'info, TokenBank>,

    #[account(
        init,
        associated_token::authority = bank,
        associated_token::mint = mint,
        payer = payer
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// TODO: should this be "configure_mint", we pass an explicit index, and allow
// overwriting config as long as the mint account stays the same?
pub fn register_token(ctx: Context<RegisterToken>, decimals: u8) -> Result<()> {
    ctx.accounts.bank.load_init()?;

    let mut group = ctx.accounts.group.load_mut()?;
    // TODO: Error if mint is already configured (techincally, init of vault will fail)
    // TOOD: Error type
    let token_index = group
        .tokens
        .iter()
        .position(|ti| !ti.is_valid())
        .ok_or(MangoError::SomeError)?;
    group.tokens[token_index] = TokenInfo {
        mint: ctx.accounts.mint.key(),
        decimals,
        reserved: [0u8; 31],
    };
    Ok(())
}
