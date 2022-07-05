use anchor_lang::prelude::*;

use crate::logs::UpdateIndexLog;
use crate::{
    accounts_zerocopy::{LoadMutZeroCopyRef, LoadZeroCopyRef},
    state::{Bank, MintInfo},
};
use checked_math as cm;
use fixed::types::I80F48;
#[derive(Accounts)]
pub struct UpdateIndex<'info> {
    pub mint_info: AccountLoader<'info, MintInfo>,
}

pub fn update_index(ctx: Context<UpdateIndex>) -> Result<()> {
    ctx.accounts
        .mint_info
        .load()?
        .verify_banks_ais(ctx.remaining_accounts)?;

    let mut indexed_total_deposits = I80F48::ZERO;
    let mut indexed_total_borrows = I80F48::ZERO;
    for ai in ctx.remaining_accounts.iter() {
        let bank = ai.load::<Bank>()?;
        indexed_total_deposits = cm!(indexed_total_deposits + bank.indexed_deposits);
        indexed_total_borrows = cm!(indexed_total_borrows + bank.indexed_borrows);
    }

    let now_ts = Clock::get()?.unix_timestamp;
    let (diff_ts, deposit_index, borrow_index) = {
        let mut some_bank = ctx.remaining_accounts[0].load_mut::<Bank>()?;

        // TODO: should we enforce a minimum window between 2 update_index ix calls?
        let diff_ts = I80F48::from_num(now_ts - some_bank.last_updated);

        let (deposit_index, borrow_index) =
            some_bank.compute_index(indexed_total_deposits, indexed_total_borrows, diff_ts)?;

        (diff_ts, deposit_index, borrow_index)
    };

    msg!("indexed_total_deposits {}", indexed_total_deposits);
    msg!("indexed_total_borrows {}", indexed_total_borrows);
    msg!("diff_ts {}", diff_ts);
    msg!("deposit_index {}", deposit_index);
    msg!("borrow_index {}", borrow_index);

    for ai in ctx.remaining_accounts.iter() {
        let mut bank = ai.load_mut::<Bank>()?;

        bank.cached_indexed_total_deposits = indexed_total_deposits;
        bank.cached_indexed_total_borrows = indexed_total_borrows;

        bank.last_updated = now_ts;
        bank.charge_loan_fee(diff_ts);

        bank.deposit_index = deposit_index;
        bank.borrow_index = borrow_index;

        // clarkeni TODO: add prices
        emit!(UpdateIndexLog {
            mango_group: bank.group.key(),
            token_index: bank.token_index,
            deposit_index: bank.deposit_index.to_bits(),
            borrow_index: bank.borrow_index.to_bits(),
            // price: oracle_price.to_bits(),
        });
    }

    Ok(())
}
