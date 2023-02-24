use std::collections::HashMap;

use anchor_lang::prelude::*;

use fixed::types::I80F48;

use crate::events::{Equity, TokenEquity};
use crate::health::ScanningAccountRetriever;

use super::MangoAccountRef;

pub fn compute_equity(
    account: &MangoAccountRef,
    retriever: &ScanningAccountRetriever,
) -> Result<Equity> {
    let mut token_equity_map = HashMap::new();

    // token contributions
    for (_i, position) in account.active_token_positions().enumerate() {
        let (bank, oracle_price) = retriever.scanned_bank_and_oracle(position.token_index)?;
        // converts the token value to the basis token value for health computations
        // TODO: health basis token == USDC?
        let native = position.native(bank);
        token_equity_map.insert(bank.token_index, native * oracle_price);
    }

    // token contributions from Serum3
    for (_i, serum_account) in account.active_serum3_orders().enumerate() {
        let oo = retriever.scanned_serum_oo(&serum_account.open_orders)?;

        // note base token value
        let (_bank, oracle_price) =
            retriever.scanned_bank_and_oracle(serum_account.base_token_index)?;
        let accumulated_equity = token_equity_map
            .get(&serum_account.base_token_index)
            .unwrap_or(&I80F48::ZERO);
        let native_coin_total_i80f48 = I80F48::from_num(oo.native_coin_total);
        let new_equity = accumulated_equity + native_coin_total_i80f48 * oracle_price;
        token_equity_map.insert(serum_account.base_token_index, new_equity);

        // note quote token value
        let (_bank, oracle_price) =
            retriever.scanned_bank_and_oracle(serum_account.quote_token_index)?;
        let accumulated_equity = token_equity_map
            .get(&serum_account.quote_token_index)
            .unwrap_or(&I80F48::ZERO);
        let native_pc_total_i80f48 = I80F48::from_num(oo.native_pc_total);
        let new_equity = accumulated_equity + native_pc_total_i80f48 * oracle_price;
        token_equity_map.insert(serum_account.quote_token_index, new_equity);
    }

    let tokens = token_equity_map
        .iter()
        .map(|tuple| TokenEquity {
            token_index: *tuple.0,
            value: *tuple.1,
        })
        .collect::<Vec<TokenEquity>>();

    // TODO: perp contributions
    let perps = Vec::new();

    Ok(Equity { tokens, perps })
}
