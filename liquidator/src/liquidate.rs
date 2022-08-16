use std::time::Duration;

use crate::account_shared_data::KeyedAccountSharedData;

use client::{chain_data, AccountFetcher, MangoClient, MangoClientError, MangoGroupContext};
use mango_v4::state::{
    new_health_cache, oracle_price, Bank, FixedOrderAccountRetriever, HealthCache, HealthType,
    MangoAccountValue, TokenIndex, QUOTE_TOKEN_INDEX,
};

use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

pub struct Config {
    pub min_health_ratio: f64,
    pub refresh_timeout: Duration,
}

pub fn new_health_cache_(
    context: &MangoGroupContext,
    account_fetcher: &chain_data::AccountFetcher,
    account: &MangoAccountValue,
) -> anyhow::Result<HealthCache> {
    let active_token_len = account.token_iter_active().count();
    let active_perp_len = account.perp_iter_active_accounts().count();

    let metas = context.derive_health_check_remaining_account_metas(account, vec![], false)?;
    let accounts = metas
        .iter()
        .map(|meta| {
            Ok(KeyedAccountSharedData::new(
                meta.pubkey,
                account_fetcher.fetch_raw(&meta.pubkey)?,
            ))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let retriever = FixedOrderAccountRetriever {
        ais: accounts,
        n_banks: active_token_len,
        begin_perp: active_token_len * 2,
        begin_serum3: active_token_len * 2 + active_perp_len,
    };
    new_health_cache(&account.borrow(), &retriever).context("make health cache")
}

pub fn jupiter_market_can_buy(
    mango_client: &MangoClient,
    token: TokenIndex,
    quote_token: TokenIndex,
) -> bool {
    if token == quote_token {
        return true;
    }
    let token_mint = mango_client.context.token(token).mint_info.mint;
    let quote_token_mint = mango_client.context.token(quote_token).mint_info.mint;

    // Consider a market alive if we can swap $10 worth at 1% slippage
    // TODO: configurable
    // TODO: cache this, no need to recheck often
    let quote_amount = 10_000_000u64;
    let slippage = 1.0;
    mango_client
        .jupiter_route(
            quote_token_mint,
            token_mint,
            quote_amount,
            slippage,
            client::JupiterSwapMode::ExactIn,
        )
        .is_ok()
}

pub fn jupiter_market_can_sell(
    mango_client: &MangoClient,
    token: TokenIndex,
    quote_token: TokenIndex,
) -> bool {
    if token == quote_token {
        return true;
    }
    let token_mint = mango_client.context.token(token).mint_info.mint;
    let quote_token_mint = mango_client.context.token(quote_token).mint_info.mint;

    // Consider a market alive if we can swap $10 worth at 1% slippage
    // TODO: configurable
    // TODO: cache this, no need to recheck often
    let quote_amount = 10_000_000u64;
    let slippage = 1.0;
    mango_client
        .jupiter_route(
            token_mint,
            quote_token_mint,
            quote_amount,
            slippage,
            client::JupiterSwapMode::ExactOut,
        )
        .is_ok()
}

#[allow(clippy::too_many_arguments)]
pub fn maybe_liquidate_account(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    pubkey: &Pubkey,
    config: &Config,
) -> anyhow::Result<bool> {
    let min_health_ratio = I80F48::from_num(config.min_health_ratio);
    let quote_token_index = 0;

    let account = account_fetcher.fetch_mango_account(pubkey)?;
    let health_cache =
        new_health_cache_(&mango_client.context, account_fetcher, &account).expect("always ok");
    let maint_health = health_cache.health(HealthType::Maint);
    let is_bankrupt = !health_cache.has_liquidatable_assets() && health_cache.has_borrows();

    if maint_health >= 0 && !is_bankrupt {
        return Ok(false);
    }

    log::trace!(
        "possible candidate: {}, with owner: {}, maint health: {}, bankrupt: {}",
        pubkey,
        account.fixed.owner,
        maint_health,
        is_bankrupt,
    );

    // Fetch a fresh account and re-compute
    // This is -- unfortunately -- needed because the websocket streams seem to not
    // be great at providing timely updates to the account data.
    let account = account_fetcher.fetch_fresh_mango_account(pubkey)?;
    let health_cache =
        new_health_cache_(&mango_client.context, account_fetcher, &account).expect("always ok");
    let maint_health = health_cache.health(HealthType::Maint);
    let is_bankrupt = !health_cache.has_liquidatable_assets() && health_cache.has_borrows();

    // find asset and liab tokens
    let mut tokens = account
        .token_iter_active()
        .map(|token_position| {
            let token = mango_client.context.token(token_position.token_index);
            let bank = account_fetcher.fetch::<Bank>(&token.mint_info.first_bank())?;
            let oracle = account_fetcher.fetch_raw_account(token.mint_info.oracle)?;
            let price = oracle_price(
                &KeyedAccountSharedData::new(token.mint_info.oracle, oracle.into()),
                bank.oracle_config.conf_filter,
                bank.mint_decimals,
            )?;
            Ok((
                token_position.token_index,
                price,
                token_position.native(&bank) * price,
            ))
        })
        .collect::<anyhow::Result<Vec<(TokenIndex, I80F48, I80F48)>>>()?;
    tokens.sort_by(|a, b| a.2.cmp(&b.2));

    let get_max_liab_transfer = |source, target| -> anyhow::Result<I80F48> {
        let mut liqor = account_fetcher
            .fetch_fresh_mango_account(&mango_client.mango_account_address)
            .context("getting liquidator account")?;

        // Ensure the tokens are activated, so they appear in the health cache and
        // max_swap_source() will work.
        liqor.token_get_mut_or_create(source)?;
        liqor.token_get_mut_or_create(target)?;

        let health_cache =
            new_health_cache_(&mango_client.context, account_fetcher, &liqor).expect("always ok");
        let amount = health_cache
            .max_swap_source_for_health_ratio(source, target, min_health_ratio)
            .context("getting max_swap_source")?;
        Ok(amount)
    };

    // try liquidating
    let txsig = if is_bankrupt {
        if tokens.is_empty() {
            anyhow::bail!("mango account {}, is bankrupt has no active tokens", pubkey);
        }
        let liab_token_index = tokens
            .iter()
            .find(|(liab_token_index, _liab_price, liab_usdc_equivalent)| {
                liab_usdc_equivalent.is_negative()
                    && jupiter_market_can_buy(mango_client, *liab_token_index, QUOTE_TOKEN_INDEX)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "mango account {}, has no liab tokens that are purchasable for USDC: {:?}",
                    pubkey,
                    tokens
                )
            })?
            .0;

        let max_liab_transfer = get_max_liab_transfer(liab_token_index, quote_token_index)?;

        let sig = mango_client
            .liq_token_bankruptcy((pubkey, &account), liab_token_index, max_liab_transfer)
            .context("sending liq_token_bankruptcy")?;
        log::info!(
            "Liquidated bankruptcy for {}, maint_health was {}, tx sig {:?}",
            pubkey,
            maint_health,
            sig
        );
        sig
    } else if maint_health.is_negative() {
        let asset_token_index = tokens
            .iter()
            .rev()
            .find(|(asset_token_index, _asset_price, asset_usdc_equivalent)| {
                asset_usdc_equivalent.is_positive()
                    && jupiter_market_can_sell(mango_client, *asset_token_index, QUOTE_TOKEN_INDEX)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "mango account {}, has no asset tokens that are sellable for USDC: {:?}",
                    pubkey,
                    tokens
                )
            })?
            .0;
        let liab_token_index = tokens
            .iter()
            .find(|(liab_token_index, _liab_price, liab_usdc_equivalent)| {
                liab_usdc_equivalent.is_negative()
                    && jupiter_market_can_buy(mango_client, *liab_token_index, QUOTE_TOKEN_INDEX)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "mango account {}, has no liab tokens that are purchasable for USDC: {:?}",
                    pubkey,
                    tokens
                )
            })?
            .0;

        let max_liab_transfer = get_max_liab_transfer(liab_token_index, asset_token_index)
            .context("getting max_liab_transfer")?;

        //
        // TODO: log liqor's assets in UI form
        // TODO: log liquee's liab_needed, need to refactor program code to be able to be accessed from client side
        // TODO: swap inherited liabs to desired asset for liqor
        //
        let sig = mango_client
            .liq_token_with_token(
                (pubkey, &account),
                asset_token_index,
                liab_token_index,
                max_liab_transfer,
            )
            .context("sending liq_token_with_token")?;
        log::info!(
            "Liquidated token with token for {}, maint_health was {}, tx sig {:?}",
            pubkey,
            maint_health,
            sig
        );
        sig
    } else {
        return Ok(false);
    };

    let slot = account_fetcher.transaction_max_slot(&[txsig])?;
    if let Err(e) = account_fetcher.refresh_accounts_via_rpc_until_slot(
        &[*pubkey, mango_client.mango_account_address],
        slot,
        config.refresh_timeout,
    ) {
        log::info!("could not refresh after liquidation: {}", e);
    }

    Ok(true)
}

#[allow(clippy::too_many_arguments)]
pub fn maybe_liquidate_one<'a>(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    accounts: impl Iterator<Item = &'a Pubkey>,
    config: &Config,
) -> bool {
    for pubkey in accounts {
        match maybe_liquidate_account(mango_client, account_fetcher, pubkey, config) {
            Err(err) => {
                // Not all errors need to be raised to the user's attention.
                let mut log_level = log::Level::Error;

                // Simulation errors due to liqee precondition failures on the liquidation instructions
                // will commonly happen if our liquidator is late or if there are chain forks.
                match err.downcast_ref::<MangoClientError>() {
                    Some(MangoClientError::SendTransactionPreflightFailure { logs }) => {
                        if logs.contains("HealthMustBeNegative") || logs.contains("IsNotBankrupt") {
                            log_level = log::Level::Trace;
                        }
                    }
                    _ => {}
                };
                log::log!(log_level, "liquidating account {}: {:?}", pubkey, err);
            }
            Ok(true) => return true,
            _ => {}
        };
    }

    false
}
