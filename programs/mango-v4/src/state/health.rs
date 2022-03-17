use anchor_lang::prelude::*;
use anchor_spl::dex::serum_dex;
use fixed::types::I80F48;
use std::cell::Ref;

use crate::error::MangoError;
use crate::state::{oracle_price, Bank, MangoAccount};
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

struct TokenInfo<'a> {
    bank: Ref<'a, Bank>,
    oracle_price: I80F48, // native/native
    // in native tokens, summing token deposits/borrows and serum open orders
    balance: I80F48,
}

fn health_contribution(bank: &Bank, price: I80F48, balance: I80F48) -> Result<I80F48> {
    Ok(if balance.is_negative() {
        cm!(balance * price * bank.init_liab_weight)
    } else {
        cm!(balance * price * bank.init_asset_weight)
    })
}

fn pair_health(
    info1: &TokenInfo,
    balance1: I80F48,
    info2: &TokenInfo,
    balance2: I80F48,
) -> Result<I80F48> {
    let health1 = health_contribution(&info1.bank, info1.oracle_price, balance1)?;
    let health2 = health_contribution(&info2.bank, info2.oracle_price, balance2)?;
    Ok(cm!(health1 + health2))
}

fn strip_dex_padding<'a>(acc: &'a AccountInfo) -> Result<Ref<'a, [u8]>> {
    require!(acc.data_len() >= 12, MangoError::SomeError);
    let unpadded_data: Ref<[u8]> = Ref::map(acc.try_borrow_data()?, |data| {
        let data_len = data.len() - 12;
        let (_, rest) = data.split_at(5);
        let (mid, _) = rest.split_at(data_len);
        mid
    });
    Ok(unpadded_data)
}

pub fn load_open_orders<'a>(acc: &'a AccountInfo) -> Result<Ref<'a, serum_dex::state::OpenOrders>> {
    Ok(Ref::map(strip_dex_padding(acc)?, bytemuck::from_bytes))
}

fn compute_health_detail(
    account: &MangoAccount,
    banks: &[AccountInfo],
    oracles: &[AccountInfo],
    serum_oos: &[AccountInfo],
) -> Result<I80F48> {
    // collect the bank and oracle data once
    let mut token_infos = util::zip!(banks.iter(), oracles.iter())
        .map(|(bank_ai, oracle_ai)| {
            let bank = bank_ai.load::<Bank>()?;
            require!(bank.oracle == oracle_ai.key(), MangoError::UnexpectedOracle);
            let oracle_price = oracle_price(oracle_ai)?;
            Ok(TokenInfo {
                bank,
                oracle_price,
                balance: I80F48::ZERO,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // token contribution from token accounts
    for (position, token_info) in util::zip!(
        account.token_account_map.iter_active(),
        token_infos.iter_mut()
    ) {
        let bank = &token_info.bank;
        // This assumes banks are passed in order
        require!(
            bank.token_index == position.token_index,
            MangoError::SomeError
        );

        // converts the token value to the basis token value for health computations
        // TODO: health basis token == USDC?
        let native = position.native(&bank);
        token_info.balance = cm!(token_info.balance + native);
    }

    // token contribution from serum accounts
    for (serum_account, oo_ai) in
        util::zip!(account.serum_account_map.iter_active(), serum_oos.iter())
    {
        // This assumes serum open orders are passed in order
        require!(
            &serum_account.open_orders == oo_ai.key,
            MangoError::SomeError
        );

        // find the prices for the market
        // TODO: each of these is a linear scan - is that too expensive?
        let base_index = token_infos
            .iter()
            .position(|ti| ti.bank.token_index == serum_account.base_token_index)
            .ok_or(error!(MangoError::SomeError))?;
        let quote_index = token_infos
            .iter()
            .position(|ti| ti.bank.token_index == serum_account.quote_token_index)
            .ok_or(error!(MangoError::SomeError))?;

        let base_info = &token_infos[base_index];
        let quote_info = &token_infos[quote_index];
        let mut base = base_info.balance;
        let mut quote = quote_info.balance;

        let oo = load_open_orders(oo_ai)?;

        // add the amounts that are freely settleable
        let base_free = I80F48::from_num(oo.native_coin_free);
        let quote_free = I80F48::from_num(cm!(oo.native_pc_free + oo.referrer_rebates_accrued));
        base = cm!(base + base_free);
        quote = cm!(quote + quote_free);

        // for the amounts that are reserved for orders, compute the worst case for health
        // by checking if everything-is-base or everything-is-quote produces worse
        // outcomes
        let reserved_base = I80F48::from_num(cm!(oo.native_coin_total - oo.native_coin_free));
        let reserved_quote = I80F48::from_num(cm!(oo.native_pc_total - oo.native_pc_free));
        let all_in_base = cm!(base
            + reserved_base
            + reserved_quote * quote_info.oracle_price / base_info.oracle_price);
        let all_in_quote = cm!(quote
            + reserved_quote
            + reserved_base * base_info.oracle_price / quote_info.oracle_price);
        if pair_health(base_info, all_in_base, quote_info, quote)?
            < pair_health(base_info, base, quote_info, all_in_quote)?
        {
            base = all_in_base;
        } else {
            quote = all_in_quote;
        }

        token_infos[base_index].balance = base;
        token_infos[quote_index].balance = quote;
    }

    // convert the token balance to health
    let mut health = I80F48::ZERO;
    for token_info in token_infos.iter() {
        let contrib = health_contribution(
            &token_info.bank,
            token_info.oracle_price,
            token_info.balance,
        )?;
        health = cm!(health + contrib);
    }

    Ok(health)
}
