use anchor_lang::prelude::*;

use crate::state::Bank;

#[derive(Accounts)]
pub struct UpdateIndex<'info> {
    // TODO: should we support arbitrary number of banks with remaining accounts?
    // ix - consumed 17641 of 101000 compute units, so we have a lot of compute
    #[account(mut)]
    pub bank: AccountLoader<'info, Bank>,
}
pub fn update_index(ctx: Context<UpdateIndex>) -> Result<()> {
    // TODO: should we enforce a minimum window between 2 update_index ix calls?
    let now_ts = Clock::get()?.unix_timestamp;

    let mut bank = ctx.accounts.bank.load_mut()?;
    bank.update_index(now_ts)?;

    Ok(())
}
