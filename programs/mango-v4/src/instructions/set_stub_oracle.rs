use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::state::*;

#[derive(Accounts)]
pub struct SetStubOracle<'info> {
    #[account(mut)]
    pub stub_oracle: AccountLoader<'info, StubOracle>,
}

pub fn set_stub_oracle(ctx: Context<SetStubOracle>, price: I80F48) -> Result<()> {
    let mut stub_oracle = ctx.accounts.stub_oracle.load_init()?;
    stub_oracle.price = price;
    stub_oracle.last_updated = Clock::get()?.unix_timestamp;

    Ok(())
}
