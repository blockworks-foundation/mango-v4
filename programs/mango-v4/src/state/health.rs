use anchor_lang::prelude::*;
use fixed::types::I80F48;
use pyth_client::load_price;

use crate::error::MangoError;
use crate::state::{determine_oracle_type, Bank, MangoAccount, OracleType, StubOracle};
use crate::util;
use crate::util::checked_math as cm;

pub fn compute_health(account: &MangoAccount, ais: &[AccountInfo]) -> Result<I80F48> {
    let active_len = account.token_account_map.iter_active().count();
    require!(
        ais.len() == active_len * 2, // banks + oracles
        MangoError::SomeError
    );

    let banks = &ais[0..active_len];
    let oracles = &ais[active_len..active_len * 2];

    compute_health_detail(account, banks, oracles)
}

fn compute_health_detail(
    account: &MangoAccount,
    banks: &[AccountInfo],
    oracles: &[AccountInfo],
) -> Result<I80F48> {
    let mut assets = I80F48::ZERO;
    let mut liabilities = I80F48::ZERO; // absolute value
    for (position, (bank_ai, oracle_ai)) in util::zip!(
        account.token_account_map.iter_active(),
        banks.iter(),
        oracles.iter()
    ) {
        let bank_loader = AccountLoader::<'_, Bank>::try_from(bank_ai)?;
        let bank = bank_loader.load()?;

        // TODO: This assumes banks are passed in order - is that an ok assumption?
        require!(
            bank.token_index == position.token_index,
            MangoError::SomeError
        );

        // converts the token value to the basis token value for health computations
        // TODO: health basis token == USDC?
        let oracle_data = &oracle_ai.try_borrow_data()?;
        let oracle_type = determine_oracle_type(oracle_data)?;
        require!(bank.oracle == oracle_ai.key(), MangoError::UnexpectedOracle);

        let price = match oracle_type {
            OracleType::Stub => {
                AccountLoader::<'_, StubOracle>::try_from(oracle_ai)?
                    .load()?
                    .price
            }
            OracleType::Pyth => {
                let price_struct = load_price(&oracle_data).unwrap();
                I80F48::from_num(price_struct.agg.price)
            }
        };

        let native_position = position.native(&bank);
        let native_basis = cm!(native_position * price);
        if native_basis.is_positive() {
            assets = cm!(assets + bank.init_asset_weight * native_basis);
        } else {
            liabilities = cm!(liabilities - bank.init_liab_weight * native_basis);
        }
    }

    // TODO: Serum open orders
    // - for each active serum market, pass the OpenOrders in order
    // - store the base_token_index and quote_token_index in the account, so we don't
    //   need to also pass SerumMarket
    // - find the bank and oracle for base and quote, and add appropriately

    Ok(cm!(assets - liabilities))
}
