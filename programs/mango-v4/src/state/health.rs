use std::cell::RefMut;

use anchor_lang::prelude::*;
use fixed::types::I80F48;
use pyth_client::load_price;

use crate::error::MangoError;
use crate::state::{determine_oracle_type, MangoAccount, OracleType, StubOracle, TokenBank};

macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.zip(
            zip!($($y), +))
    )
}

pub fn compute_health(
    account: &mut RefMut<MangoAccount>,
    banks: &[AccountInfo],
    oracles: &[AccountInfo],
) -> Result<I80F48> {
    let mut assets = I80F48::ZERO;
    let mut liabilities = I80F48::ZERO; // absolute value
    for (position, (bank_ai, oracle_ai)) in zip!(
        account.indexed_positions.iter_active(),
        banks.iter(),
        oracles.iter()
    ) {
        let bank_loader = AccountLoader::<'_, TokenBank>::try_from(bank_ai)?;
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

        let native_basis = position.native(&bank) * price;
        if native_basis.is_positive() {
            assets += bank.init_asset_weight * native_basis;
        } else {
            liabilities -= bank.init_liab_weight * native_basis;
        }
    }
    Ok(assets - liabilities)
}
