use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::{error::MangoError, state::*};

#[derive(Accounts)]
pub struct StubOracleSet<'info> {
    #[account(
        has_one = admin,
        constraint = group.load()?.is_operational() @ MangoError::GroupIsHalted
    )]
    pub group: AccountLoader<'info, Group>,

    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub oracle: AccountLoader<'info, StubOracle>,
}

pub fn stub_oracle_set(ctx: Context<StubOracleSet>, price: I80F48) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    oracle.price = price;
    oracle.last_updated = Clock::get()?.unix_timestamp;

    Ok(())
}
