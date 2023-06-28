use std::time::Duration;

use mango_v4::state::TokenConditionalSwap;
use mango_v4_client::{chain_data, health_cache, JupiterSwapMode, MangoClient};

use rand::seq::SliceRandom;
use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

use crate::{token_swap_info, util};

pub struct Config {
    pub min_health_ratio: f64,
    pub refresh_timeout: Duration,
    pub mock_jupiter: bool,
}

async fn tcs_is_executable(
    mango_client: &MangoClient,
    tcs: &TokenConditionalSwap,
) -> anyhow::Result<bool> {
    let buy_token_price = mango_client.bank_oracle_price(tcs.buy_token_index).await?;
    let sell_token_price = mango_client.bank_oracle_price(tcs.sell_token_index).await?;
    let base_price = (buy_token_price / sell_token_price).to_num();
    let premium_price = tcs.premium_price(base_price);
    let maker_price = tcs.maker_price(premium_price);

    if !tcs.price_threshold_reached(base_price) || maker_price > tcs.price_limit {
        return Ok(false);
    }

    return Ok(true);
}

fn tcs_has_plausible_premium(
    tcs: &TokenConditionalSwap,
    token_swap_info: &token_swap_info::TokenSwapInfoUpdater,
) -> anyhow::Result<bool> {
    // The premium the taker receives needs to take taker fees into account
    let premium = tcs.taker_price(tcs.premium_price(1.0)) as f64;

    // Never take tcs where the fee exceeds the premium and the triggerer exchanges
    // tokens at below oracle price.
    if premium < 1.0 {
        return Ok(false);
    }

    let buy_info = token_swap_info
        .swap_info(tcs.buy_token_index)
        .ok_or_else(|| anyhow::anyhow!("no swap info for token {}", tcs.buy_token_index))?;
    let sell_info = token_swap_info
        .swap_info(tcs.sell_token_index)
        .ok_or_else(|| anyhow::anyhow!("no swap info for token {}", tcs.sell_token_index))?;

    // If this is 1.0 then the exchange can (probably) happen at oracle price.
    // 1.5 would mean we need to pay 50% more than oracle etc.
    let cost = buy_info.buy_over_oracle * sell_info.sell_over_oracle;

    Ok(cost <= premium)
}

async fn tcs_is_interesting(
    mango_client: &MangoClient,
    tcs: &TokenConditionalSwap,
    token_swap_info: &token_swap_info::TokenSwapInfoUpdater,
) -> anyhow::Result<bool> {
    Ok(tcs_is_executable(mango_client, tcs).await?
        && tcs_has_plausible_premium(tcs, token_swap_info)?)
}

#[allow(clippy::too_many_arguments)]
pub async fn maybe_execute_token_conditional_swap(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    token_swap_info: &token_swap_info::TokenSwapInfoUpdater,
    pubkey: &Pubkey,
    config: &Config,
) -> anyhow::Result<bool> {
    let liqee = account_fetcher.fetch_mango_account(pubkey)?;

    // Check for triggerable conditional swap and good health
    let tcs_id;
    {
        let mut tcs_id_inner = None;
        let mut tcs_shuffled = liqee.active_token_conditional_swap().collect::<Vec<&_>>();
        {
            let mut rng = rand::thread_rng();
            tcs_shuffled.shuffle(&mut rng);
        }
        for tcs in tcs_shuffled {
            if tcs_is_interesting(mango_client, tcs, token_swap_info).await? {
                tcs_id_inner = Some(tcs.id);
                break;
            }
        }
        if tcs_id_inner.is_none() {
            return Ok(false);
        }
        tcs_id = tcs_id_inner.unwrap();

        let health_cache = health_cache::new(&mango_client.context, account_fetcher, &liqee)
            .await
            .context("creating health cache 1")?;
        if health_cache.is_liquidatable() {
            return Ok(false);
        }
    }

    // get a fresh account and re-check the tcs and health
    let liqee = account_fetcher.fetch_fresh_mango_account(pubkey).await?;
    let (_, tcs) = liqee.token_conditional_swap_by_id(tcs_id)?;
    if !tcs_is_interesting(mango_client, tcs, token_swap_info).await? {
        return Ok(false);
    }

    let health_cache = health_cache::new(&mango_client.context, account_fetcher, &liqee)
        .await
        .context("creating health cache 1")?;
    if health_cache.is_liquidatable() {
        return Ok(false);
    }

    // TODO: if it's expired, just trigger it to close it?

    let liqor_min_health_ratio = I80F48::from_num(config.min_health_ratio);

    // Compute the max viable swap (for liqor and liqee) and min it
    let buy_token_price = mango_client.bank_oracle_price(tcs.buy_token_index).await?;
    let sell_token_price = mango_client.bank_oracle_price(tcs.sell_token_index).await?;

    let base_price = buy_token_price / sell_token_price;
    let premium_price = tcs.premium_price(base_price.to_num());
    let maker_price = I80F48::from_num(tcs.maker_price(premium_price));
    let taker_price = I80F48::from_num(tcs.taker_price(premium_price));

    // TODO: configurable
    let max_take_quote = I80F48::from(1_000_000_000);

    let max_sell_token_to_liqor = util::max_swap_source(
        mango_client,
        account_fetcher,
        &liqee,
        tcs.sell_token_index,
        tcs.buy_token_index,
        I80F48::ONE / maker_price,
        I80F48::from_num(0.5), // TODO: explain that this target relates to the 1% closure target in the program
    )
    .await?
    .min(max_take_quote / sell_token_price)
    .floor()
    .to_num::<u64>()
    .min(tcs.remaining_sell());

    let max_buy_token_to_liqee = util::max_swap_source(
        mango_client,
        account_fetcher,
        &mango_client.mango_account().await?,
        tcs.buy_token_index,
        tcs.sell_token_index,
        taker_price,
        liqor_min_health_ratio,
    )
    .await?
    .min(max_take_quote / buy_token_price)
    .floor()
    .to_num::<u64>()
    .min(tcs.remaining_buy());

    if max_sell_token_to_liqor == 0 || max_buy_token_to_liqee == 0 {
        return Ok(false);
    }

    // Final check of the reverse trade on jupiter
    {
        let buy_mint = mango_client.context.mint_info(tcs.buy_token_index).mint;
        let sell_mint = mango_client.context.mint_info(tcs.sell_token_index).mint;
        let slippage = 100; // TODO: configurable
        let swap_mode = JupiterSwapMode::ExactIn;
        let input_amount = max_sell_token_to_liqor.min(
            (I80F48::from(max_buy_token_to_liqee) * taker_price)
                .floor()
                .to_num(),
        );
        let route = util::jupiter_route(
            mango_client,
            sell_mint,
            buy_mint,
            input_amount,
            slippage,
            swap_mode,
            config.mock_jupiter,
        )
        .await?;

        let sell_amount = route.in_amount.parse::<f64>()?;
        let buy_amount = route.out_amount.parse::<f64>()?;
        let swap_price = sell_amount / buy_amount;

        if swap_price > taker_price.to_num::<f64>() {
            log::trace!(
                "skipping token conditional swap for: {pubkey}, id: {tcs_id}, \
                max_buy: {max_buy_token_to_liqee}, max_sell: {max_sell_token_to_liqor}, \
                because counter swap price: {swap_price} while taker price: {taker_price}",
            );
            return Ok(false);
        }
    }

    log::trace!(
        "executing token conditional swap for: {}, with owner: {}, id: {}, max_buy: {}, max_sell: {}",
        pubkey,
        liqee.fixed.owner,
        tcs_id,
        max_buy_token_to_liqee,
        max_sell_token_to_liqor,
    );

    let txsig = mango_client
        .token_conditional_swap_trigger(
            (pubkey, &liqee),
            tcs.id,
            max_buy_token_to_liqee,
            max_sell_token_to_liqor,
        )
        .await?;
    log::info!(
        "Executed swap account {}, tcs index {}, tx sig {:?}",
        pubkey,
        tcs_id,
        txsig
    );

    let slot = account_fetcher.transaction_max_slot(&[txsig]).await?;
    if let Err(e) = account_fetcher
        .refresh_accounts_via_rpc_until_slot(
            &[*pubkey, mango_client.mango_account_address],
            slot,
            config.refresh_timeout,
        )
        .await
    {
        log::info!("could not refresh after tcs: {}", e);
    }

    Ok(true)
}
