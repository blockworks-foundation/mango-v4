use anchor_lang::prelude::*;

use crate::state::Bank;

#[derive(Accounts)]
pub struct UpdateIndex<'info> {
    #[account(mut)]
    pub bank: AccountLoader<'info, Bank>,
}
pub fn update_index(ctx: Context<UpdateIndex>) -> Result<()> {
    let now_ts = Clock::get()?.unix_timestamp;

    let mut bank = ctx.accounts.bank.load_mut()?;
    bank.update_index(now_ts)?;

    Ok(())
}
