use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(token_index: TokenIndex, bank_num: u32)]
pub struct TokenAddBank<'info> {
    #[account(
        has_one = admin,
        constraint = group.load()?.is_operational(),
        constraint = group.load()?.multiple_banks_supported()
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        constraint = existing_bank.load()?.token_index == token_index,
        has_one = group,
        has_one = mint,
    )]
    pub existing_bank: AccountLoader<'info, Bank>,

    #[account(
        init,
        // using the token_index in this seed guards against reusing it
        seeds = [b"Bank".as_ref(), group.key().as_ref(), &token_index.to_le_bytes(), &bank_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Bank>(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        init,
        seeds = [b"Vault".as_ref(), group.key().as_ref(), &token_index.to_le_bytes(), &bank_num.to_le_bytes()],
        bump,
        token::authority = group,
        token::mint = mint,
        payer = payer
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = mint_info.load()?.token_index == token_index,
        has_one = group,
        has_one = mint,
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
pub fn token_add_bank(
    ctx: Context<TokenAddBank>,
    token_index: TokenIndex,
    bank_num: u32,
) -> Result<()> {
    let existing_bank = ctx.accounts.existing_bank.load()?;
    let mut bank = ctx.accounts.bank.load_init()?;
    let bump = *ctx.bumps.get("bank").ok_or(MangoError::SomeError)?;
    *bank = Bank::from_existing_bank(&existing_bank, ctx.accounts.vault.key(), bank_num, bump);

    let mut mint_info = ctx.accounts.mint_info.load_mut()?;
    let free_slot = mint_info
        .banks
        .iter()
        .position(|bank| bank == &Pubkey::default())
        .unwrap();
    require_eq!(bank_num as usize, free_slot);
    mint_info.banks[free_slot] = ctx.accounts.bank.key();
    mint_info.vaults[free_slot] = ctx.accounts.vault.key();

    Ok(())
}
