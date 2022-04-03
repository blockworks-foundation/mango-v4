use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use fixed::types::I80F48;

use crate::state::*;

#[derive(Accounts)]
pub struct CreateStubOracle<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"StubOracle".as_ref(), token_mint.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<StubOracle>(),
    )]
    pub oracle: AccountLoader<'info, StubOracle>,

    pub admin: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_stub_oracle(ctx: Context<CreateStubOracle>, price: I80F48) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_init()?;
    oracle.group = ctx.accounts.group.key();
    oracle.price = price;
    oracle.last_updated = Clock::get()?.unix_timestamp;

    Ok(())
}
