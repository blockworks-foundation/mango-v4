use crate::AnyhowWrap;

use client::{chain_data, AccountFetcher, MangoClient, TokenContext};
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::state::{Bank, TokenIndex, TokenPosition, QUOTE_TOKEN_INDEX};

use {fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

use futures::{stream, StreamExt, TryStreamExt};
use std::{collections::HashMap, time::Duration};

#[derive(Clone)]
pub struct Config {
    /// Maximum slippage allowed in Jupiter
    pub slippage: f64,
    /// When closing borrows, the rebalancer can't close token positions exactly.
    /// Instead it purchases too much and then gets rid of the excess in a second step.
    /// If this is 1.05, then it'll swap borrow_value * 1.05 quote token into borrow token.
    pub borrow_settle_excess: f64,
    pub refresh_timeout: Duration,
}

#[derive(Debug)]
struct TokenState {
    price: I80F48,
    native_position: I80F48,
}

impl TokenState {
    async fn new_position(
        token: &TokenContext,
        position: &TokenPosition,
        account_fetcher: &chain_data::AccountFetcher,
    ) -> anyhow::Result<Self> {
        let bank = Self::bank(token, account_fetcher)?;
        Ok(Self {
            price: Self::fetch_price(token, &bank, account_fetcher).await?,
            native_position: position.native(&bank),
        })
    }

    fn bank(
        token: &TokenContext,
        account_fetcher: &chain_data::AccountFetcher,
    ) -> anyhow::Result<Bank> {
        account_fetcher.fetch::<Bank>(&token.mint_info.first_bank())
    }

    async fn fetch_price(
        token: &TokenContext,
        bank: &Bank,
        account_fetcher: &chain_data::AccountFetcher,
    ) -> anyhow::Result<I80F48> {
        let oracle = account_fetcher
            .fetch_raw_account(&token.mint_info.oracle)
            .await?;
        bank.oracle_price(
            &KeyedAccountSharedData::new(token.mint_info.oracle, oracle.into()),
            None,
        )
        .map_err_anyhow()
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn zero_all_non_quote(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    mango_account_address: &Pubkey,
    config: &Config,
) -> anyhow::Result<()> {
    log::trace!("checking for rebalance: {}", mango_account_address);

    // TODO: configurable?
    let quote_token = mango_client.context.token(QUOTE_TOKEN_INDEX);

    let account = account_fetcher.fetch_mango_account(mango_account_address)?;

    let tokens: anyhow::Result<HashMap<TokenIndex, TokenState>> =
        stream::iter(account.active_token_positions())
            .then(|token_position| async {
                let token = mango_client.context.token(token_position.token_index);
                Ok((
                    token.token_index,
                    TokenState::new_position(token, token_position, account_fetcher).await?,
                ))
            })
            .try_collect()
            .await;
    let tokens = tokens?;
    log::trace!("account tokens: {:?}", tokens);

    // Function to refresh the mango account after the txsig confirmed. Returns false on timeout.
    let refresh_mango_account = |txsig| async move {
        let res: anyhow::Result<bool> = {
            let max_slot = account_fetcher.transaction_max_slot(&[txsig]).await?;
            if let Err(e) = account_fetcher
                .refresh_accounts_via_rpc_until_slot(
                    &[*mango_account_address],
                    max_slot,
                    config.refresh_timeout,
                )
                .await
            {
                // If we don't get fresh data, maybe the tx landed on a fork?
                // Rebalance is technically still ok.
                log::info!("could not refresh account data: {}", e);
                return Ok(false);
            }
            Ok(true)
        };
        res
    };

    for (token_index, token_state) in tokens {
        let token = mango_client.context.token(token_index);
        if token_index == quote_token.token_index {
            continue;
        }
        let token_mint = token.mint_info.mint;
        let quote_mint = quote_token.mint_info.mint;

        // It's not always possible to bring the native balance to 0 through swaps:
        // Consider a price <1. You need to sell a bunch of tokens to get 1 USDC native and
        // similarly will get multiple tokens when buying.
        // Imagine SOL at 0.04 USDC-native per SOL-native: Any amounts below 25 SOL-native
        // would not be worth a single USDC-native.
        //
        // To avoid errors, we consider all amounts below 2 * (1/oracle) dust and don't try
        // to sell them. Instead they will be withdrawn at the end.
        // Purchases will aim to purchase slightly more than is needed, such that we can
        // again withdraw the dust at the end.
        let dust_threshold = I80F48::from(2) / token_state.price;

        let mut amount = token_state.native_position;

        if amount < 0 {
            // Buy
            let buy_amount = amount.abs().ceil() + (dust_threshold - I80F48::ONE).max(I80F48::ZERO);
            let input_amount =
                buy_amount * token_state.price * I80F48::from_num(config.borrow_settle_excess);
            let txsig = mango_client
                .jupiter_swap(
                    quote_mint,
                    token_mint,
                    input_amount.to_num::<u64>(),
                    config.slippage,
                    client::JupiterSwapMode::ExactIn,
                )
                .await?;
            log::info!(
                "bought {} {} for {} in tx {}",
                token.native_to_ui(buy_amount),
                token.name,
                quote_token.name,
                txsig
            );
            if !refresh_mango_account(txsig).await? {
                return Ok(());
            }
            let bank = TokenState::bank(token, account_fetcher)?;
            amount = mango_client
                .mango_account()
                .await?
                .token_position_and_raw_index(token_index)
                .map(|(position, _)| position.native(&bank))
                .unwrap_or(I80F48::ZERO);
        }

        if amount > dust_threshold {
            // Sell
            let txsig = mango_client
                .jupiter_swap(
                    token_mint,
                    quote_mint,
                    amount.to_num::<u64>(),
                    config.slippage,
                    client::JupiterSwapMode::ExactIn,
                )
                .await?;
            log::info!(
                "sold {} {} for {} in tx {}",
                token.native_to_ui(amount),
                token.name,
                quote_token.name,
                txsig
            );
            if !refresh_mango_account(txsig).await? {
                return Ok(());
            }
            let bank = TokenState::bank(token, account_fetcher)?;
            amount = mango_client
                .mango_account()
                .await?
                .token_position_and_raw_index(token_index)
                .map(|(position, _)| position.native(&bank))
                .unwrap_or(I80F48::ZERO);
        }

        // Any remainder that could not be sold just gets withdrawn to ensure the
        // TokenPosition is freed up
        if amount > 0 && amount <= dust_threshold {
            let allow_borrow = false;
            let txsig = mango_client
                .token_withdraw(token_mint, amount.to_num::<u64>(), allow_borrow)
                .await?;
            log::info!(
                "withdrew {} {} to liqor wallet in {}",
                token.native_to_ui(amount),
                token.name,
                txsig
            );
            if !refresh_mango_account(txsig).await? {
                return Ok(());
            }
        } else if amount > dust_threshold {
            anyhow::bail!(
                "unexpected {} position after rebalance swap: {} native",
                token.name,
                amount
            );
        }
    }

    Ok(())
}
