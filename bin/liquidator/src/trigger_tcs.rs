use std::{
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use futures_core::Future;
use itertools::Itertools;
use mango_v4::{
    i80f48::ClampToInt,
    state::{Bank, MangoAccountValue, TokenConditionalSwap, TokenIndex},
};
use mango_v4_client::{chain_data, health_cache, jupiter, MangoClient, MangoGroupContext};

use solana_sdk::signature::Signature;
use tracing::*;
use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

use crate::{token_swap_info, util, ErrorTracking};

/// When computing the max possible swap for a liqee, assume the price is this fraction worse for them.
///
/// That way when executing the swap, the prices may move this much against the liqee without
/// making the whole execution fail.
const SLIPPAGE_BUFFER: f64 = 0.01; // 1%

/// If a tcs gets limited due to exhausted net borrows, don't trigger execution if
/// the possible value is below this amount. This avoids spamming executions when net
/// borrows are exhausted.
const NET_BORROW_EXECUTION_THRESHOLD: u64 = 1_000_000; // 1 USD

#[derive(Clone)]
pub struct Config {
    pub min_health_ratio: f64,
    pub max_trigger_quote_amount: u64,
    pub refresh_timeout: Duration,
    pub jupiter_version: jupiter::Version,
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
    Ok(tcs.price_in_range(base_price))
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

    let sell_borrows = (I80F48::from(max_sell_ignoring_net_borrows)
        - sell_position.max(I80F48::ZERO))
    .clamp_to_u64();
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

#[derive(Clone)]
struct PreparedExecution {
    pubkey: Pubkey,
    tcs_id: u64,
    volume: u64,
    token_indexes: Vec<TokenIndex>,
    max_buy_token_to_liqee: u64,
    max_sell_token_to_liqor: u64,
}

#[allow(clippy::too_many_arguments)]
async fn prepare_token_conditional_swap(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    token_swap_info: &token_swap_info::TokenSwapInfoUpdater,
    pubkey: &Pubkey,
    tcs_id: u64,
    config: &Config,
) -> anyhow::Result<Option<PreparedExecution>> {
    let now_ts: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        .try_into()?;
    let liqee = account_fetcher.fetch_mango_account(pubkey)?;
    let tcs = liqee.token_conditional_swap_by_id(tcs_id)?.1;

    if tcs.is_expired(now_ts) {
        // Triggering like this will close the expired tcs and not affect the liqor
        Ok(Some(PreparedExecution {
            pubkey: *pubkey,
            tcs_id,
            volume: 0,
            token_indexes: vec![],
            max_buy_token_to_liqee: 0,
            max_sell_token_to_liqor: 0,
        }))
    } else {
        prepare_token_conditional_swap_inner(
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

#[allow(clippy::too_many_arguments)]
async fn prepare_token_conditional_swap_inner(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    token_swap_info: &token_swap_info::TokenSwapInfoUpdater,
    pubkey: &Pubkey,
    liqee_old: &MangoAccountValue,
    tcs_id: u64,
    config: &Config,
    now_ts: u64,
) -> anyhow::Result<Option<PreparedExecution>> {
    let health_cache = health_cache::new(&mango_client.context, account_fetcher, &liqee_old)
        .await
        .context("creating health cache 1")?;
    if health_cache.is_liquidatable() {
        return Ok(None);
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
        return Ok(None);
    }

    let health_cache = health_cache::new(&mango_client.context, account_fetcher, &liqee)
        .await
        .context("creating health cache 2")?;
    if health_cache.is_liquidatable() {
        return Ok(None);
    }

    prepare_token_conditional_swap_inner2(
        mango_client,
        account_fetcher,
        pubkey,
        config,
        &liqee,
        tcs,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, fields(%pubkey, tcs_id = tcs.id))]
async fn prepare_token_conditional_swap_inner2(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    pubkey: &Pubkey,
    config: &Config,
    liqee: &MangoAccountValue,
    tcs: &TokenConditionalSwap,
) -> anyhow::Result<Option<PreparedExecution>> {
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
            None => return Ok(None),
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
        return Ok(None);
    }

    // Final check of the reverse trade on jupiter
    {
        let buy_mint = mango_client.context.mint_info(tcs.buy_token_index).mint;
        let sell_mint = mango_client.context.mint_info(tcs.sell_token_index).mint;
        // The slippage does not matter since we're not going to execute it
        let slippage = 100;
        let input_amount = max_sell_token_to_liqor.min(
            (I80F48::from(max_buy_token_to_liqee) * taker_price)
                .floor()
                .to_num(),
        );
        let route = mango_client
            .jupiter()
            .quote(
                sell_mint,
                buy_mint,
                input_amount,
                slippage,
                false,
                config.jupiter_version,
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
                "skipping because of prices",
            );
            return Ok(None);
        }
    }

    trace!(
        max_buy = max_buy_token_to_liqee,
        max_sell = max_sell_token_to_liqor,
        "prepared execution",
    );

    let volume = (I80F48::from(max_buy_token_to_liqee) * buy_token_price)
        .min(I80F48::from(max_sell_token_to_liqor) * sell_token_price)
        .floor()
        .clamp_to_u64();

    Ok(Some(PreparedExecution {
        pubkey: *pubkey,
        tcs_id: tcs.id,
        volume,
        token_indexes: vec![tcs.buy_token_index, tcs.sell_token_index],
        max_buy_token_to_liqee,
        max_sell_token_to_liqor,
    }))
}

#[derive(Clone)]
pub struct ExecutionContext {
    pub mango_client: Arc<MangoClient>,
    pub account_fetcher: Arc<chain_data::AccountFetcher>,
    pub token_swap_info: Arc<token_swap_info::TokenSwapInfoUpdater>,
    pub config: Config,
}

struct PreparationResult {
    pubkey: Pubkey,
    pending_volume: u64,
    prepared: anyhow::Result<Option<PreparedExecution>>,
}

impl ExecutionContext {
    /// Runs tcs jobs in parallel
    ///
    /// Will run jobs until either the max_trigger_quote_amount is exhausted or
    /// max_completed jobs have been run while respecting the available free token
    /// positions on the liqor.
    ///
    /// It proceeds in two phases:
    /// - Preparation: Evaluates tcs and collects a set of them to trigger.
    ///   The preparation does things like check jupiter for profitability and
    ///   refetching the account to make sure it's up to date.
    /// - Execution: Selects the prepared jobs that fit the liqor's available or free
    ///   token positions.
    ///
    /// Returns a list of transaction signatures as well as the pubkeys of liqees.
    pub async fn execute_tcs(
        &self,
        tcs: &mut Vec<(Pubkey, u64, u64)>,
        error_tracking: &mut ErrorTracking,
    ) -> anyhow::Result<(Vec<Signature>, Vec<Pubkey>)> {
        use rand::distributions::{Distribution, WeightedError, WeightedIndex};
        let now = Instant::now();

        let max_volume = self.config.max_trigger_quote_amount;
        let mut pending_volume = 0;
        let mut prepared_volume = 0;

        let max_prepared = 32;
        let mut prepared_executions = vec![];

        let mut pending = vec![];
        let mut no_new_job = false;

        // What goes on below is roughly the following:
        //
        // We have a bunch of tcs we want to try executing in `tcs`.
        // We pick a random ones (weighted by volume) and collect their `pending` jobs.
        // Once the maximum number of prepared jobs (`max_prepared`) or `max_volume`
        // for this run is reached, we wait for one of the jobs to finish.
        // This will either free up the job slot and volume or commit it.
        // If it freed up a slot, another job can be added to `pending`
        // If `no_new_job` can be added to `pending`, we also start waiting for completion.
        while prepared_executions.len() < max_prepared && prepared_volume < max_volume {
            // If it's impossible to start another job right now, we need to wait
            // for one to complete (or we're done)
            if prepared_executions.len() + pending.len() >= max_prepared
                || prepared_volume + pending_volume >= max_volume
                || no_new_job
            {
                if pending.is_empty() {
                    break;
                }

                // select_all to run until one completes
                let (result, _index, remaining): (PreparationResult, _, _) =
                    futures::future::select_all(pending).await;
                pending = remaining;
                pending_volume -= result.pending_volume;
                match result.prepared {
                    Ok(Some(prepared)) => {
                        prepared_volume += prepared.volume;
                        prepared_executions.push(prepared);
                    }
                    Ok(None) => {
                        // maybe the tcs isn't executable after the account was updated
                    }
                    Err(e) => {
                        error_tracking.record_error(&result.pubkey, now, e.to_string());
                    }
                }
                no_new_job = false;
                continue;
            }

            // Pick a random tcs with volume that would still fit the limit
            let available_volume = max_volume - pending_volume - prepared_volume;
            let (pubkey, tcs_id, volume) = {
                let weights = tcs.iter().map(|(_, _, volume)| {
                    if *volume == u64::MAX {
                        // entries marked like this have been processed already
                        return 0;
                    }
                    let volume = (*volume).min(max_volume).max(1);
                    if volume <= available_volume {
                        volume
                    } else {
                        0
                    }
                });
                let dist_result = WeightedIndex::new(weights);
                if let Err(WeightedError::AllWeightsZero) = dist_result {
                    // If there's no fitting new job, complete one of the pending
                    // ones to check if that frees up some volume allowance
                    no_new_job = true;
                    continue;
                }
                let dist = dist_result.unwrap();

                let mut rng = rand::thread_rng();
                let sample = dist.sample(&mut rng);
                let (pubkey, tcs_id, volume) = &mut tcs[sample];
                let volume_copy = *volume;
                *volume = u64::MAX; // don't run this one again
                (*pubkey, *tcs_id, volume_copy)
            };

            // start the new one
            if let Some(job) = self.prepare_job(&pubkey, tcs_id, volume, error_tracking) {
                pending_volume += volume;
                pending.push(job);
            }
        }

        // We have now prepared a list of tcs we want to execute in `prepared_jobs`.
        // The complication is that they will alter the liqor and we need to  make sure to send
        // health accounts that will work independently of the order of these tx hitting the chain.

        let mut liqor = self.mango_client.mango_account().await?;
        let allowed_tokens = prepared_executions
            .iter()
            .map(|v| &v.token_indexes)
            .flatten()
            .copied()
            .unique()
            .filter(|&idx| liqor.ensure_token_position(idx).is_ok())
            .collect_vec();

        // Create futures for all the executions that use only allowed tokens
        let jobs = prepared_executions
            .into_iter()
            .filter(|v| {
                v.token_indexes
                    .iter()
                    .all(|token| allowed_tokens.contains(token))
            })
            .map(|v| self.start_prepared_job(v, allowed_tokens.clone()));

        // Execute everything
        let results = futures::future::join_all(jobs).await;
        let successes = results
            .into_iter()
            .filter_map(|(pubkey, result)| match result {
                Ok(v) => Some((pubkey, v)),
                Err(err) => {
                    error_tracking.record_error(&pubkey, Instant::now(), err.to_string());
                    None
                }
            });

        let (completed_pubkeys, completed_txs) = successes.unzip();
        Ok((completed_txs, completed_pubkeys))
    }

    // Maybe returns a future that might return a PreparedExecution
    fn prepare_job(
        &self,
        pubkey: &Pubkey,
        tcs_id: u64,
        volume: u64,
        error_tracking: &ErrorTracking,
    ) -> Option<Pin<Box<dyn Future<Output = PreparationResult> + Send>>> {
        // Skip a pubkey if there've been too many errors recently
        if let Some(error_entry) = error_tracking.had_too_many_errors(pubkey, Instant::now()) {
            trace!(
                "skip checking for tcs on account {pubkey}, had {} errors recently",
                error_entry.count
            );
            return None;
        }

        let context = self.clone();
        let pubkey = pubkey.clone();
        let job = async move {
            PreparationResult {
                pubkey,
                pending_volume: volume,
                prepared: prepare_token_conditional_swap(
                    &context.mango_client,
                    &context.account_fetcher,
                    &context.token_swap_info,
                    &pubkey,
                    tcs_id,
                    &context.config,
                )
                .await,
            }
        };
        Some(Box::pin(job))
    }

    async fn start_prepared_job(
        &self,
        pending: PreparedExecution,
        allowed_tokens: Vec<TokenIndex>,
    ) -> (Pubkey, anyhow::Result<Signature>) {
        (
            pending.pubkey,
            self.start_prepared_job_inner(pending, allowed_tokens).await,
        )
    }

    async fn start_prepared_job_inner(
        &self,
        pending: PreparedExecution,
        allowed_tokens: Vec<TokenIndex>,
    ) -> anyhow::Result<Signature> {
        let liqee = self.account_fetcher.fetch_mango_account(&pending.pubkey)?;
        let compute_ix =
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                self.config.compute_limit_for_trigger,
            );
        let trigger_ix = self
            .mango_client
            .token_conditional_swap_trigger_instruction(
                (&pending.pubkey, &liqee),
                pending.tcs_id,
                pending.max_buy_token_to_liqee,
                pending.max_sell_token_to_liqor,
                &allowed_tokens,
            )
            .await?;
        let txsig = self
            .mango_client
            .send_and_confirm_owner_tx(vec![compute_ix, trigger_ix])
            .await?;
        info!(
            pubkey = %pending.pubkey,
            tcs_id = pending.tcs_id,
            %txsig,
            "executed token conditional swap",
        );
        Ok(txsig)
    }
}
