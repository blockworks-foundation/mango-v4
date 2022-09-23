use std::time::Duration;

use client::{chain_data, health_cache, AccountFetcher, MangoClient, MangoClientError};
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::state::{
    Bank, HealthType, PerpMarketIndex, Serum3Orders, Side, TokenIndex, QUOTE_TOKEN_INDEX,
};

use itertools::Itertools;
use rand::seq::SliceRandom;
use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

pub struct Config {
    pub min_health_ratio: f64,
    pub refresh_timeout: Duration,
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
        health_cache::new(&mango_client.context, account_fetcher, &account).expect("always ok");
    let maint_health = health_cache.health(HealthType::Maint);
    if !health_cache.is_liquidatable() {
        return Ok(false);
    }

    log::trace!(
        "possible candidate: {}, with owner: {}, maint health: {}",
        pubkey,
        account.fixed.owner,
        maint_health,
    );

    // Fetch a fresh account and re-compute
    // This is -- unfortunately -- needed because the websocket streams seem to not
    // be great at providing timely updates to the account data.
    let account = account_fetcher.fetch_fresh_mango_account(pubkey)?;
    let health_cache =
        health_cache::new(&mango_client.context, account_fetcher, &account).expect("always ok");
    if !health_cache.is_liquidatable() {
        return Ok(false);
    }

    let maint_health = health_cache.health(HealthType::Maint);
    let is_spot_bankrupt = health_cache.can_call_spot_bankruptcy();
    let is_spot_liquidatable = health_cache.has_borrows() && !is_spot_bankrupt;

    // find asset and liab tokens
    let mut tokens = account
        .active_token_positions()
        .map(|token_position| {
            let token = mango_client.context.token(token_position.token_index);
            let bank = account_fetcher.fetch::<Bank>(&token.mint_info.first_bank())?;
            let oracle = account_fetcher.fetch_raw_account(&token.mint_info.oracle)?;
            let price = bank.oracle_price(&KeyedAccountSharedData::new(
                token.mint_info.oracle,
                oracle.into(),
            ))?;
            Ok((
                token_position.token_index,
                price,
                token_position.native(&bank) * price,
            ))
        })
        .collect::<anyhow::Result<Vec<(TokenIndex, I80F48, I80F48)>>>()?;
    tokens.sort_by(|a, b| a.2.cmp(&b.2));

    // look for any open serum orders or settleable balances
    let serum_force_cancels = account
        .active_serum3_orders()
        .map(|orders| {
            let open_orders_account = account_fetcher.fetch_raw_account(&orders.open_orders)?;
            let open_orders = mango_v4::serum3_cpi::load_open_orders(&open_orders_account)?;
            let can_force_cancel = open_orders.native_coin_total > 0
                || open_orders.native_pc_total > 0
                || open_orders.referrer_rebates_accrued > 0;
            if can_force_cancel {
                Ok(Some(*orders))
            } else {
                Ok(None)
            }
        })
        .filter_map_ok(|v| v)
        .collect::<anyhow::Result<Vec<Serum3Orders>>>()?;

    // look for any perp open orders and base positions
    let perp_force_cancels = account
        .active_perp_positions()
        .filter_map(|pp| pp.has_open_orders().then(|| pp.market_index))
        .collect::<Vec<PerpMarketIndex>>();
    let mut perp_base_positions = account
        .active_perp_positions()
        .map(|pp| {
            let base_lots = pp.base_position_lots();
            if base_lots == 0 {
                return Ok(None);
            }
            let perp = mango_client.context.perp(pp.market_index);
            let oracle = account_fetcher.fetch_raw_account(&perp.market.oracle)?;
            let price = perp.market.oracle_price(&KeyedAccountSharedData::new(
                perp.market.oracle,
                oracle.into(),
            ))?;
            Ok(Some((
                pp.market_index,
                base_lots,
                price,
                I80F48::from(base_lots.abs()) * price,
            )))
        })
        .filter_map_ok(|v| v)
        .collect::<anyhow::Result<Vec<(PerpMarketIndex, i64, I80F48, I80F48)>>>()?;
    // sort by base_position_value, ascending
    perp_base_positions.sort_by(|a, b| a.3.cmp(&b.3));

    let get_max_liab_transfer = |source, target| -> anyhow::Result<I80F48> {
        let mut liqor = account_fetcher
            .fetch_fresh_mango_account(&mango_client.mango_account_address)
            .context("getting liquidator account")?;

        // Ensure the tokens are activated, so they appear in the health cache and
        // max_swap_source() will work.
        liqor.ensure_token_position(source)?;
        liqor.ensure_token_position(target)?;

        let health_cache =
            health_cache::new(&mango_client.context, account_fetcher, &liqor).expect("always ok");

        let source_price = health_cache.token_info(source).unwrap().oracle_price;
        let target_price = health_cache.token_info(target).unwrap().oracle_price;
        // TODO: This is where we could multiply in the liquidation fee factors
        let oracle_swap_price = source_price / target_price;

        let amount = health_cache
            .max_swap_source_for_health_ratio(source, target, oracle_swap_price, min_health_ratio)
            .context("getting max_swap_source")?;
        Ok(amount)
    };

    // try liquidating
    let txsig = if !serum_force_cancels.is_empty() {
        // Cancel all orders on a random serum market
        let serum_orders = serum_force_cancels.choose(&mut rand::thread_rng()).unwrap();
        let sig = mango_client.serum3_liq_force_cancel_orders(
            (pubkey, &account),
            serum_orders.market_index,
            &serum_orders.open_orders,
        )?;
        log::info!(
            "Force cancelled serum orders on account {}, market index {}, maint_health was {}, tx sig {:?}",
            pubkey,
            serum_orders.market_index,
            maint_health,
            sig
        );
        sig
    } else if !perp_force_cancels.is_empty() {
        // Cancel all orders on a random perp market
        let perp_market_index = *perp_force_cancels.choose(&mut rand::thread_rng()).unwrap();
        let sig =
            mango_client.perp_liq_force_cancel_orders((pubkey, &account), perp_market_index)?;
        log::info!(
            "Force cancelled perp orders on account {}, market index {}, maint_health was {}, tx sig {:?}",
            pubkey,
            perp_market_index,
            maint_health,
            sig
        );
        sig
    } else if !perp_base_positions.is_empty() {
        // Liquidate the highest-value perp base position
        let (perp_market_index, base_lots, price, _) = perp_base_positions.last().unwrap();
        let perp = mango_client.context.perp(*perp_market_index);

        let (side, side_signum) = if *base_lots > 0 {
            (Side::Bid, 1)
        } else {
            (Side::Ask, -1)
        };

        // Compute the max number of base_lots the liqor is willing to take
        let max_base_transfer_abs = {
            let mut liqor = account_fetcher
                .fetch_fresh_mango_account(&mango_client.mango_account_address)
                .context("getting liquidator account")?;
            liqor.ensure_perp_position(*perp_market_index, QUOTE_TOKEN_INDEX)?;
            let health_cache = health_cache::new(&mango_client.context, account_fetcher, &liqor)
                .expect("always ok");
            health_cache.max_perp_for_health_ratio(
                *perp_market_index,
                *price,
                perp.market.base_lot_size,
                side,
                min_health_ratio,
            )?
        };
        log::info!("computed max_base_transfer to be {max_base_transfer_abs}");

        let sig = mango_client.perp_liq_base_position(
            (pubkey, &account),
            *perp_market_index,
            side_signum * max_base_transfer_abs,
        )?;
        log::info!(
            "Liquidated base position for perp market on account {}, market index {}, maint_health was {}, tx sig {:?}",
            pubkey,
            perp_market_index,
            maint_health,
            sig
        );
        sig
    } else if is_spot_bankrupt {
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
            .token_liq_bankruptcy((pubkey, &account), liab_token_index, max_liab_transfer)
            .context("sending liq_token_bankruptcy")?;
        log::info!(
            "Liquidated bankruptcy for {}, maint_health was {}, tx sig {:?}",
            pubkey,
            maint_health,
            sig
        );
        sig
    } else if is_spot_liquidatable {
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
        //
        let sig = mango_client
            .token_liq_with_token(
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
        anyhow::bail!(
            "Don't know what to do with liquidatable account {}, maint_health was {}",
            pubkey,
            maint_health
        );
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
