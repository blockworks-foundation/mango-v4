use std::collections::HashSet;
use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, RwLock},
    time::Instant,
};

use futures_core::Future;
use itertools::Itertools;
use mango_v4::{
    i80f48::ClampToInt,
    state::{Bank, MangoAccountValue, TokenConditionalSwap, TokenIndex},
};
use mango_v4_client::{chain_data, jupiter, MangoClient, TransactionBuilder};

use anyhow::Context as AnyhowContext;
use solana_sdk::signature::Signature;
use tracing::*;
use {fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

use crate::{token_swap_info, util, ErrorTracking, LiqErrorType};

/// When computing the max possible swap for a liqee, assume the price is this fraction worse for them.
///
/// That way when executing the swap, the prices may move this much against the liqee without
/// making the whole execution fail.
const SLIPPAGE_BUFFER: f64 = 0.01; // 1%

/// If a tcs gets limited due to exhausted net borrows or deposit limits, don't trigger execution if
/// the possible value is below this amount. This avoids spamming executions when limits are exhausted.
const EXECUTION_THRESHOLD: u64 = 1_000_000; // 1 USD

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Directly execute the trigger, borrowing any buy tokens needed. Normal rebalancing will
    /// resolve deposits and withdraws created by trigger execution
    BorrowBuyToken,

    /// Do a jupiter swap in the same tx as the trigger, possibly creating a buy token deposit
    /// and a sell token borrow. This can create a temporary sell token borrow that gets
    /// mostly closed by the trigger execution.
    ///
    /// This is the nicest mode because it leaves the smallest buy/sell token balances, the
    /// trigger execution is almost single transaction arbitrage.
    ///
    /// If sell token borrows are impossible (reduce only, borrow limits), this
    /// will back back on SwapCollateralIntoBuy.
    SwapSellIntoBuy,

    /// Do a jupiter swap in the same tx as the trigger, buying the buy token for the
    /// collateral token. This way the liquidator won't need to borrow tokens.
    SwapCollateralIntoBuy,
}

#[derive(Clone)]
pub struct Config {
    pub min_health_ratio: f64,
    pub max_trigger_quote_amount: u64,
    pub compute_limit_for_trigger: u32,
    pub collateral_token_index: TokenIndex,

    /// At 0, the liquidator would trigger tcs if the cost to the liquidator is the
    /// same as the cost to the liqee. 0.1 would mean a 10% better price to the liquidator.
    pub profit_fraction: f64,

    /// Minimum fraction of max_buy to buy for success when triggering,
    /// useful in conjunction with jupiter swaps in same tx to avoid over-buying.
    ///
    /// Can be set to 0 to allow executions of any size.
    pub min_buy_fraction: f64,

    pub jupiter_version: jupiter::Version,
    pub jupiter_slippage_bps: u64,
    pub mode: Mode,

    pub only_allowed_tokens: HashSet<TokenIndex>,
    pub forbidden_tokens: HashSet<TokenIndex>,
}

pub enum JupiterQuoteCacheResult<T> {
    Quote(T),
    BadPrice(f64),
}

#[derive(Clone)]
pub struct JupiterQuoteCacheEntry {
    // The lowest seen price (starting at f64::MAX) in input-per-output tokens
    //
    // A separate mutex is necessary since we want to wait for the first jupiter quote for
    // each pair to return before initiating any further ones. Later on there can be multiple
    // jupiter quotes for a pair at the same time if the initial price check passes.
    price: Arc<tokio::sync::Mutex<f64>>,
}

// While preparing tcs executions, there may be a lot of jupiter queries for the same
// pairs. This caches results to avoid hitting jupiter with too many requests by allowing
// a cheap-early out.
#[derive(Default)]
pub struct JupiterQuoteCache {
    // cache lowest price for each in-out mint pair
    pub quote_cache: RwLock<HashMap<(Pubkey, Pubkey), JupiterQuoteCacheEntry>>,
}

impl JupiterQuoteCache {
    fn cache_entry(&self, input_mint: Pubkey, output_mint: Pubkey) -> JupiterQuoteCacheEntry {
        let mut quote_cache = self.quote_cache.write().unwrap();
        quote_cache
            .entry((input_mint, output_mint))
            .or_insert_with(|| JupiterQuoteCacheEntry {
                price: Arc::new(tokio::sync::Mutex::new(f64::MAX)),
            })
            .clone()
    }

    /// Quotes. Returns BadPrice if the cache or quote returns a price above max_in_per_out_price.
    pub async fn quote(
        &self,
        client: &MangoClient,
        input_mint: Pubkey,
        output_mint: Pubkey,
        input_amount: u64,
        slippage_bps: u64,
        version: jupiter::Version,
        max_in_per_out_price: f64,
    ) -> anyhow::Result<JupiterQuoteCacheResult<(f64, jupiter::Quote)>> {
        let cache_entry = self.cache_entry(input_mint, output_mint);

        let held_lock = {
            let cached_price_lock = cache_entry.price.lock().await;

            if *cached_price_lock == f64::MAX {
                // If we're the first quote for this pair, run the quote while holding the lock:
                // we don't want multiple parallel requests to go out that will all potentially
                // tell us about the same poor price.
                Some(cached_price_lock)
            } else {
                // If a cached price exists, check it against the max
                if *cached_price_lock > max_in_per_out_price {
                    return Ok(JupiterQuoteCacheResult::BadPrice(*cached_price_lock));
                }

                // Don't hold the lock, parallel requests are ok!
                None
            }
        };

        let (price, quote) = self
            .quote_inner(
                client,
                input_mint,
                output_mint,
                input_amount,
                slippage_bps,
                version,
            )
            .await?;

        {
            let mut cached_price_lock = if let Some(lock) = held_lock {
                lock
            } else {
                cache_entry.price.lock().await
            };
            if price < *cached_price_lock {
                *cached_price_lock = price;
            }
        }

        Ok(if price > max_in_per_out_price {
            JupiterQuoteCacheResult::BadPrice(price)
        } else {
            JupiterQuoteCacheResult::Quote((price, quote))
        })
    }

    async fn quote_inner(
        &self,
        client: &MangoClient,
        input_mint: Pubkey,
        output_mint: Pubkey,
        input_amount: u64,
        slippage_bps: u64,
        version: jupiter::Version,
    ) -> anyhow::Result<(f64, jupiter::Quote)> {
        let quote = client
            .jupiter()
            .quote(
                input_mint,
                output_mint,
                input_amount,
                slippage_bps,
                false,
                version,
            )
            .await?;
        let quote_price = quote.in_amount as f64 / quote.out_amount as f64;
        Ok((quote_price, quote))
    }

    async fn unchecked_quote(
        &self,
        client: &MangoClient,
        input_mint: Pubkey,
        output_mint: Pubkey,
        input_amount: u64,
        slippage_bps: u64,
        version: jupiter::Version,
    ) -> anyhow::Result<(f64, jupiter::Quote)> {
        match self
            .quote(
                client,
                input_mint,
                output_mint,
                input_amount,
                slippage_bps,
                version,
                f64::MAX,
            )
            .await?
        {
            JupiterQuoteCacheResult::Quote(v) => Ok(v),
            _ => anyhow::bail!("unreachable case in unchecked_quote"),
        }
    }

    async fn cached_price(&self, input_mint: Pubkey, output_mint: Pubkey) -> Option<f64> {
        if input_mint == output_mint {
            return Some(1.0);
        }

        let cache_entry = self.cache_entry(input_mint, output_mint);
        let cached_price = *cache_entry.price.lock().await;
        if cached_price != f64::MAX {
            Some(cached_price)
        } else {
            None
        }
    }

    /// Quotes collateral -> buy and sell -> collateral swaps
    ///
    /// Returns BadPrice if the full route (cached or queried) exceeds the max_sell_per_buy_price.
    ///
    /// Returns Quote((sell_per_buy price, collateral->buy quote, sell->collateral quote)) on success
    pub async fn quote_collateral_swap(
        &self,
        client: &MangoClient,
        collateral_mint: Pubkey,
        buy_mint: Pubkey,
        sell_mint: Pubkey,
        collateral_amount: u64,
        sell_amount: u64,
        slippage_bps: u64,
        version: jupiter::Version,
        max_sell_per_buy_price: f64,
    ) -> anyhow::Result<
        JupiterQuoteCacheResult<(f64, Option<jupiter::Quote>, Option<jupiter::Quote>)>,
    > {
        // First check if we have cached prices for both legs and
        // if those break the specified limit
        let cached_collateral_to_buy = self.cached_price(collateral_mint, buy_mint).await;
        let cached_sell_to_collateral = self.cached_price(sell_mint, collateral_mint).await;
        if let (Some(c_to_b), Some(s_to_c)) = (cached_collateral_to_buy, cached_sell_to_collateral)
        {
            let s_to_b = s_to_c * c_to_b;
            if s_to_b > max_sell_per_buy_price {
                return Ok(JupiterQuoteCacheResult::BadPrice(s_to_b));
            }
        }

        // Get fresh quotes
        let collateral_to_buy_quote;
        let collateral_per_buy_price;
        if collateral_mint != buy_mint {
            let (buy_price, buy_quote) = self
                .unchecked_quote(
                    client,
                    collateral_mint,
                    buy_mint,
                    collateral_amount,
                    slippage_bps,
                    version,
                )
                .await?;

            collateral_per_buy_price = buy_price;
            collateral_to_buy_quote = Some(buy_quote);
        } else {
            collateral_per_buy_price = 1.0;
            collateral_to_buy_quote = None;
        }

        let sell_to_collateral_quote;
        let sell_per_collateral_price;
        if collateral_mint != sell_mint {
            let (sell_price, sell_quote) = self
                .unchecked_quote(
                    client,
                    sell_mint,
                    collateral_mint,
                    sell_amount,
                    slippage_bps,
                    version,
                )
                .await?;
            sell_per_collateral_price = sell_price;
            sell_to_collateral_quote = Some(sell_quote);
        } else {
            sell_per_collateral_price = 1.0;
            sell_to_collateral_quote = None;
        }

        // Check price limit on the new quotes
        let price = sell_per_collateral_price * collateral_per_buy_price;
        if price > max_sell_per_buy_price {
            return Ok(JupiterQuoteCacheResult::BadPrice(price));
        }

        Ok(JupiterQuoteCacheResult::Quote((
            price,
            collateral_to_buy_quote,
            sell_to_collateral_quote,
        )))
    }
}

#[derive(Clone)]
struct PreparedExecution {
    pubkey: Pubkey,
    tcs_id: u64,
    volume: u64,
    token_indexes: Vec<TokenIndex>,
    max_buy_token_to_liqee: u64,
    max_sell_token_to_liqor: u64,
    min_buy_token: u64,
    min_taker_price: f32,
    jupiter_quote: Option<jupiter::Quote>,
}

struct PreparationResult {
    pubkey: Pubkey,
    pending_volume: u64,
    prepared: anyhow::Result<Option<PreparedExecution>>,
}

#[derive(Clone)]
pub struct Context {
    pub mango_client: Arc<MangoClient>,
    pub account_fetcher: Arc<chain_data::AccountFetcher>,

    /// Information about current token prices is used to reject tcs early
    /// that are very likely to not be executable profitably.
    pub token_swap_info: Arc<token_swap_info::TokenSwapInfoUpdater>,

    /// Cache jupiter prices. Sometimes a lot of tcs look potentially interesting at the same time.
    /// To avoid spamming the jupiter API for each one, cache the returned prices and don't query again
    /// if we are likely to get a result that would lead to us not wanting to trigger the tcs.
    pub jupiter_quote_cache: Arc<JupiterQuoteCache>,

    pub config: Config,
    pub now_ts: u64,
}

impl Context {
    fn token_bank_price_mint(
        &self,
        token_index: TokenIndex,
    ) -> anyhow::Result<(Bank, I80F48, Pubkey)> {
        let info = self.mango_client.context.token(token_index);
        let (bank, price) = self
            .account_fetcher
            .fetch_bank_and_price(&info.first_bank())?;
        Ok((bank, price, info.mint))
    }

    fn tcs_has_plausible_price(
        &self,
        tcs: &TokenConditionalSwap,
        base_price: f64,
    ) -> anyhow::Result<bool> {
        // The premium the taker receives needs to take taker fees into account
        let taker_price = tcs.taker_price(tcs.premium_price(base_price, self.now_ts));

        // Never take tcs where the fee exceeds the premium and the triggerer exchanges
        // tokens at below oracle price.
        if taker_price < base_price {
            return Ok(false);
        }

        let buy_info = self
            .token_swap_info
            .swap_info(tcs.buy_token_index)
            .ok_or_else(|| anyhow::anyhow!("no swap info for token {}", tcs.buy_token_index))?;
        let sell_info = self
            .token_swap_info
            .swap_info(tcs.sell_token_index)
            .ok_or_else(|| anyhow::anyhow!("no swap info for token {}", tcs.sell_token_index))?;

        // If this is 1.0 then the exchange can (probably) happen at oracle price.
        // 1.5 would mean we need to pay 50% more than oracle etc.
        let cost_over_oracle = buy_info.buy_over_oracle() * sell_info.sell_over_oracle();

        Ok(taker_price >= base_price * cost_over_oracle * (1.0 + self.config.profit_fraction))
    }

    // excluded by config
    fn tcs_pair_is_allowed(
        &self,
        buy_token_index: TokenIndex,
        sell_token_index: TokenIndex,
    ) -> bool {
        if self.config.forbidden_tokens.contains(&buy_token_index) {
            return false;
        }

        if self.config.forbidden_tokens.contains(&sell_token_index) {
            return false;
        }

        if self.config.only_allowed_tokens.is_empty() {
            return true;
        }

        if self.config.only_allowed_tokens.contains(&buy_token_index) {
            return true;
        }

        if self.config.only_allowed_tokens.contains(&sell_token_index) {
            return true;
        }

        return false;
    }

    // Either expired or triggerable with ok-looking price.
    fn tcs_is_interesting(&self, tcs: &TokenConditionalSwap) -> anyhow::Result<bool> {
        if tcs.is_expired(self.now_ts) {
            return Ok(true);
        }
        if !self.tcs_pair_is_allowed(tcs.buy_token_index, tcs.buy_token_index) {
            return Ok(false);
        }

        let (_, buy_token_price, _) = self.token_bank_price_mint(tcs.buy_token_index)?;
        let (_, sell_token_price, _) = self.token_bank_price_mint(tcs.sell_token_index)?;
        let base_price = (buy_token_price / sell_token_price).to_num();

        Ok(tcs.is_triggerable(base_price, self.now_ts)
            && self.tcs_has_plausible_price(tcs, base_price)?)
    }

    /// Returns the maximum execution size of a tcs order in quote units
    pub fn tcs_max_volume(
        &self,
        account: &MangoAccountValue,
        tcs: &TokenConditionalSwap,
    ) -> anyhow::Result<Option<u64>> {
        let (_, buy_token_price, _) = self.token_bank_price_mint(tcs.buy_token_index)?;
        let (_, sell_token_price, _) = self.token_bank_price_mint(tcs.sell_token_index)?;

        let (max_buy, max_sell) = match self.tcs_max_liqee_execution(account, tcs)? {
            Some(v) => v,
            None => return Ok(None),
        };

        let max_quote = (I80F48::from(max_buy) * buy_token_price)
            .min(I80F48::from(max_sell) * sell_token_price);

        Ok(Some(max_quote.floor().clamp_to_u64()))
    }

    /// Compute the max viable swap for liqee
    /// This includes
    /// - tcs restrictions (remaining buy/sell, create borrows/deposits)
    /// - reduce only banks
    /// - net borrow limits:
    ///   - the account may borrow the sell token (and the liqor side may not be a repay)
    ///   - the liqor may borrow the buy token (and the account side may not be a repay)
    ///     this is technically a liqor limitation: the liqor could acquire the token before trying the
    ///     execution... but in practice the liqor may work on margin
    /// - deposit limits:
    ///   - the account may deposit the buy token (while the liqor borrowed it)
    ///   - the liqor may deposit the sell token (while the account borrowed it)
    ///
    /// Returns Some((native buy amount, native sell amount)) if execution is sensible
    /// Returns None if the execution should be skipped (due to limits)
    pub fn tcs_max_liqee_execution(
        &self,
        account: &MangoAccountValue,
        tcs: &TokenConditionalSwap,
    ) -> anyhow::Result<Option<(u64, u64)>> {
        let (buy_bank, buy_token_price, _) = self.token_bank_price_mint(tcs.buy_token_index)?;
        let (sell_bank, sell_token_price, _) = self.token_bank_price_mint(tcs.sell_token_index)?;

        let base_price = buy_token_price / sell_token_price;
        let premium_price = tcs.premium_price(base_price.to_num(), self.now_ts);
        let maker_price = tcs.maker_price(premium_price);

        let liqee_buy_position = account
            .token_position(tcs.buy_token_index)
            .map(|p| p.native(&buy_bank))
            .unwrap_or(I80F48::ZERO);
        let liqee_sell_position = account
            .token_position(tcs.sell_token_index)
            .map(|p| p.native(&sell_bank))
            .unwrap_or(I80F48::ZERO);

        // this is in "buy token received per sell token given" units
        let swap_price = I80F48::from_num((1.0 - SLIPPAGE_BUFFER) / maker_price);
        let max_sell_ignoring_limits = util::max_swap_source_ignoring_limits(
            &self.mango_client,
            &self.account_fetcher,
            account,
            tcs.sell_token_index,
            tcs.buy_token_index,
            swap_price,
            I80F48::ZERO,
        )?
        .floor()
        .to_num::<u64>()
        .min(tcs.max_sell_for_position(liqee_sell_position, &sell_bank));

        let max_buy_ignoring_limits = tcs.max_buy_for_position(liqee_buy_position, &buy_bank);

        // What follows is a complex manual handling of net borrow/deposit limits, for
        // the following reason:
        // Usually, we want to execute tcs even for small amounts because that will close the
        // tcs order: either due to full execution or due to the health threshold being reached.
        //
        // However, when the limits are hit, it will not closed when no further execution
        // is possible, because limit issues are transient. Furthermore, we don't want to send
        // tiny tcs trigger transactions, because there's a good chance we would then be sending
        // lot of those as oracle prices fluctuate.
        //
        // Thus, we need to detect if the possible execution amount is tiny _because_ of the
        // limits. Then skip. If it's tiny for other reasons we can proceed.

        // Do the liqor buy tokens come from deposits or are they borrowed?
        let mut liqor_buy_borrows = match self.config.mode {
            Mode::BorrowBuyToken => {
                // Assume that the liqor has enough buy token if it's collateral
                if tcs.buy_token_index == self.config.collateral_token_index {
                    0
                } else {
                    max_buy_ignoring_limits
                }
            }
            Mode::SwapCollateralIntoBuy { .. } => 0,
            Mode::SwapSellIntoBuy { .. } => {
                // Never needs buy borrows.
                // This might need extra sell borrows, but falls back onto SwapCollateralIntoBuy if needed
                0
            }
        };

        // First, net borrow limits
        let max_sell_net_borrows;
        let max_buy_net_borrows;
        {
            fn available_borrows(bank: &Bank, price: I80F48) -> u64 {
                bank.remaining_net_borrows_quote(price)
                    .saturating_div(price)
                    .clamp_to_u64()
            }
            let available_buy_borrows = available_borrows(&buy_bank, buy_token_price);
            let available_sell_borrows = available_borrows(&sell_bank, sell_token_price);

            // New borrows if max_sell_ignoring_limits was withdrawn on the liqee
            // We assume that on the liqor side the position is >= 0, so these are true
            // new borrows.
            let sell_borrows = (I80F48::from(max_sell_ignoring_limits)
                - liqee_sell_position.max(I80F48::ZERO))
            .ceil()
            .clamp_to_u64();

            // On the buy side, the liqor might need to borrow, see liqor_buy_borrows.
            // On the liqee side, the bought tokens may repay a borrow, reducing net borrows again
            let buy_borrows = (I80F48::from(liqor_buy_borrows)
                + liqee_buy_position.min(I80F48::ZERO))
            .ceil()
            .clamp_to_u64();

            // New maximums adjusted for net borrow limits
            max_sell_net_borrows = max_sell_ignoring_limits
                - (sell_borrows - sell_borrows.min(available_sell_borrows));
            max_buy_net_borrows =
                max_buy_ignoring_limits - (buy_borrows - buy_borrows.min(available_buy_borrows));
            liqor_buy_borrows = liqor_buy_borrows.min(max_buy_net_borrows);
        }

        // Second, deposit limits
        let max_sell;
        let max_buy;
        {
            let available_buy_deposits = buy_bank.remaining_deposits_until_limit().clamp_to_u64();
            let available_sell_deposits = sell_bank.remaining_deposits_until_limit().clamp_to_u64();

            // New deposits on the liqee side (reduced by repaid borrows)
            let liqee_buy_deposits = (I80F48::from(max_buy_net_borrows)
                + liqee_buy_position.min(I80F48::ZERO))
            .ceil()
            .clamp_to_u64();
            // the net new deposits can only be as big as the liqor borrows
            // (assume no borrows, then deposits only move from liqor to liqee)
            let buy_deposits = liqee_buy_deposits.min(liqor_buy_borrows);

            // We assume the liqor position is always >= 0, meaning there are new sell token deposits if
            // the sell token gets borrowed on the liqee side.
            let sell_deposits = (I80F48::from(max_sell_net_borrows)
                - liqee_sell_position.max(I80F48::ZERO))
            .ceil()
            .clamp_to_u64();

            max_sell =
                max_sell_net_borrows - (sell_deposits - sell_deposits.min(available_sell_deposits));
            max_buy =
                max_buy_net_borrows - (buy_deposits - buy_deposits.min(available_buy_deposits));
        }

        let tiny_due_to_limits = {
            let buy_threshold = I80F48::from(EXECUTION_THRESHOLD) / buy_token_price;
            let sell_threshold = I80F48::from(EXECUTION_THRESHOLD) / sell_token_price;
            max_buy < buy_threshold && max_buy_ignoring_limits > buy_threshold
                || max_sell < sell_threshold && max_sell_ignoring_limits > sell_threshold
        };
        if tiny_due_to_limits {
            return Ok(None);
        }

        Ok(Some((max_buy, max_sell)))
    }

    pub fn find_interesting_tcs_for_account(
        &self,
        pubkey: &Pubkey,
    ) -> anyhow::Result<Vec<anyhow::Result<(Pubkey, u64, u64)>>> {
        let liqee = self.account_fetcher.fetch_mango_account(pubkey)?;

        let interesting_tcs = liqee
            .active_token_conditional_swaps()
            .filter_map(|tcs| {
                match self.tcs_is_interesting(tcs) {
                    Ok(true) => {
                        // Filter out Ok(None) resuts of tcs that shouldn't be executed right now
                        match self.tcs_max_volume(&liqee, tcs) {
                            Ok(Some(v)) => Some(Ok((*pubkey, tcs.id, v))),
                            Ok(None) => None,
                            Err(e) => Some(Err(e)),
                        }
                    }
                    Ok(false) => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .collect_vec();
        if !interesting_tcs.is_empty() {
            trace!(%pubkey, interesting_tcs_count=interesting_tcs.len(), "found interesting tcs");
        }
        Ok(interesting_tcs)
    }

    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all, fields(%pubkey, %tcs_id))]
    async fn prepare_token_conditional_swap(
        &self,
        pubkey: &Pubkey,
        tcs_id: u64,
    ) -> anyhow::Result<Option<PreparedExecution>> {
        let now_ts: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let liqee = self.account_fetcher.fetch_mango_account(pubkey)?;
        let tcs = liqee.token_conditional_swap_by_id(tcs_id)?.1;

        if tcs.is_expired(now_ts) {
            trace!("tcs is expired");
            // Triggering like this will close the expired tcs and not affect the liqor
            Ok(Some(PreparedExecution {
                pubkey: *pubkey,
                tcs_id,
                volume: 0,
                token_indexes: vec![],
                max_buy_token_to_liqee: 0,
                max_sell_token_to_liqor: 0,
                min_buy_token: 0,
                min_taker_price: 0.0,
                jupiter_quote: None,
            }))
        } else {
            self.prepare_token_conditional_swap_inner(pubkey, &liqee, tcs.id)
                .await
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn prepare_token_conditional_swap_inner(
        &self,
        pubkey: &Pubkey,
        liqee_old: &MangoAccountValue,
        tcs_id: u64,
    ) -> anyhow::Result<Option<PreparedExecution>> {
        let health_cache = self
            .mango_client
            .health_cache(liqee_old)
            .await
            .context("creating health cache 1")?;
        if health_cache.is_liquidatable() {
            trace!("account is liquidatable (pre-fetch)");
            return Ok(None);
        }

        // get a fresh account and re-check the tcs and health
        let liqee = self
            .account_fetcher
            .fetch_fresh_mango_account(pubkey)
            .await?;
        let (_, tcs) = liqee.token_conditional_swap_by_id(tcs_id)?;
        if tcs.is_expired(self.now_ts) || !self.tcs_is_interesting(tcs)? {
            trace!("tcs is expired or uninteresting");
            return Ok(None);
        }

        let health_cache = self
            .mango_client
            .health_cache(&liqee)
            .await
            .context("creating health cache 2")?;
        if health_cache.is_liquidatable() {
            trace!("account is liquidatable (post-fetch)");
            return Ok(None);
        }

        self.prepare_token_conditional_swap_inner2(pubkey, &liqee, tcs)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn prepare_token_conditional_swap_inner2(
        &self,
        pubkey: &Pubkey,
        liqee: &MangoAccountValue,
        tcs: &TokenConditionalSwap,
    ) -> anyhow::Result<Option<PreparedExecution>> {
        let liqor_min_health_ratio = I80F48::from_num(self.config.min_health_ratio);

        // Compute the max viable swap (for liqor and liqee) and min it
        let (buy_bank, buy_token_price, buy_mint) =
            self.token_bank_price_mint(tcs.buy_token_index)?;
        let (sell_bank, sell_token_price, sell_mint) =
            self.token_bank_price_mint(tcs.sell_token_index)?;

        let base_price = buy_token_price / sell_token_price;
        let premium_price = tcs.premium_price(base_price.to_num(), self.now_ts);
        let taker_price = I80F48::from_num(tcs.taker_price(premium_price));

        let max_take_quote = I80F48::from(self.config.max_trigger_quote_amount);

        let (liqee_max_buy, liqee_max_sell) = match self.tcs_max_liqee_execution(liqee, tcs)? {
            Some(v) => v,
            None => {
                trace!("no liqee execution possible");
                return Ok(None);
            }
        };
        let max_sell_token_to_liqor = liqee_max_sell;

        // In addition to the liqee's requirements, the liqor also has requirement,
        // because it might not have enough buy token:
        // - if taking by borrowing buy token or swapping sell into buy:
        //   - respect the min_health_ratio
        //   - possible net borrow limit restrictions from the liqor borrowing the token
        // - if taking by buying buy token with collateral:
        //   - limit by current current collateral
        // - liqor has a defined max_take_quote

        // Mode fallback?
        let mode = match self.config.mode.clone() {
            Mode::SwapSellIntoBuy => {
                // This mode falls back on another when sell token borrows are impossible
                // or too limited
                let available_quote = sell_bank.remaining_net_borrows_quote(sell_token_price);
                // Note that needed_quote does not account for an existing sell token balance:
                // That is fine - if sell token is the collateral, we can just use the collateral
                // based mode and the result will be the same.
                let needed_quote = (I80F48::from(max_sell_token_to_liqor) * sell_token_price)
                    .min(I80F48::from(liqee_max_buy) * buy_token_price)
                    .min(max_take_quote);
                if sell_bank.are_borrows_reduce_only() || needed_quote > available_quote {
                    Mode::SwapCollateralIntoBuy
                } else {
                    Mode::SwapSellIntoBuy
                }
            }
            m => m,
        };

        let jupiter_slippage_fraction = 1.0 - self.config.jupiter_slippage_bps as f64 * 0.0001;

        let mut liqor = self.mango_client.mango_account().await?;

        let collateral_token_index = self.config.collateral_token_index;
        let liqor_existing_buy_token = liqor
            .ensure_token_position(tcs.buy_token_index)?
            .0
            .native(&buy_bank);
        let liqor_available_buy_token = match mode {
            Mode::BorrowBuyToken => util::max_swap_source_with_limits(
                &self.mango_client,
                &self.account_fetcher,
                &liqor,
                tcs.buy_token_index,
                tcs.sell_token_index,
                taker_price,
                liqor_min_health_ratio,
            )?,
            Mode::SwapCollateralIntoBuy => {
                // The transaction will be net-positive, but how much buy token can
                // the collateral be swapped into?
                if tcs.buy_token_index == collateral_token_index {
                    liqor_existing_buy_token
                } else {
                    let (_, collateral_price, _) =
                        self.token_bank_price_mint(collateral_token_index)?;
                    let buy_per_collateral_price = (collateral_price / buy_token_price)
                        * I80F48::from_num(jupiter_slippage_fraction);
                    let collateral_amount = util::max_swap_source_with_limits(
                        &self.mango_client,
                        &self.account_fetcher,
                        &liqor,
                        collateral_token_index,
                        tcs.buy_token_index,
                        buy_per_collateral_price,
                        liqor_min_health_ratio,
                    )?;

                    collateral_amount * buy_per_collateral_price
                }
            }
            Mode::SwapSellIntoBuy => {
                // How big can the sell -> buy swap be?
                let buy_per_sell_price =
                    (I80F48::from(1) / taker_price) * I80F48::from_num(jupiter_slippage_fraction);
                let max_sell = util::max_swap_source_with_limits(
                    &self.mango_client,
                    &self.account_fetcher,
                    &liqor,
                    tcs.sell_token_index,
                    tcs.buy_token_index,
                    buy_per_sell_price,
                    liqor_min_health_ratio,
                )?;
                max_sell * buy_per_sell_price
            }
        };
        let max_buy_token_to_liqee = liqor_available_buy_token
            .min(max_take_quote / buy_token_price)
            .clamp_to_u64()
            .min(liqee_max_buy);

        if max_sell_token_to_liqor == 0 || max_buy_token_to_liqee == 0 {
            trace!(
                liqee_max_buy,
                liqee_max_sell,
                max_buy = max_buy_token_to_liqee,
                max_sell = max_sell_token_to_liqor,
                "no execution possible"
            );
            return Ok(None);
        }

        // The quote amount the swap could be at
        let volume = (I80F48::from(max_buy_token_to_liqee) * buy_token_price)
            .min(I80F48::from(max_sell_token_to_liqor) * sell_token_price);

        // Final check of the reverse trade on jupiter
        //
        // We want swap_at_taker_price * counterswap >= 1 + profit_fraction
        // so 1/counterswap <= swap_at_taker_price / (1 + profit_fraction)
        let taker_price_profit = taker_price.to_num::<f64>() / (1.0 + self.config.profit_fraction);
        let jupiter_quote;
        let swap_price;
        let mut bad_price = false;
        match mode {
            Mode::BorrowBuyToken | Mode::SwapSellIntoBuy => {
                // Even if we borrow, we want to check that the rebalance would be profitable
                let input_amount = volume / sell_token_price;
                match self
                    .jupiter_quote_cache
                    .quote(
                        &self.mango_client,
                        sell_mint,
                        buy_mint,
                        input_amount.clamp_to_u64(),
                        self.config.jupiter_slippage_bps,
                        self.config.jupiter_version,
                        taker_price_profit,
                    )
                    .await?
                {
                    JupiterQuoteCacheResult::Quote((price, quote)) => {
                        swap_price = price;

                        // Store and execute quote if mode needs it
                        jupiter_quote = (mode == Mode::SwapSellIntoBuy).then_some(quote);
                    }
                    JupiterQuoteCacheResult::BadPrice(price) => {
                        swap_price = price;
                        bad_price = true;
                        jupiter_quote = None;
                    }
                }
            }
            Mode::SwapCollateralIntoBuy => {
                let (_, collateral_price, collateral_mint) =
                    self.token_bank_price_mint(collateral_token_index)?;

                let max_sell = volume / sell_token_price;
                let max_buy_collateral_cost = volume / collateral_price;

                // In this mode, we buy the buy token with collateral token before taking the tcs.
                // Get a quote and store it so it will get executed later.
                // The quote for sell_token -> collateral is just to check profitability, rebalancing
                // will take care of it.
                match self
                    .jupiter_quote_cache
                    .quote_collateral_swap(
                        &self.mango_client,
                        collateral_mint,
                        buy_mint,
                        sell_mint,
                        max_buy_collateral_cost.clamp_to_u64(),
                        max_sell.clamp_to_u64(),
                        self.config.jupiter_slippage_bps,
                        self.config.jupiter_version,
                        taker_price_profit,
                    )
                    .await?
                {
                    JupiterQuoteCacheResult::Quote((price, collateral_to_buy_quote, _)) => {
                        swap_price = price;
                        jupiter_quote = collateral_to_buy_quote;
                    }
                    JupiterQuoteCacheResult::BadPrice(price) => {
                        swap_price = price;
                        bad_price = true;
                        jupiter_quote = None;
                    }
                }
            }
        };

        let min_taker_price = (swap_price * (1.0 + self.config.profit_fraction)) as f32;
        if bad_price || taker_price.to_num::<f32>() < min_taker_price {
            trace!(
                max_buy = max_buy_token_to_liqee,
                max_sell = max_sell_token_to_liqor,
                jupiter_swap_price = %swap_price,
                tcs_taker_price = %taker_price,
                "skipping because swap price isn't good enough compared to trigger price",
            );
            return Ok(None);
        }

        let min_buy = (volume / buy_token_price).to_num::<f64>() * self.config.min_buy_fraction;

        trace!(
            max_buy = max_buy_token_to_liqee,
            max_sell = max_sell_token_to_liqor,
            "prepared execution",
        );

        Ok(Some(PreparedExecution {
            pubkey: *pubkey,
            tcs_id: tcs.id,
            volume: volume.clamp_to_u64(),
            token_indexes: vec![tcs.buy_token_index, tcs.sell_token_index],
            max_buy_token_to_liqee,
            max_sell_token_to_liqor,
            min_buy_token: min_buy as u64,
            min_taker_price,
            jupiter_quote,
        }))
    }

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
        tcs: &mut [(Pubkey, u64, u64)],
        error_tracking: &mut ErrorTracking<Pubkey, LiqErrorType>,
    ) -> anyhow::Result<(Vec<Signature>, Vec<Pubkey>)> {
        use rand::distributions::{Distribution, WeightedError, WeightedIndex};

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
                        trace!(%result.pubkey, "preparation error {:?}", e);
                        error_tracking.record(
                            LiqErrorType::TcsExecution,
                            &result.pubkey,
                            e.to_string(),
                        );
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
            .flat_map(|v| &v.token_indexes)
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
                    trace!(%pubkey, "execution error {:?}", err);
                    error_tracking.record(LiqErrorType::TcsExecution, &pubkey, err.to_string());
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
        error_tracking: &ErrorTracking<Pubkey, LiqErrorType>,
    ) -> Option<Pin<Box<dyn Future<Output = PreparationResult> + Send>>> {
        // Skip a pubkey if there've been too many errors recently
        if let Some(error_entry) =
            error_tracking.had_too_many_errors(LiqErrorType::TcsExecution, pubkey, Instant::now())
        {
            trace!(
                "skip checking for tcs on account {pubkey}, had {} errors recently",
                error_entry.count
            );
            return None;
        }

        let context = self.clone();
        let pubkey = *pubkey;
        let job = async move {
            PreparationResult {
                pubkey,
                pending_volume: volume,
                prepared: context
                    .prepare_token_conditional_swap(&pubkey, tcs_id)
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
        // Jupiter quote is provided only for triggers, not close-expired
        let mut tx_builder = if let Some(jupiter_quote) = pending.jupiter_quote {
            self.mango_client
                .jupiter()
                .prepare_swap_transaction(&jupiter_quote)
                .await?
        } else {
            // compute ix is part of the jupiter swap in the above case
            let compute_ix =
                solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                    self.config.compute_limit_for_trigger,
                );
            let fee_payer = self.mango_client.client.fee_payer();
            TransactionBuilder {
                instructions: vec![compute_ix],
                signers: vec![self.mango_client.owner.clone(), fee_payer],
                ..self.mango_client.transaction_builder().await?
            }
        };

        let liqee = self.account_fetcher.fetch_mango_account(&pending.pubkey)?;
        let mut trigger_ixs = self
            .mango_client
            .token_conditional_swap_trigger_instruction(
                (&pending.pubkey, &liqee),
                pending.tcs_id,
                pending.max_buy_token_to_liqee,
                pending.max_sell_token_to_liqor,
                pending.min_buy_token,
                pending.min_taker_price,
                &allowed_tokens,
            )
            .await?;
        tx_builder
            .instructions
            .append(&mut trigger_ixs.instructions);

        let txsig = tx_builder
            .send_and_confirm(&self.mango_client.client)
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
