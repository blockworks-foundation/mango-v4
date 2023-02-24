use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::MangoError;
use crate::logs::{UpdateIndexLog, UpdateRateLog};
use crate::state::HOUR;
use crate::{
    accounts_zerocopy::{AccountInfoRef, LoadMutZeroCopyRef, LoadZeroCopyRef},
    state::Bank,
};
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;
use anchor_lang::Discriminator;

use fixed::types::I80F48;

pub mod compute_budget {
    use solana_program::declare_id;
    declare_id!("ComputeBudget111111111111111111111111111111");
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
                (ix.program_id == crate::id()
                    && ix.data[0..8]
                        == crate::instruction::TokenUpdateIndexAndRate::discriminator())
                    || (ix.program_id == compute_budget::id()),
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

    let clock = Clock::get()?;
    let now_ts: u64 = clock.unix_timestamp.try_into().unwrap();

    // compute indexed_total
    let mut indexed_total_deposits = I80F48::ZERO;
    let mut indexed_total_borrows = I80F48::ZERO;
    for ai in ctx.remaining_accounts.iter() {
        let bank = ai.load::<Bank>()?;
        indexed_total_deposits += bank.indexed_deposits;
        indexed_total_borrows += bank.indexed_borrows;
    }

    // compute and set latest index and average utilization on each bank
    // also update moving average prices
    {
        let mut some_bank = ctx.remaining_accounts[0].load_mut::<Bank>()?;

        let diff_ts = I80F48::from_num(now_ts - some_bank.index_last_updated);

        let (deposit_index, borrow_index, borrow_fees, borrow_rate, deposit_rate) =
            some_bank.compute_index(indexed_total_deposits, indexed_total_borrows, diff_ts)?;

        some_bank.collected_fees_native += borrow_fees;

        let new_avg_utilization = some_bank.compute_new_avg_utilization(
            indexed_total_deposits,
            indexed_total_borrows,
            now_ts,
        );

        let price = some_bank.oracle_price(
            &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
            Some(clock.slot),
        )?;

        some_bank
            .stable_price_model
            .update(now_ts as u64, price.to_num());
        let stable_price_model = some_bank.stable_price_model;

        emit!(UpdateIndexLog {
            mango_group: mint_info.group.key(),
            token_index: mint_info.token_index,
            deposit_index: deposit_index.to_bits(),
            borrow_index: borrow_index.to_bits(),
            avg_utilization: new_avg_utilization.to_bits(),
            price: price.to_bits(),
            stable_price: some_bank.stable_price().to_bits(),
            collected_fees: some_bank.collected_fees_native.to_bits(),
            loan_fee_rate: some_bank.loan_fee_rate.to_bits(),
            total_deposits: (deposit_index * indexed_total_deposits).to_bits(),
            total_borrows: (borrow_index * indexed_total_borrows).to_bits(),
            borrow_rate: borrow_rate.to_bits(),
            deposit_rate: deposit_rate.to_bits(),
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

            bank.index_last_updated = now_ts;

            bank.deposit_index = deposit_index;
            bank.borrow_index = borrow_index;

            bank.avg_utilization = new_avg_utilization;

            bank.stable_price_model = stable_price_model;
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
