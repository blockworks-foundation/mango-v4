use anchor_lang::prelude::*;
use fixed::types::I80F48;
use std::cell::Ref;

use crate::error::MangoError;
use crate::state::{oracle_price, Bank, MangoAccount, TokenIndex};
use crate::util;
use crate::util::checked_math as cm;
use crate::util::LoadZeroCopy;

pub fn compute_health(account: &MangoAccount, ais: &[AccountInfo]) -> Result<I80F48> {
    let active_token_len = account.token_account_map.iter_active().count();
    let active_serum_len = account.serum_account_map.iter_active().count();
    let expected_ais = active_token_len * 2 // banks + oracles
        + active_serum_len; // open_orders
    require!(ais.len() == expected_ais, MangoError::SomeError);
    let banks = &ais[0..active_token_len];
    let oracles = &ais[active_token_len..active_token_len * 2];
    let serum_oos = &ais[active_token_len * 2..];

    compute_health_detail(account, banks, oracles, serum_oos)
}

struct BankAndPrice<'a> {
    bank: Ref<'a, Bank>,
    price: I80F48,
}

fn find_price(token_index: TokenIndex, banks_and_prices: &[BankAndPrice]) -> Result<I80F48> {
    Ok(banks_and_prices
        .iter()
        .find(|b| b.bank.token_index == token_index)
        .ok_or(error!(MangoError::SomeError))?
        .price)
}

fn compute_health_detail(
    account: &MangoAccount,
    banks: &[AccountInfo],
    oracles: &[AccountInfo],
    serum_oos: &[AccountInfo],
) -> Result<I80F48> {
    let mut assets = I80F48::ZERO;
    let mut liabilities = I80F48::ZERO; // absolute value

    // collect the bank and oracle data once
    let banks_and_prices = util::zip!(banks.iter(), oracles.iter())
        .map(|(bank_ai, oracle_ai)| {
            let bank = bank_ai.load::<Bank>()?;
            require!(bank.oracle == oracle_ai.key(), MangoError::UnexpectedOracle);
            let price = oracle_price(oracle_ai)?;
            Ok(BankAndPrice { bank, price })
        })
        .collect::<Result<Vec<BankAndPrice>>>()?;

    // health contribution from token accounts
    for (position, BankAndPrice { bank, price }) in util::zip!(
        account.token_account_map.iter_active(),
        banks_and_prices.iter()
    ) {
        // This assumes banks are passed in order
        require!(
            bank.token_index == position.token_index,
            MangoError::SomeError
        );

        // converts the token value to the basis token value for health computations
        // TODO: health basis token == USDC?
        let price = *price;
        let native_position = position.native(&bank);
        let native_basis = cm!(native_position * price);
        if native_basis.is_positive() {
            assets = cm!(assets + bank.init_asset_weight * native_basis);
        } else {
            liabilities = cm!(liabilities - bank.init_liab_weight * native_basis);
        }
    }

    // health contribution from serum accounts
    for (serum_account, oo_ai) in
        util::zip!(account.serum_account_map.iter_active(), serum_oos.iter())
    {
        // This assumes serum open orders are passed in order
        require!(
            &serum_account.open_orders == oo_ai.key,
            MangoError::SomeError
        );

        // find the prices for the market
        // TODO: each of these is a linear scan through banks_and_prices - is that too expensive?
        let _base_price = find_price(serum_account.base_token_index, &banks_and_prices)?;
        let _quote_price = find_price(serum_account.quote_token_index, &banks_and_prices)?;
    }

    Ok(cm!(assets - liabilities))
}
