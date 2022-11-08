use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::state::*;

#[derive(Accounts)]
pub struct GroupCreateMsrmVault<'info> {
    #[account(
        mut,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    pub msrm_mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [b"MsrmVault".as_ref(), group.key().as_ref()],
        bump,
        token::authority = group,
        token::mint = msrm_mint,
        payer = payer
    )]
    pub msrm_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// Ix only exists to add vaults to groups created before msrm vault integration was done
pub fn group_create_msrm_vault(ctx: Context<GroupCreateMsrmVault>) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;
    group.msrm_vault = ctx.accounts.msrm_vault.key();
    group.msrm_mint = ctx.accounts.msrm_mint.key();
    Ok(())
}
