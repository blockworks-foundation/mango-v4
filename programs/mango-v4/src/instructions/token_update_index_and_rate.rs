use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::MangoError;
use crate::logs::{emit_stack, UpdateIndexLog, UpdateRateLogV2};
use crate::state::{OracleAccountInfos, HOUR};
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

        // Limit the maximal time interval that interest is applied for. This means we won't use
        // a fixed interest rate for a very long time period in exceptional circumstances, like
        // when there is a solana downtime or the security council disables this instruction.
        let max_interest_timestep = 3600; // hour
        let diff_ts =
            I80F48::from_num((now_ts - some_bank.index_last_updated).min(max_interest_timestep));

        let (deposit_index, borrow_index, borrow_fees, borrow_rate, deposit_rate) =
            some_bank.compute_index(indexed_total_deposits, indexed_total_borrows, diff_ts)?;

        some_bank.collected_fees_native += borrow_fees;

        let new_avg_utilization = some_bank.compute_new_avg_utilization(
            indexed_total_deposits,
            indexed_total_borrows,
            now_ts,
        );

        let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
        let price = some_bank.oracle_price(
            &OracleAccountInfos::from_reader(oracle_ref),
            Some(clock.slot),
        )?;

        some_bank
            .stable_price_model
            .update(now_ts as u64, price.to_num());
        let stable_price_model = some_bank.stable_price_model;

        // If a maint weight shift is done, copy the target into the normal values
        // and clear the transition parameters.
        let maint_shift_done = some_bank.maint_weight_shift_duration_inv.is_positive()
            && now_ts >= some_bank.maint_weight_shift_end;

        emit_stack(UpdateIndexLog {
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

            if maint_shift_done {
                bank.maint_asset_weight = bank.maint_weight_shift_asset_target;
                bank.maint_liab_weight = bank.maint_weight_shift_liab_target;
                bank.maint_weight_shift_duration_inv = I80F48::ZERO;
                bank.maint_weight_shift_asset_target = I80F48::ZERO;
                bank.maint_weight_shift_liab_target = I80F48::ZERO;
                bank.maint_weight_shift_start = 0;
                bank.maint_weight_shift_end = 0;
            }
        }
    }

    // compute optimal rates, and max rate and set them on the bank
    {
        let mut some_bank = ctx.remaining_accounts[0].load_mut::<Bank>()?;

        let diff_ts = I80F48::from_num(now_ts - some_bank.bank_rate_last_updated);

        // update each hour
        if diff_ts > HOUR {
            // First setup when new parameters are introduced
            if some_bank.interest_curve_scaling == 0.0 {
                let old_max_rate = 0.5;
                some_bank.interest_curve_scaling =
                    some_bank.max_rate.to_num::<f64>() / old_max_rate;
                some_bank.interest_target_utilization = some_bank.util0.to_num();

                let descale_factor = I80F48::from_num(1.0 / some_bank.interest_curve_scaling);
                some_bank.rate0 *= descale_factor;
                some_bank.rate1 *= descale_factor;
                some_bank.max_rate *= descale_factor;
            }

            some_bank.update_interest_rate_scaling();

            let rate0 = some_bank.rate0;
            let rate1 = some_bank.rate1;
            let max_rate = some_bank.max_rate;
            let scaling = some_bank.interest_curve_scaling;
            let target_util = some_bank.interest_target_utilization;

            emit_stack(UpdateRateLogV2 {
                mango_group: mint_info.group.key(),
                token_index: mint_info.token_index,
                rate0: rate0.to_bits(),
                util0: some_bank.util0.to_bits(),
                rate1: rate1.to_bits(),
                util1: some_bank.util1.to_bits(),
                max_rate: max_rate.to_bits(),
                curve_scaling: some_bank.interest_curve_scaling,
                target_utilization: some_bank.interest_target_utilization,
            });

            drop(some_bank);

            // Apply the new parameters to all banks
            for ai in ctx.remaining_accounts.iter() {
                let mut bank = ai.load_mut::<Bank>()?;

                bank.bank_rate_last_updated = now_ts;
                bank.interest_curve_scaling = scaling;
                bank.interest_target_utilization = target_util;
                bank.rate0 = rate0;
                bank.rate1 = rate1;
                bank.max_rate = max_rate;
            }
        }
    }

    Ok(())
}
