use std::time::Duration;

use itertools::Itertools;
use mango_v4::{
    i80f48::ClampToInt,
    state::{Bank, MangoAccountValue, TokenConditionalSwap},
};
use mango_v4_client::{chain_data, health_cache, JupiterSwapMode, MangoClient, MangoGroupContext};

use tracing::*;
use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

use crate::{token_swap_info, util};

// The liqee health ratio to aim for when executing tcs orders that are bigger
// than the liqee can support.
//
// The background here is that the program considers bringing the liqee health ratio
// below 1% as "the tcs was completely fulfilled" and then closes the tcs.
// Choosing a value too close to 0 is problematic, since then small oracle fluctuations
// could bring the final health below 0 and make the triggering invalid!
const TARGET_HEALTH_RATIO: f64 = 0.5;

pub struct Config {
    pub min_health_ratio: f64,
    pub max_trigger_quote_amount: u64,
    pub refresh_timeout: Duration,
    pub mock_jupiter: bool,
    pub compute_limit_for_trigger: u32,
}

fn tcs_is_in_price_range(
    context: &MangoGroupContext,
    account_fetcher: &chain_data::AccountFetcher,
    tcs: &TokenConditionalSwap,
) -> anyhow::Result<bool> {
    let buy_bank = context.mint_info(tcs.buy_token_index).first_bank();
    let sell_bank = context.mint_info(tcs.sell_token_index).first_bank();
    let buy_token_price = account_fetcher.fetch_bank_price(&buy_bank)?;
    let sell_token_price = account_fetcher.fetch_bank_price(&sell_bank)?;
    let base_price = (buy_token_price / sell_token_price).to_num();
    if !tcs.price_in_range(base_price) {
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

fn tcs_is_interesting(
    context: &MangoGroupContext,
    account_fetcher: &chain_data::AccountFetcher,
    tcs: &TokenConditionalSwap,
    token_swap_info: &token_swap_info::TokenSwapInfoUpdater,
    now_ts: u64,
) -> anyhow::Result<bool> {
    Ok(tcs.is_expired(now_ts)
        || (tcs_is_in_price_range(context, account_fetcher, tcs)?
            && tcs_has_plausible_premium(tcs, token_swap_info)?))
}

#[allow(clippy::too_many_arguments)]
async fn maybe_execute_token_conditional_swap_inner(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    token_swap_info: &token_swap_info::TokenSwapInfoUpdater,
    pubkey: &Pubkey,
    liqee_old: &MangoAccountValue,
    tcs_id: u64,
    config: &Config,
    now_ts: u64,
) -> anyhow::Result<bool> {
    let health_cache = health_cache::new(&mango_client.context, account_fetcher, &liqee_old)
        .await
        .context("creating health cache 1")?;
    if health_cache.is_liquidatable() {
        return Ok(false);
    }

    // get a fresh account and re-check the tcs and health
    let liqee = account_fetcher.fetch_fresh_mango_account(pubkey).await?;
    let (_, tcs) = liqee.token_conditional_swap_by_id(tcs_id)?;
    if tcs.is_expired(now_ts)
        || !tcs_is_interesting(
            &mango_client.context,
            account_fetcher,
            tcs,
            token_swap_info,
            now_ts,
        )?
    {
        return Ok(false);
    }

    let health_cache = health_cache::new(&mango_client.context, account_fetcher, &liqee)
        .await
        .context("creating health cache 1")?;
    if health_cache.is_liquidatable() {
        return Ok(false);
    }

    execute_token_conditional_swap(mango_client, account_fetcher, pubkey, config, &liqee, tcs).await
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, fields(%pubkey, tcs_id = tcs.id))]
async fn execute_token_conditional_swap(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    pubkey: &Pubkey,
    config: &Config,
    liqee: &MangoAccountValue,
    tcs: &TokenConditionalSwap,
) -> anyhow::Result<bool> {
    let liqor_min_health_ratio = I80F48::from_num(config.min_health_ratio);

    // Compute the max viable swap (for liqor and liqee) and min it
    let buy_bank = mango_client
        .context
        .mint_info(tcs.buy_token_index)
        .first_bank();
    let sell_bank = mango_client
        .context
        .mint_info(tcs.sell_token_index)
        .first_bank();
    let buy_token_price = account_fetcher.fetch_bank_price(&buy_bank)?;
    let sell_token_price = account_fetcher.fetch_bank_price(&sell_bank)?;

    let base_price = buy_token_price / sell_token_price;
    let premium_price = tcs.premium_price(base_price.to_num());
    let maker_price = I80F48::from_num(tcs.maker_price(premium_price));
    let taker_price = I80F48::from_num(tcs.taker_price(premium_price));

    let max_take_quote = I80F48::from(config.max_trigger_quote_amount);

    let liqee_target_health_ratio = I80F48::from_num(TARGET_HEALTH_RATIO);

    let max_sell_token_to_liqor = util::max_swap_source(
        mango_client,
        account_fetcher,
        &liqee,
        tcs.sell_token_index,
        tcs.buy_token_index,
        I80F48::ONE / maker_price,
        liqee_target_health_ratio,
    )?
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
    )?
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
        let swap_mode = JupiterSwapMode::ExactIn;
        // The slippage does not matter since we're not going to execute it
        let slippage = 100;
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
            false,
            config.mock_jupiter,
        )
        .await?;

        let sell_amount = route.in_amount as f64;
        let buy_amount = route.out_amount as f64;
        let swap_price = sell_amount / buy_amount;

        if swap_price > taker_price.to_num::<f64>() {
            trace!(
                max_buy = max_buy_token_to_liqee,
                max_sell = max_sell_token_to_liqor,
                jupiter_swap_price = %swap_price,
                tcs_taker_price = %taker_price,
                "skipping token conditional swap because of prices",
            );
            return Ok(false);
        }
    }

    trace!(
        max_buy = max_buy_token_to_liqee,
        max_sell = max_sell_token_to_liqor,
        "executing token conditional swap",
    );

    let compute_ix = solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
        config.compute_limit_for_trigger,
    );
    let trigger_ix = mango_client
        .token_conditional_swap_trigger_instruction(
            (pubkey, &liqee),
            tcs.id,
            max_buy_token_to_liqee,
            max_sell_token_to_liqor,
        )
        .await?;
    let txsig = mango_client
        .send_and_confirm_owner_tx(vec![compute_ix, trigger_ix])
        .await?;
    info!(
        %txsig,
        "Executed token conditional swap",
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
        info!(%txsig, "could not refresh after tcs execution: {}", e);
    }

    Ok(true)
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, fields(%pubkey, %tcs_id))]
pub async fn remove_expired_token_conditional_swap(
    mango_client: &MangoClient,
    pubkey: &Pubkey,
    liqee: &MangoAccountValue,
    tcs_id: u64,
) -> anyhow::Result<bool> {
    let ix = mango_client
        .token_conditional_swap_trigger_instruction((pubkey, &liqee), tcs_id, 0, 0)
        .await?;
    let txsig = mango_client.send_and_confirm_owner_tx(vec![ix]).await?;
    info!(
        %txsig,
        "Removed expired token conditional swap",
    );

    Ok(true)
}

#[allow(clippy::too_many_arguments)]
pub async fn maybe_execute_token_conditional_swap(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    token_swap_info: &token_swap_info::TokenSwapInfoUpdater,
    pubkey: &Pubkey,
    tcs_id: u64,
    config: &Config,
) -> anyhow::Result<bool> {
    let now_ts: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        .try_into()?;
    let liqee = account_fetcher.fetch_mango_account(pubkey)?;
    let tcs = liqee.token_conditional_swap_by_id(tcs_id)?.1;

    if tcs.is_expired(now_ts) {
        remove_expired_token_conditional_swap(mango_client, pubkey, &liqee, tcs.id).await
    } else {
        maybe_execute_token_conditional_swap_inner(
            mango_client,
            account_fetcher,
            token_swap_info,
            pubkey,
            &liqee,
            tcs.id,
            config,
            now_ts,
        )
        .await
    }
}

/// Returns the maximum execution size of a tcs order in quote units
fn tcs_max_volume(
    account: &MangoAccountValue,
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    tcs: &TokenConditionalSwap,
) -> anyhow::Result<u64> {
    // Compute the max viable swap (for liqor and liqee) and min it
    let buy_bank_pk = mango_client
        .context
        .mint_info(tcs.buy_token_index)
        .first_bank();
    let sell_bank_pk = mango_client
        .context
        .mint_info(tcs.sell_token_index)
        .first_bank();
    let buy_bank: Bank = account_fetcher.fetch(&buy_bank_pk)?;
    let sell_bank: Bank = account_fetcher.fetch(&sell_bank_pk)?;
    let buy_token_price = account_fetcher.fetch_bank_price(&buy_bank_pk)?;
    let sell_token_price = account_fetcher.fetch_bank_price(&sell_bank_pk)?;

    let buy_position = account
        .token_position(tcs.buy_token_index)
        .map(|p| p.native(&buy_bank))
        .unwrap_or(I80F48::ZERO);
    let sell_position = account
        .token_position(tcs.sell_token_index)
        .map(|p| p.native(&sell_bank))
        .unwrap_or(I80F48::ZERO);

    let base_price = buy_token_price / sell_token_price;
    let premium_price = tcs.premium_price(base_price.to_num());
    let maker_price = tcs.maker_price(premium_price);

    let liqee_target_health_ratio = I80F48::from_num(TARGET_HEALTH_RATIO);

    let max_sell = util::max_swap_source(
        mango_client,
        account_fetcher,
        &account,
        tcs.sell_token_index,
        tcs.buy_token_index,
        I80F48::from_num(1.0 / maker_price),
        liqee_target_health_ratio,
    )?
    .floor()
    .to_num::<u64>()
    .min(tcs.max_sell_for_position(sell_position, &sell_bank));

    let max_buy = tcs.max_buy_for_position(buy_position, &buy_bank);

    let max_quote =
        (I80F48::from(max_buy) * buy_token_price).min(I80F48::from(max_sell) * sell_token_price);

    Ok(max_quote.floor().clamp_to_u64())
}

pub fn find_interesting_tcs_for_account(
    pubkey: &Pubkey,
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    token_swap_info: &token_swap_info::TokenSwapInfoUpdater,
    now_ts: u64,
) -> anyhow::Result<Vec<anyhow::Result<(Pubkey, u64, u64)>>> {
    let liqee = account_fetcher.fetch_mango_account(pubkey)?;

    let interesting_tcs = liqee.active_token_conditional_swaps().filter_map(|tcs| {
        match tcs_is_interesting(
            &mango_client.context,
            account_fetcher,
            tcs,
            token_swap_info,
            now_ts,
        ) {
            Ok(true) => {
                let volume_result = tcs_max_volume(&liqee, mango_client, account_fetcher, tcs);
                Some(volume_result.map(|v| (*pubkey, tcs.id, v)))
            }
            Ok(false) => None,
            Err(e) => Some(Err(e)),
        }
    });
    Ok(interesting_tcs.collect_vec())
}
