use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct RegisterToken<'info> {
    #[account(
        has_one = owner,
    )]
    pub group: AccountLoader<'info, MangoGroup>,
    pub owner: Signer<'info>,

    pub mint: Account<'info, Mint>,

    // TODO: Create the bank PDA
    // TODO: Create the vault PDA
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// TODO: should this be "configure_mint", we pass an explicit index, and allow
// overwriting config as long as the mint account stays the same?
pub fn register_token(ctx: Context<RegisterToken>, decimals: u8) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;
    // TOOD: Error type
    let token_index = group
        .tokens
        .iter()
        .position(|ti| !ti.is_valid())
        .ok_or(MangoError::SomeError)?;
    group.tokens[token_index] = TokenInfo {
        mint: ctx.accounts.mint.key(),
        decimals,
        reserved: [0u8; 31],
    };
    Ok(())
}
