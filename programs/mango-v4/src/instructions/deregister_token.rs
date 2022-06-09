use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount};

use crate::state::*;

#[derive(Accounts)]
pub struct DeregisterToken<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    // match bank to group
    #[account(
        mut,
        constraint = bank.load()?.group == group.key(),
        close = sol_destination
    )]
    pub bank: AccountLoader<'info, Bank>,

    // match vault to bank
    #[account(
        mut,
        constraint =  vault.key() == bank.load()?.vault,
        token::mint = bank.load()?.mint,
    )]
    pub vault: Account<'info, TokenAccount>,

    // match mint info to bank
    #[account(
        mut,
        constraint = mint_info.load()?.bank == bank.key(),
        close = sol_destination
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    #[account(mut)]
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[allow(clippy::too_many_arguments)]
pub fn deregister_token(ctx: Context<DeregisterToken>) -> Result<()> {
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
