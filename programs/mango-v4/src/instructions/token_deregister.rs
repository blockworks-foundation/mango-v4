use crate::{accounts_zerocopy::LoadZeroCopyRef, state::*};
use anchor_lang::prelude::*;
use anchor_lang::AccountsClose;
use anchor_spl::token::{self, CloseAccount, Token};

#[derive(Accounts)]
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
        close = sol_destination
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    #[account(mut)]
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[allow(clippy::too_many_arguments)]
pub fn token_deregister<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, TokenDeregister<'info>>,
) -> Result<()> {
    let mint_info = ctx.accounts.mint_info.load()?;
    
    for i in 

    Ok(())
}
`