use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;

pub fn stub_oracle_create(ctx: Context<StubOracleCreate>, price: I80F48) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_init()?;
    oracle.group = ctx.accounts.group.key();
    oracle.mint = ctx.accounts.mint.key();
    oracle.price = price;
    oracle.last_update_ts = Clock::get()?.unix_timestamp;

    Ok(())
}
