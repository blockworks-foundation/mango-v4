use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use fixed::types::I80F48;

use crate::{error::MangoError, state::*};

#[derive(Accounts)]
pub struct StubOracleCreate<'info> {
    #[account(
        has_one = admin,
        constraint = group.load()?.is_ix_enabled(IxGate::StubOracleCreate) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        init,
        seeds = [b"StubOracle".as_ref(), group.key().as_ref(), mint.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<StubOracle>(),
    )]
    pub oracle: AccountLoader<'info, StubOracle>,

    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn stub_oracle_create(ctx: Context<StubOracleCreate>, price: I80F48) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_init()?;
    oracle.group = ctx.accounts.group.key();
    oracle.mint = ctx.accounts.mint.key();
    oracle.price = price;
    oracle.last_updated = Clock::get()?.unix_timestamp;

    Ok(())
}
