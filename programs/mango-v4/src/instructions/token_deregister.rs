use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token};

use crate::{accounts_zerocopy::LoadZeroCopyRef, state::*};
use anchor_lang::AccountsClose;

#[derive(Accounts)]
#[instruction(token_index: TokenIndex)]
pub struct TokenDeregister<'info> {
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

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[allow(clippy::too_many_arguments)]
pub fn token_deregister<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, TokenDeregister<'info>>,
    token_index: TokenIndex,
) -> Result<()> {
    let mint_info = ctx.accounts.mint_info.load()?;
    {
        let total_banks = mint_info
            .banks
            .iter()
            .filter(|bank| *bank != &Pubkey::default())
            .count();
        require_eq!(total_banks * 2, ctx.remaining_accounts.len());
    }

    let group = ctx.accounts.group.load()?;
    let group_seeds = group_seeds!(group);

    // todo: use itertools::chunks(2)
    for i in (0..ctx.remaining_accounts.len()).step_by(2) {
        let vault_ai = &ctx.remaining_accounts[i + 1];
        let bank_ai = &ctx.remaining_accounts[i];

        require_eq!(bank_ai.key(), mint_info.banks[i / 2]);
        require_eq!(vault_ai.key(), mint_info.vaults[i / 2]);

        // todo: these checks might be superfluous, after above 2 checks
        {
            let bank = bank_ai.load::<Bank>()?;
            require_keys_eq!(bank.group, ctx.accounts.group.key());
            require_eq!(bank.token_index, token_index);
            require_keys_eq!(bank.vault, vault_ai.key());
        }

        // note: vault seems to need closing before bank, weird solana oddity
        // todo: add test to see if we can even close more than one bank in same ix
        // close vault
        let cpi_accounts = CloseAccount {
            account: vault_ai.to_account_info(),
            destination: ctx.accounts.sol_destination.to_account_info(),
            authority: ctx.accounts.group.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        token::close_account(CpiContext::new_with_signer(
            cpi_program,
            cpi_accounts,
            &[group_seeds],
        ))?;
        vault_ai.exit(ctx.program_id)?;
    }

    // Close banks in a second step, because the cpi calls above don't like when someone
    // else touches sol_destination.lamports in between.
    for i in (0..ctx.remaining_accounts.len()).step_by(2) {
        let bank_ai = &ctx.remaining_accounts[i];
        let bank_al: AccountLoader<Bank> = AccountLoader::try_from(bank_ai)?;
        bank_al.close(ctx.accounts.sol_destination.to_account_info())?;
    }

    Ok(())
}
