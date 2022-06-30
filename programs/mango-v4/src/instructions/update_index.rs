use anchor_lang::prelude::*;

use crate::logs::UpdateIndexLog;
use crate::{
    accounts_zerocopy::{AccountInfoRef, LoadMutZeroCopyRef, LoadZeroCopyRef},
    error::MangoError,
    state::{oracle_price, Bank, MintInfo},
};
use checked_math as cm;
use fixed::types::I80F48;
#[derive(Accounts)]
pub struct UpdateIndex<'info> {
    pub mint_info: AccountLoader<'info, MintInfo>,
    pub oracle: UncheckedAccount<'info>,
}

pub fn update_index(ctx: Context<UpdateIndex>) -> Result<()> {
    let mint_info = ctx.accounts.mint_info.load()?;
    require_keys_eq!(mint_info.oracle.key(), ctx.accounts.oracle.key());

    let total_banks = mint_info
        .banks
        .iter()
        .filter(|bank| *bank != &Pubkey::default())
        .count();

    require_eq!(total_banks, ctx.remaining_accounts.len());
    let all_banks = ctx.remaining_accounts;
    check_banks(all_banks, &mint_info)?;

    let mut indexed_total_deposits = I80F48::ZERO;
    let mut indexed_total_borrows = I80F48::ZERO;
    for ai in all_banks.iter() {
        let bank = ai.load::<Bank>()?;
        indexed_total_deposits = cm!(indexed_total_deposits + bank.indexed_deposits);
        indexed_total_borrows = cm!(indexed_total_borrows + bank.indexed_borrows);
    }

    let now_ts = Clock::get()?.unix_timestamp;
    let (diff_ts, deposit_index, borrow_index, oracle_conf_filter, base_token_decimals) = {
        let mut some_bank = all_banks[0].load_mut::<Bank>()?;

        // TODO: should we enforce a minimum window between 2 update_index ix calls?
        let diff_ts = I80F48::from_num(now_ts - some_bank.last_updated);

        let (deposit_index, borrow_index) =
            some_bank.compute_index(indexed_total_deposits, indexed_total_borrows, diff_ts)?;

        (
            diff_ts,
            deposit_index,
            borrow_index,
            some_bank.oracle_config.conf_filter,
            some_bank.mint_decimals,
        )
    };

    msg!("indexed_total_deposits {}", indexed_total_deposits);
    msg!("indexed_total_borrows {}", indexed_total_borrows);
    msg!("diff_ts {}", diff_ts);
    msg!("deposit_index {}", deposit_index);
    msg!("borrow_index {}", borrow_index);

    for ai in all_banks.iter() {
        let mut bank = ai.load_mut::<Bank>()?;

        bank.indexed_total_deposits = indexed_total_deposits;
        bank.indexed_total_borrows = indexed_total_borrows;

        bank.last_updated = now_ts;
        bank.charge_loan_fee(diff_ts);

        bank.deposit_index = deposit_index;
        bank.borrow_index = borrow_index;
    }

    let price = oracle_price(
        &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
        oracle_conf_filter,
        base_token_decimals,
    )?;

    emit!(UpdateIndexLog {
        mango_group: mint_info.group.key(),
        token_index: mint_info.token_index,
        deposit_index: deposit_index.to_bits(),
        borrow_index: borrow_index.to_bits(),
        price: price.to_bits(),
    });

    Ok(())
}

fn check_banks(all_banks: &[AccountInfo], mint_info: &MintInfo) -> Result<()> {
    for (idx, ai) in all_banks.iter().enumerate() {
        match ai.load::<Bank>() {
            Ok(bank) => {
                if mint_info.token_index != bank.token_index
                    || mint_info.group != bank.group
                    // todo: just below check should be enough, above 2 checks are superfluous and defensive
                    || mint_info.banks[idx] != ai.key()
                {
                    return Err(error!(MangoError::SomeError));
                }
            }
            Err(error) => return Err(error),
        }
    }

    Ok(())
}
