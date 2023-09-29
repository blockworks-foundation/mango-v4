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
use mango_v4_client::{chain_data, health_cache, jupiter, MangoClient, TransactionBuilder};

use anyhow::Context as AnyhowContext;
use solana_sdk::{signature::Signature, signer::Signer};
use tracing::*;
use {fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

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

#[derive(Clone, Debug)]
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
    SwapSellIntoBuy {
        /// Allowed slippage in jupiter swaps
        slippage_bps: u64,
    },

    /// Do a jupiter swap in the same tx as the trigger, buying the buy token for the
    /// collateral token. This way the liquidator won't need to borrow tokens.
    SwapCollateralIntoBuy { slippage_bps: u64 },
}

#[derive(Clone)]
pub struct Config {
    pub min_health_ratio: f64,
    pub max_trigger_quote_amount: u64,
    pub refresh_timeout: Duration,
    pub compute_limit_for_trigger: u32,
    pub collateral_token_index: TokenIndex,

    /// At 0, the liquidator would trigger tcs if the cost to the liquidator is the
    /// same as the cost to the liqee. 0.1 would mean a 10% better price to the liquidator.
    pub profit_fraction: f64,

    /// Minimum fraction of max_buy to buy for success when triggering,
    /// useful in conjuction with jupiter swaps in same tx to avoid over-buying.
    ///
    /// Can be set to 0 to allow executions of any size.
    pub min_buy_fraction: f64,

    pub jupiter_version: jupiter::Version,
    pub mode: Mode,
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
    min_taker_price: f64,
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
    pub token_swap_info: Arc<token_swap_info::TokenSwapInfoUpdater>,
    pub config: Config,
    pub now_ts: u64,
}

impl Context {
    fn token_bank_price_mint(
        &self,
        token_index: TokenIndex,
    ) -> anyhow::Result<(Bank, I80F48, Pubkey)> {
        let info = self.mango_client.context.mint_info(token_index);
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
        let taker_price = tcs.taker_price(tcs.premium_price(base_price, self.now_ts)) as f64;

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
        let cost_over_oracle = buy_info.buy_over_oracle * sell_info.sell_over_oracle;

        Ok(taker_price >= base_price * cost_over_oracle * (1.0 + self.config.profit_fraction))
    }

    // Either expired or triggerable with ok-looking price.
    fn tcs_is_interesting(&self, tcs: &TokenConditionalSwap) -> anyhow::Result<bool> {
        if tcs.is_expired(self.now_ts) {
            return Ok(true);
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
    /// - net borrow limits on BOTH sides, even though the buy side is technically
    ///   a liqor limitation: the liqor could acquire the token before trying the
    ///   execution... but in practice the liqor will work on margin
    ///
    /// Returns Some((native buy amount, native sell amount)) if execution is sensible
    /// Returns None if the execution should be skipped (due to net borrow limits...)
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
            &self.mango_client,
            &self.account_fetcher,
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
        // Usually, we want to execute tcs even for small amounts because that will close the
        // tcs order: either due to full execution or due to the health threshold being reached.
        //
        // However, when the net borrow limits are hit, it will not closed when no further execution
        // is possible, because net borrow limit issues are considered transient. Furthermore, we
        // don't even want to send a tiny tcs trigger transactions, because there's a good chance we
        // would then be sending lot of those as oracle prices fluctuate.
        //
        // Thus, we need to detect if the possible execution amount is tiny _because_ of the
        // net borrow limits. Then skip. If it's tiny for other reasons we can proceed.

        fn available_borrows(bank: &Bank, price: I80F48) -> u64 {
            (bank.remaining_net_borrows_quote(price) / price).clamp_to_u64()
        }
        let available_buy_borrows = available_borrows(&buy_bank, buy_token_price);
        let available_sell_borrows = available_borrows(&sell_bank, sell_token_price);

        // New borrows if max_sell_ignoring_net_borrows was withdrawn on the liqee
        let sell_borrows = (I80F48::from(max_sell_ignoring_net_borrows)
            - sell_position.max(I80F48::ZERO))
        .clamp_to_u64();

        // On the buy side, the liqor might need to borrow
        let buy_borrows = match self.config.mode {
            Mode::BorrowBuyToken => {
                // Assume that the liqor has enough buy token if it's collateral
                if tcs.buy_token_index == self.config.collateral_token_index {
                    0
                } else {
                    max_buy_ignoring_net_borrows
                }
            }
            Mode::SwapCollateralIntoBuy { .. } => 0,
            Mode::SwapSellIntoBuy { .. } => {
                // Never needs buy borrows.
                // This might need extra sell borrows, but falls back onto SwapCollateralIntoBuy if needed
                0
            }
        };

        // New maximums adjusted for net borrow limits
        let max_sell =
            max_sell_ignoring_net_borrows - sell_borrows + sell_borrows.min(available_sell_borrows);
        let max_buy =
            max_buy_ignoring_net_borrows - buy_borrows + buy_borrows.min(available_buy_borrows);

        info!(
            buy_borrows,
            available_buy_borrows,
            max_buy,
            sell_borrows,
            available_sell_borrows,
            max_sell,
            "borrows?"
        );

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
        &self,
        pubkey: &Pubkey,
    ) -> anyhow::Result<Vec<anyhow::Result<(Pubkey, u64, u64)>>> {
        let liqee = self.account_fetcher.fetch_mango_account(pubkey)?;

        let interesting_tcs = liqee.active_token_conditional_swaps().filter_map(|tcs| {
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
        });
        Ok(interesting_tcs.collect_vec())
    }

    #[allow(clippy::too_many_arguments)]
    async fn prepare_token_conditional_swap(
        &self,
        pubkey: &Pubkey,
        tcs_id: u64,
    ) -> anyhow::Result<Option<PreparedExecution>> {
        let now_ts: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
            .try_into()?;
        let liqee = self.account_fetcher.fetch_mango_account(pubkey)?;
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
        let fetcher = self.account_fetcher.as_ref();
        let health_cache = health_cache::new(&self.mango_client.context, fetcher, &liqee_old)
            .await
            .context("creating health cache 1")?;
        if health_cache.is_liquidatable() {
            return Ok(None);
        }

        // get a fresh account and re-check the tcs and health
        let liqee = self
            .account_fetcher
            .fetch_fresh_mango_account(pubkey)
            .await?;
        let (_, tcs) = liqee.token_conditional_swap_by_id(tcs_id)?;
        if tcs.is_expired(self.now_ts) || !self.tcs_is_interesting(tcs)? {
            return Ok(None);
        }

        let health_cache = health_cache::new(&self.mango_client.context, fetcher, &liqee)
            .await
            .context("creating health cache 2")?;
        if health_cache.is_liquidatable() {
            return Ok(None);
        }

        self.prepare_token_conditional_swap_inner2(pubkey, &liqee, tcs)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all, fields(%pubkey, tcs_id = tcs.id))]
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
            None => return Ok(None),
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
            Mode::SwapSellIntoBuy { slippage_bps } => {
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
                    Mode::SwapCollateralIntoBuy { slippage_bps }
                } else {
                    Mode::SwapSellIntoBuy { slippage_bps }
                }
            }
            m @ _ => m,
        };

        let mut liqor = self.mango_client.mango_account().await?;

        let collateral_token_index = self.config.collateral_token_index;
        let liqor_existing_buy_token = liqor
            .ensure_token_position(tcs.buy_token_index)?
            .0
            .native(&buy_bank);
        let liqor_available_buy_token = match mode {
            Mode::BorrowBuyToken => util::max_swap_source(
                &self.mango_client,
                &self.account_fetcher,
                &liqor,
                tcs.buy_token_index,
                tcs.sell_token_index,
                taker_price,
                liqor_min_health_ratio,
            )?,
            Mode::SwapCollateralIntoBuy { slippage_bps } => {
                // The transaction will be net-positive, but how much buy token can
                // the collateral be swapped into?
                if tcs.buy_token_index == collateral_token_index {
                    liqor_existing_buy_token
                } else {
                    let (_, collateral_price, _) =
                        self.token_bank_price_mint(collateral_token_index)?;
                    let buy_per_collateral_price = (collateral_price / buy_token_price)
                        * I80F48::from_num(1.0 - slippage_bps as f64 * 0.0001);
                    let collateral_amount = util::max_swap_source(
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
            Mode::SwapSellIntoBuy { slippage_bps } => {
                // How big can the sell -> buy swap be?
                let buy_per_sell_price = (I80F48::from(1) / taker_price)
                    * I80F48::from_num(1.0 - slippage_bps as f64 * 0.0001);
                let max_sell = util::max_swap_source(
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
            return Ok(None);
        }

        // The quote amount the swap could be at
        let volume = (I80F48::from(max_buy_token_to_liqee) * buy_token_price)
            .min(I80F48::from(max_sell_token_to_liqor) * sell_token_price);

        // Final check of the reverse trade on jupiter
        let jupiter_quote;
        let swap_price;
        match mode {
            Mode::BorrowBuyToken => {
                // Quote only to verify that the execution is profitable
                // Slippage does not matter.
                let slippage_bps = 100;
                let input_amount = volume / sell_token_price;
                let quote = self
                    .mango_client
                    .jupiter()
                    .quote(
                        sell_mint,
                        buy_mint,
                        input_amount.clamp_to_u64(),
                        slippage_bps,
                        false,
                        self.config.jupiter_version,
                    )
                    .await?;

                let sell_amount = quote.in_amount as f64;
                let buy_amount = quote.out_amount as f64;

                swap_price = sell_amount / buy_amount;
                jupiter_quote = None;
            }
            Mode::SwapSellIntoBuy { slippage_bps, .. } => {
                // Quote will get executed
                let input_amount = volume / sell_token_price;
                let quote = self
                    .mango_client
                    .jupiter()
                    .quote(
                        sell_mint,
                        buy_mint,
                        input_amount.clamp_to_u64(),
                        slippage_bps,
                        false,
                        self.config.jupiter_version,
                    )
                    .await?;

                let sell_amount = quote.in_amount as f64;
                let buy_amount = quote.out_amount as f64;

                swap_price = sell_amount / buy_amount;
                jupiter_quote = Some(quote);
            }
            Mode::SwapCollateralIntoBuy { slippage_bps } => {
                let (_, collateral_price, collateral_mint) =
                    self.token_bank_price_mint(collateral_token_index)?;

                let max_sell = volume / sell_token_price;
                let max_buy_collateral_cost = volume / collateral_price;

                // In this mode, we buy the buy token with collateral token before taking the tcs.
                // Get a quote and store it so it will get executed later.
                let buy_price; // collateral per buy token
                if collateral_token_index != tcs.buy_token_index {
                    let buy_quote = self
                        .mango_client
                        .jupiter()
                        .quote(
                            collateral_mint,
                            buy_mint,
                            max_buy_collateral_cost.clamp_to_u64(),
                            slippage_bps,
                            false,
                            self.config.jupiter_version,
                        )
                        .await?;

                    buy_price = buy_quote.in_amount as f64 / buy_quote.out_amount as f64;
                    jupiter_quote = Some(buy_quote);
                } else {
                    buy_price = 1.0;
                    jupiter_quote = None;
                }

                // To get the overall price and profitability, we need to get a quote for converting back
                let sell_price;
                if collateral_token_index != tcs.sell_token_index {
                    // this one verifies profitability
                    let sell_quote = self
                        .mango_client
                        .jupiter()
                        .quote(
                            sell_mint,
                            collateral_mint,
                            max_sell.clamp_to_u64(),
                            slippage_bps,
                            false,
                            self.config.jupiter_version,
                        )
                        .await?;
                    sell_price = sell_quote.out_amount as f64 / sell_quote.in_amount as f64;
                } else {
                    sell_price = 1.0;
                }

                // sell token per buy token
                swap_price = buy_price / sell_price;
            }
        };

        let min_taker_price = swap_price * (1.0 + self.config.profit_fraction);
        if min_taker_price > taker_price.to_num::<f64>() {
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
            TransactionBuilder {
                instructions: vec![compute_ix],
                address_lookup_tables: vec![],
                payer: self.mango_client.client.fee_payer.pubkey(),
                signers: vec![
                    self.mango_client.owner.clone(),
                    self.mango_client.client.fee_payer.clone(),
                ],
                config: self.mango_client.client.transaction_builder_config,
            }
        };

        let liqee = self.account_fetcher.fetch_mango_account(&pending.pubkey)?;
        let trigger_ix = self
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
        tx_builder.instructions.push(trigger_ix);

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
