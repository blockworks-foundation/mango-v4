use crate::accounts_ix::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount};

pub fn group_close(ctx: Context<GroupClose>) -> Result<()> {
    // close insurance vault (must be empty)
    let group = ctx.accounts.group.load()?;
    let group_seeds = group_seeds!(group);
    let cpi_accounts = CloseAccount {
        account: ctx.accounts.insurance_vault.to_account_info(),
        destination: ctx.accounts.sol_destination.to_account_info(),
        authority: ctx.accounts.group.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    token::close_account(CpiContext::new_with_signer(
        cpi_program,
        cpi_accounts,
        &[group_seeds],
    ))?;
    ctx.accounts.insurance_vault.exit(ctx.program_id)?;

    Ok(())
}
