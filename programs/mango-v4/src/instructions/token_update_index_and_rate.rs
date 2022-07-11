use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::logs::{UpdateIndexLog, UpdateRateLog};
use crate::state::HOUR;
use crate::{
    accounts_zerocopy::{AccountInfoRef, LoadMutZeroCopyRef, LoadZeroCopyRef},
    state::{oracle_price, Bank, MintInfo},
};
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;
use checked_math as cm;
use fixed::types::I80F48;

#[derive(Accounts)]
pub struct TokenUpdateIndexAndRate<'info> {
    #[account(
        has_one = oracle
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    pub oracle: UncheckedAccount<'info>,

    #[account(address = tx_instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

pub fn token_update_index_and_rate(ctx: Context<TokenUpdateIndexAndRate>) -> Result<()> {
    {
        let ixs = ctx.accounts.instructions.as_ref();

        let mut index = 0;
        loop {
            let ix = match tx_instructions::load_instruction_at_checked(index, ixs) {
                Ok(ix) => ix,
                Err(ProgramError::InvalidArgument) => break,
                Err(e) => return Err(e.into()),
            };

            // 1. we want to forbid token deposit and token withdraw and similar
            // (serum3 place order could be used as a withdraw and a serum3 cancel order as a deposit)
            // to be called in same tx as this ix to prevent index or rate manipulation,
            // for now we just whitelist to other token_update_index_and_rate ix
            // 2. we want to forbid cpi, since ix we would like to blacklist could just be called from cpi
            require!(
                ix.program_id == crate::id()
                    && ix.data[0..8] == [131, 136, 194, 39, 11, 50, 10, 198], // token_update_index_and_rate
                MangoError::SomeError
            );

            index += 1;
        }
    }

    let mint_info = ctx.accounts.mint_info.load()?;

    ctx.accounts
        .mint_info
        .load()?
        .verify_banks_ais(ctx.remaining_accounts)?;

    let now_ts = Clock::get()?.unix_timestamp;

    // compute indexed_total
    let mut indexed_total_deposits = I80F48::ZERO;
    let mut indexed_total_borrows = I80F48::ZERO;
    for ai in ctx.remaining_accounts.iter() {
        let bank = ai.load::<Bank>()?;
        indexed_total_deposits = cm!(indexed_total_deposits + bank.indexed_deposits);
        indexed_total_borrows = cm!(indexed_total_borrows + bank.indexed_borrows);
    }

    // compute and set latest index and average utilization on each bank
    {
        let some_bank = ctx.remaining_accounts[0].load::<Bank>()?;

        let now_ts_i80f48 = I80F48::from_num(now_ts);
        let diff_ts = I80F48::from_num(now_ts - some_bank.index_last_updated);

        let (deposit_index, borrow_index) =
            some_bank.compute_index(indexed_total_deposits, indexed_total_borrows, diff_ts)?;

        let new_avg_utilization = some_bank.compute_new_avg_utilization(
            indexed_total_deposits,
            indexed_total_borrows,
            now_ts_i80f48,
        );

        let price = oracle_price(
            &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
            some_bank.oracle_config.conf_filter,
            some_bank.mint_decimals,
        )?;
        emit!(UpdateIndexLog {
            mango_group: mint_info.group.key(),
            token_index: mint_info.token_index,
            deposit_index: deposit_index.to_bits(),
            borrow_index: borrow_index.to_bits(),
            avg_utilization: new_avg_utilization.to_bits(),
            price: price.to_bits()
        });

        drop(some_bank);

        msg!("indexed_total_deposits {}", indexed_total_deposits);
        msg!("indexed_total_borrows {}", indexed_total_borrows);
        msg!("diff_ts {}", diff_ts);
        msg!("deposit_index {}", deposit_index);
        msg!("borrow_index {}", borrow_index);
        msg!("avg_utilization {}", new_avg_utilization);

        for ai in ctx.remaining_accounts.iter() {
            let mut bank = ai.load_mut::<Bank>()?;

            bank.cached_indexed_total_deposits = indexed_total_deposits;
            bank.cached_indexed_total_borrows = indexed_total_borrows;

            bank.index_last_updated = now_ts;
            bank.charge_loan_fee(diff_ts);

            bank.deposit_index = deposit_index;
            bank.borrow_index = borrow_index;

            bank.avg_utilization = new_avg_utilization;
        }
    }

    // compute optimal rates, and max rate and set them on the bank
    {
        let some_bank = ctx.remaining_accounts[0].load::<Bank>()?;

        let diff_ts = I80F48::from_num(now_ts - some_bank.bank_rate_last_updated);

        // update each hour
        if diff_ts > HOUR {
            let (rate0, rate1, max_rate) = some_bank.compute_rates();

            emit!(UpdateRateLog {
                mango_group: mint_info.group.key(),
                token_index: mint_info.token_index,
                rate0: rate0.to_bits(),
                rate1: rate1.to_bits(),
                max_rate: max_rate.to_bits(),
            });

            drop(some_bank);

            msg!("rate0 {}", rate0);
            msg!("rate1 {}", rate1);
            msg!("max_rate {}", max_rate);

            for ai in ctx.remaining_accounts.iter() {
                let mut bank = ai.load_mut::<Bank>()?;

                bank.bank_rate_last_updated = now_ts;
                bank.rate0 = rate0;
                bank.rate1 = rate1;
                bank.max_rate = max_rate;
            }
        }
    }

    Ok(())
}
