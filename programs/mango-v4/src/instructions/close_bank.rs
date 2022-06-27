use crate::{accounts_zerocopy::LoadZeroCopyRef, state::*};
use anchor_lang::prelude::*;

use anchor_spl::token::{self, CloseAccount, Token, TokenAccount};

#[derive(Accounts)]
#[instruction(token_index: TokenIndex)]
pub struct CloseBank<'info> {
    #[account(
        constraint = group.load()?.testing == 1,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    // match mint info to bank
    #[account(
        mut,
        has_one = group,
        constraint = mint_info.load()?.token_index == token_index,
        close = sol_destination
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    #[account(
        mut,
        has_one = group,
        has_one = vault,
        close = sol_destination
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
pub fn close_bank<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, CloseBank<'info>>,
    token_index: TokenIndex,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let group_seeds = group_seeds!(group);

    let banks = ctx.accounts.mint_info.load()?.banks.to_vec();
    let vaults = ctx.accounts.mint_info.load()?.vaults.to_vec();

    // verify we are closing latest added bank first
    // verify bank and vault have same position on mint_info
    // verify vault belongs to bank
    let bank_pos = banks.len() - banks
            .iter()
            .rev()
            .position(|bank| *bank != Pubkey::default())
            .unwrap() - 1;
    let vault_pos = vaults.len() - vaults
            .iter()
            .rev()
            .position(|vault| *vault != Pubkey::default())
            .unwrap() - 1;

    require_keys_eq!(
        ctx.accounts.bank.key(),
        banks[bank_pos]
    );
    require_keys_eq!(
        ctx.accounts.vault.key(),
        vaults[vault_pos]
    );
    require_eq!(bank_pos, vault_pos);
    require_keys_eq!(ctx.accounts.bank.load()?.vault, ctx.accounts.vault.key());

    // close token account
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

    // update mint_info
    let mut mint_info = ctx.accounts.mint_info.load_mut()?;
    mint_info.banks[bank_pos] = Pubkey::default();
    mint_info.banks[vault_pos] = Pubkey::default();

    Ok(())
}
