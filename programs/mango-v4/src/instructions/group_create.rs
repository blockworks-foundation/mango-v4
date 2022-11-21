use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(group_num: u32)]
pub struct GroupCreate<'info> {
    #[account(
        init,
        seeds = [b"Group".as_ref(), creator.key().as_ref(), &group_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Group>(),
    )]
    pub group: AccountLoader<'info, Group>,

    pub creator: Signer<'info>,

    pub insurance_mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [b"InsuranceVault".as_ref(), group.key().as_ref()],
        bump,
        token::authority = group,
        token::mint = insurance_mint,
        payer = payer
    )]
    pub insurance_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn group_create(
    ctx: Context<GroupCreate>,
    group_num: u32,
    testing: u8,
    version: u8,
) -> Result<()> {
    let mut group = ctx.accounts.group.load_init()?;
    group.creator = ctx.accounts.creator.key();
    group.group_num = group_num;
    group.admin = ctx.accounts.creator.key();
    group.fast_listing_admin = Pubkey::default();
    group.insurance_vault = ctx.accounts.insurance_vault.key();
    group.insurance_mint = ctx.accounts.insurance_mint.key();
    group.bump = *ctx.bumps.get("group").ok_or(MangoError::SomeError)?;
    group.testing = testing;
    group.version = version;
    Ok(())
}
