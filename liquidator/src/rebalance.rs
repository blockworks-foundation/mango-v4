use crate::{account_shared_data::KeyedAccountSharedData, AnyhowWrap};

use client::{chain_data, AccountFetcher, MangoClient, TokenContext};
use mango_v4::state::{oracle_price, Bank, TokenIndex, TokenPosition, QUOTE_TOKEN_INDEX};

use {fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

use std::{collections::HashMap, time::Duration};

pub struct Config {
    pub slippage: f64,
    pub refresh_timeout: Duration,
}

#[derive(Debug)]
struct TokenState {
    _price: I80F48,
    native_position: I80F48,
}

impl TokenState {
    fn new_position(
        token: &TokenContext,
        position: &TokenPosition,
        account_fetcher: &chain_data::AccountFetcher,
    ) -> anyhow::Result<Self> {
        let bank = account_fetcher.fetch::<Bank>(&token.mint_info.first_bank())?;
        Ok(Self {
            _price: Self::fetch_price(token, &bank, account_fetcher)?,
            native_position: position.native(&bank),
        })
    }

    fn fetch_price(
        token: &TokenContext,
        bank: &Bank,
        account_fetcher: &chain_data::AccountFetcher,
    ) -> anyhow::Result<I80F48> {
        let oracle = account_fetcher.fetch_raw_account(token.mint_info.oracle)?;
        oracle_price(
            &KeyedAccountSharedData::new(token.mint_info.oracle, oracle.into()),
            bank.oracle_config.conf_filter,
            bank.mint_decimals,
        )
        .map_err_anyhow()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn zero_all_non_quote(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    mango_account_address: &Pubkey,
    config: &Config,
) -> anyhow::Result<()> {
    log::trace!("checking for rebalance: {}", mango_account_address);

    // TODO: configurable?
    let quote_token = mango_client.context.token(QUOTE_TOKEN_INDEX);

    let account = account_fetcher.fetch_mango_account(mango_account_address)?;

    let tokens = account
        .token_iter_active()
        .map(|token_position| {
            let token = mango_client.context.token(token_position.token_index);
            Ok((
                token.token_index,
                TokenState::new_position(token, token_position, account_fetcher)?,
            ))
        })
        .collect::<anyhow::Result<HashMap<TokenIndex, TokenState>>>()?;
    log::trace!("account tokens: {:?}", tokens);

    let mut txsigs = vec![];
    for (token_index, token_state) in tokens {
        let token = mango_client.context.token(token_index);
        if token_index == quote_token.token_index {
            continue;
        }

        if token_state.native_position > 0 {
            let amount = token_state.native_position;
            let txsig = mango_client.jupiter_swap(
                token.mint_info.mint,
                quote_token.mint_info.mint,
                amount.to_num::<u64>(),
                config.slippage,
                client::JupiterSwapMode::ExactIn,
            )?;
            log::info!(
                "sold {} {} for {} in tx {}",
                token.native_to_ui(amount),
                token.name,
                quote_token.name,
                txsig
            );
            txsigs.push(txsig);
        } else if token_state.native_position < 0 {
            let amount = (-token_state.native_position).ceil();
            let txsig = mango_client.jupiter_swap(
                quote_token.mint_info.mint,
                token.mint_info.mint,
                amount.to_num::<u64>(),
                config.slippage,
                client::JupiterSwapMode::ExactOut,
            )?;
            log::info!(
                "bought {} {} for {} in tx {}",
                token.native_to_ui(amount),
                token.name,
                quote_token.name,
                txsig
            );
            txsigs.push(txsig);
        }
    }

    let max_slot = account_fetcher.transaction_max_slot(&txsigs)?;
    if let Err(e) = account_fetcher.refresh_accounts_via_rpc_until_slot(
        &[*mango_account_address],
        max_slot,
        config.refresh_timeout,
    ) {
        // If we don't get fresh data, maybe the tx landed on a fork?
        // Rebalance is technically still ok.
        log::info!("could not refresh account data: {}", e);
    }

    Ok(())
}
