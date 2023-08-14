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

/// When computing the max possible swap for a liqee, assume the price is this fraction worse for them.
///
/// That way when executing the swap, the prices may move this much against the liqee without
/// making the whole execution fail.
const SLIPPAGE_BUFFER: f64 = 0.01; // 1%

/// If a tcs gets limited due to exhausted net borrows, don't trigger execution if
/// the possible value is below this amount. This avoids spamming executions when net
/// borrows are exhausted.
const NET_BORROW_EXECUTION_THRESHOLD: u64 = 1_000_000; // 1 USD

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
    let taker_price = I80F48::from_num(tcs.taker_price(premium_price));

    let max_take_quote = I80F48::from(config.max_trigger_quote_amount);

    let (liqee_max_buy, liqee_max_sell) =
        match tcs_max_liqee_execution(liqee, mango_client, account_fetcher, tcs)? {
            Some(v) => v,
            None => return Ok(false),
        };
    let max_sell_token_to_liqor = liqee_max_sell;

    // In addition to the liqee's requirements, the liqor also has requirements:
    // - only swap while the health ratio stays high enough
    // - possible net borrow limit restrictions from the liqor borrowing the buy token
    // - liqor has a max_take_quote
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
    .min(liqee_max_buy);

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

        let sell_amount = route.in_amount.parse::<f64>()?;
        let buy_amount = route.out_amount.parse::<f64>()?;
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
) -> anyhow::Result<Option<u64>> {
    let buy_bank_pk = mango_client
        .context
        .mint_info(tcs.buy_token_index)
        .first_bank();
    let sell_bank_pk = mango_client
        .context
        .mint_info(tcs.sell_token_index)
        .first_bank();
    let buy_token_price = account_fetcher.fetch_bank_price(&buy_bank_pk)?;
    let sell_token_price = account_fetcher.fetch_bank_price(&sell_bank_pk)?;

    let (max_buy, max_sell) =
        match tcs_max_liqee_execution(account, mango_client, account_fetcher, tcs)? {
            Some(v) => v,
            None => return Ok(None),
        };

    let max_quote =
        (I80F48::from(max_buy) * buy_token_price).min(I80F48::from(max_sell) * sell_token_price);

    Ok(Some(max_quote.floor().clamp_to_u64()))
}

/// Compute the max viable swap for liqee
/// This includes
/// - tcs restrictions (remaining buy/sell, create borrows/deposits)
/// - reduce only banks
/// - net borrow limits on BOTH sides, even though the buy side is technically
///   a liqor limitation: the liqor could acquire the token before trying the
///   execution... but in practice the liqor will work on margin
///
/// Returns Some((native buy amount, native sell amount)) if execution is sensible
/// Returns None if the execution should be skipped (due to net borrow limits...)
fn tcs_max_liqee_execution(
    account: &MangoAccountValue,
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    tcs: &TokenConditionalSwap,
) -> anyhow::Result<Option<(u64, u64)>> {
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

    let base_price = buy_token_price / sell_token_price;
    let premium_price = tcs.premium_price(base_price.to_num());
    let maker_price = tcs.maker_price(premium_price);

    let buy_position = account
        .token_position(tcs.buy_token_index)
        .map(|p| p.native(&buy_bank))
        .unwrap_or(I80F48::ZERO);
    let sell_position = account
        .token_position(tcs.sell_token_index)
        .map(|p| p.native(&sell_bank))
        .unwrap_or(I80F48::ZERO);

    // this is in "buy token received per sell token given" units
    let swap_price = I80F48::from_num((1.0 - SLIPPAGE_BUFFER) / maker_price);
    let max_sell_ignoring_net_borrows = util::max_swap_source_ignore_net_borrows(
        mango_client,
        account_fetcher,
        &account,
        tcs.sell_token_index,
        tcs.buy_token_index,
        swap_price,
        I80F48::ZERO,
    )?
    .floor()
    .to_num::<u64>()
    .min(tcs.max_sell_for_position(sell_position, &sell_bank));

    let max_buy_ignoring_net_borrows = tcs.max_buy_for_position(buy_position, &buy_bank);

    // What follows is a complex manual handling of net borrow limits, for the following reason:
    // Usually, we _do_ want to execute tcs even for small amounts because that will close the
    // tcs order: either due to full execution or due to the health threshold being reached.
    //
    // However, when the net borrow limits are hit, we do _not_ want to close the tcs order
    // even though no further execution is possible at that time. Furthermore, we don't even
    // want to send a too-tiny tcs execution transaction, because there's a good chance we
    // would then be sending lot of those as oracle prices fluctuate.
    //
    // Thus, we need to detect if the possible execution amount is tiny _because_ of the
    // net borrow limits. Then skip. If it's tiny for other reasons we can proceed.

    fn available_borrows(bank: &Bank, price: I80F48) -> u64 {
        if bank.net_borrow_limit_per_window_quote < 0 {
            u64::MAX
        } else {
            let limit = (I80F48::from(bank.net_borrow_limit_per_window_quote) / price)
                .floor()
                .clamp_to_i64();
            (limit - bank.net_borrows_in_window).max(0) as u64
        }
    }
    let available_buy_borrows = available_borrows(&buy_bank, buy_token_price);
    let available_sell_borrows = available_borrows(&sell_bank, sell_token_price);

    // This technically depends on the liqor's buy token position, but we
    // just assume it'll be fully margined here
    let max_buy = max_buy_ignoring_net_borrows.min(available_buy_borrows);

    let sell_borrows = (I80F48::from(max_sell_ignoring_net_borrows) - sell_position).clamp_to_u64();
    let max_sell =
        max_sell_ignoring_net_borrows - sell_borrows + sell_borrows.min(available_sell_borrows);

    let tiny_due_to_net_borrows = {
        let buy_threshold = I80F48::from(NET_BORROW_EXECUTION_THRESHOLD) / buy_token_price;
        let sell_threshold = I80F48::from(NET_BORROW_EXECUTION_THRESHOLD) / sell_token_price;
        max_buy < buy_threshold && max_buy_ignoring_net_borrows > buy_threshold
            || max_sell < sell_threshold && max_sell_ignoring_net_borrows > sell_threshold
    };
    if tiny_due_to_net_borrows {
        return Ok(None);
    }

    Ok(Some((max_buy, max_sell)))
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
                // Filter out Ok(None) resuts of tcs that shouldn't be executed right now
                match tcs_max_volume(&liqee, mango_client, account_fetcher, tcs) {
                    Ok(Some(v)) => Some(Ok((*pubkey, tcs.id, v))),
                    Ok(None) => None,
                    Err(e) => Some(Err(e)),
                }
            }
            Ok(false) => None,
            Err(e) => Some(Err(e)),
        }
    });
    Ok(interesting_tcs.collect_vec())
}
