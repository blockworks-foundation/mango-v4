use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount};

use crate::state::*;

#[derive(Accounts)]
pub struct TokenDeregister<'info> {
    #[account(
        constraint = group.load()?.testing == 1,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    // match bank to group
    // match bank to vault
    #[account(
        mut,
        has_one = group,
        has_one = vault,
        close = sol_destination
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    // match mint info to bank
    #[account(
        mut,
        has_one = bank,
        close = sol_destination
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[allow(clippy::too_many_arguments)]
pub fn token_deregister(ctx: Context<TokenDeregister>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let group_seeds = group_seeds!(group);
    let cpi_accounts = CloseAccount {
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.sol_destination.to_account_info(),
        authority: ctx.accounts.group.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    token::close_account(CpiContext::new_with_signer(
        cpi_program,
        cpi_accounts,
        &[group_seeds],
    ))?;
    ctx.accounts.vault.exit(ctx.program_id)?;

    Ok(())
}
