use anchor_lang::prelude::*;

use crate::logs::UpdateIndexLog;
use crate::state::Bank;

#[derive(Accounts)]
pub struct UpdateIndex<'info> {
    // TODO: should we support arbitrary number of banks with remaining accounts?
    // ix - consumed 17641 of 101000 compute units, so we have a lot of compute
    #[account(mut)]
    pub bank: AccountLoader<'info, Bank>,
    // pub oracle: UncheckedAccount<'info>,
}
pub fn update_index(ctx: Context<UpdateIndex>) -> Result<()> {
    // TODO: should we enforce a minimum window between 2 update_index ix calls?
    let now_ts = Clock::get()?.unix_timestamp;

    let mut bank = ctx.accounts.bank.load_mut()?;
    bank.update_index(now_ts)?;

    // clarkeni TODO: add prices
    emit!(UpdateIndexLog {
        mango_group: bank.group.key(),
        token_index: bank.token_index,
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
        // price: oracle_price.to_bits(),
    });

    Ok(())
}
