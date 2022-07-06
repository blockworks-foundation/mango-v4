use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct CloseAccount<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        // note: should never be the delegate
        has_one = owner,
        has_one = group,
        close = sol_destination
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn account_close(ctx: Context<CloseAccount>) -> Result<()> {
    let group = ctx.accounts.group.load()?;

    // don't perform checks if group is just testing
    if group.testing == 1 {
        return Ok(());
    }

    let account = ctx.accounts.account.load()?;
    require!(!account.being_liquidated(), MangoError::SomeError);
    require!(!account.is_bankrupt(), MangoError::SomeError);
    require_eq!(account.delegate, Pubkey::default());
    for ele in account.tokens.values {
        require_eq!(ele.is_active(), false);
    }
    for ele in account.serum3.values {
        require_eq!(ele.is_active(), false);
    }
    for ele in account.perps.accounts {
        require_eq!(ele.is_active(), false);
    }

    Ok(())
}
