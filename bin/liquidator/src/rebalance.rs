use itertools::Itertools;
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::state::{
    Bank, BookSide, MangoAccountValue, OracleAccountInfos, PerpMarket, PerpPosition,
    PlaceOrderType, Side, TokenIndex, QUOTE_TOKEN_INDEX,
};
use mango_v4_client::{
    chain_data, jupiter, perp_pnl, MangoClient, PerpMarketContext, TokenContext,
    TransactionBuilder, TransactionSize,
};

use {fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

use solana_sdk::signature::Signature;
use std::sync::Arc;
use std::time::Duration;
use tracing::*;

#[derive(Clone)]
pub struct Config {
    pub enabled: bool,
    /// Maximum slippage allowed in Jupiter
    pub slippage_bps: u64,
    /// When closing borrows, the rebalancer can't close token positions exactly.
    /// Instead it purchases too much and then gets rid of the excess in a second step.
    /// If this is 1.05, then it'll swap borrow_value * 1.05 quote token into borrow token.
    pub borrow_settle_excess: f64,
    pub refresh_timeout: Duration,
    pub jupiter_version: jupiter::Version,
    pub skip_tokens: Vec<TokenIndex>,
    pub allow_withdraws: bool,
}

fn token_bank(
    token: &TokenContext,
    account_fetcher: &chain_data::AccountFetcher,
) -> anyhow::Result<Bank> {
    account_fetcher.fetch::<Bank>(&token.first_bank())
}

pub struct Rebalancer {
    pub mango_client: Arc<MangoClient>,
    pub account_fetcher: Arc<chain_data::AccountFetcher>,
    pub mango_account_address: Pubkey,
    pub config: Config,
}

impl Rebalancer {
    pub async fn zero_all_non_quote(&self) -> anyhow::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        trace!(
            pubkey = %self.mango_account_address,
            "checking for rebalance"
        );

        self.rebalance_perps().await?;
        self.rebalance_tokens().await?;

        Ok(())
    }

    /// Function to refresh the mango account after the txsig confirmed. Returns false on timeout.
    async fn refresh_mango_account_after_tx(&self, txsig: Signature) -> anyhow::Result<bool> {
        let max_slot = self.account_fetcher.transaction_max_slot(&[txsig]).await?;
        if let Err(e) = self
            .account_fetcher
            .refresh_accounts_via_rpc_until_slot(
                &[self.mango_account_address],
                max_slot,
                self.config.refresh_timeout,
            )
            .await
        {
            // If we don't get fresh data, maybe the tx landed on a fork?
            // Rebalance is technically still ok.
            info!("could not refresh account data: {}", e);
            return Ok(false);
        }
        Ok(true)
    }

    async fn jupiter_quote(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
        only_direct_routes: bool,
        jupiter_version: jupiter::Version,
    ) -> anyhow::Result<jupiter::Quote> {
        self.mango_client
            .jupiter()
            .quote(
                input_mint,
                output_mint,
                amount,
                self.config.slippage_bps,
                only_direct_routes,
                jupiter_version,
            )
            .await
    }

    /// Grab three possible routes:
    /// 1. USDC -> output (complex routes)
    /// 2. USDC -> output (direct route only)
    /// 3. SOL -> output (direct route only)
    /// Use 1. if it fits into a tx. Otherwise use the better of 2./3.
    async fn token_swap_buy(
        &self,
        output_mint: Pubkey,
        in_amount_quote: u64,
    ) -> anyhow::Result<(Signature, jupiter::Quote)> {
        let quote_token = self.mango_client.context.token(QUOTE_TOKEN_INDEX);
        let sol_token = self.mango_client.context.token(
            *self
                .mango_client
                .context
                .token_indexes_by_name
                .get("SOL") // TODO: better use mint
                .unwrap(),
        );
        let quote_mint = quote_token.mint;
        let sol_mint = sol_token.mint;
        let jupiter_version = self.config.jupiter_version;

        let full_route_job = self.jupiter_quote(
            quote_mint,
            output_mint,
            in_amount_quote,
            false,
            jupiter_version,
        );
        let direct_quote_route_job = self.jupiter_quote(
            quote_mint,
            output_mint,
            in_amount_quote,
            true,
            jupiter_version,
        );

        // For the SOL -> output route we need to adjust the in amount by the SOL price
        let sol_price = self
            .account_fetcher
            .fetch_bank_price(&sol_token.first_bank())?;
        let in_amount_sol = (I80F48::from(in_amount_quote) / sol_price)
            .ceil()
            .to_num::<u64>();
        let direct_sol_route_job =
            self.jupiter_quote(sol_mint, output_mint, in_amount_sol, true, jupiter_version);

        let jobs = vec![full_route_job, direct_quote_route_job, direct_sol_route_job];

        let mut results = futures::future::join_all(jobs).await;
        let full_route = results.remove(0)?;
        let alternatives = results.into_iter().filter_map(|v| v.ok()).collect_vec();

        let (tx_builder, route) = self
            .determine_best_jupiter_tx(
                // If the best_route couldn't be fetched, something is wrong
                &full_route,
                &alternatives,
            )
            .await?;
        let sig = tx_builder
            .send_and_confirm(&self.mango_client.client)
            .await?;
        Ok((sig, route))
    }

    /// Grab three possible routes:
    /// 1. input -> USDC (complex routes)
    /// 2. input -> USDC (direct route only)
    /// 3. input -> SOL (direct route only)
    /// Use 1. if it fits into a tx. Otherwise use the better of 2./3.
    async fn token_swap_sell(
        &self,
        input_mint: Pubkey,
        in_amount: u64,
    ) -> anyhow::Result<(Signature, jupiter::Quote)> {
        let quote_token = self.mango_client.context.token(QUOTE_TOKEN_INDEX);
        let sol_token = self.mango_client.context.token(
            *self
                .mango_client
                .context
                .token_indexes_by_name
                .get("SOL") // TODO: better use mint
                .unwrap(),
        );
        let quote_mint = quote_token.mint;
        let sol_mint = sol_token.mint;
        let jupiter_version = self.config.jupiter_version;

        let full_route_job =
            self.jupiter_quote(input_mint, quote_mint, in_amount, false, jupiter_version);
        let direct_quote_route_job =
            self.jupiter_quote(input_mint, quote_mint, in_amount, true, jupiter_version);
        let direct_sol_route_job =
            self.jupiter_quote(input_mint, sol_mint, in_amount, true, jupiter_version);

        let jobs = vec![full_route_job, direct_quote_route_job, direct_sol_route_job];

        let mut results = futures::future::join_all(jobs).await;
        let full_route = results.remove(0)?;
        let alternatives = results.into_iter().filter_map(|v| v.ok()).collect_vec();

        let (tx_builder, route) = self
            .determine_best_jupiter_tx(
                // If the best_route couldn't be fetched, something is wrong
                &full_route,
                &alternatives,
            )
            .await?;

        let sig = tx_builder
            .send_and_confirm(&self.mango_client.client)
            .await?;
        Ok((sig, route))
    }

    async fn determine_best_jupiter_tx(
        &self,
        full: &jupiter::Quote,
        alternatives: &[jupiter::Quote],
    ) -> anyhow::Result<(TransactionBuilder, jupiter::Quote)> {
        let builder = self
            .mango_client
            .jupiter()
            .prepare_swap_transaction(full)
            .await?;
        let tx_size = builder.transaction_size()?;
        if tx_size.is_ok() {
            return Ok((builder, full.clone()));
        }
        trace!(
            route_label = full.first_route_label(),
            %full.input_mint,
            %full.output_mint,
            ?tx_size,
            limit = ?TransactionSize::limit(),
            "full route does not fit in a tx",
        );

        if alternatives.is_empty() {
            anyhow::bail!(
                "no alternative routes from {} to {}",
                full.input_mint,
                full.output_mint
            );
        }

        let best = alternatives
            .iter()
            .min_by(|a, b| a.price_impact_pct.partial_cmp(&b.price_impact_pct).unwrap())
            .unwrap();
        let builder = self
            .mango_client
            .jupiter()
            .prepare_swap_transaction(best)
            .await?;
        Ok((builder, best.clone()))
    }

    fn mango_account(&self) -> anyhow::Result<Box<MangoAccountValue>> {
        Ok(Box::new(
            self.account_fetcher
                .fetch_mango_account(&self.mango_account_address)?,
        ))
    }

    async fn rebalance_tokens(&self) -> anyhow::Result<()> {
        let account = self.mango_account()?;

        // TODO: configurable?
        let quote_token = self.mango_client.context.token(QUOTE_TOKEN_INDEX);

        for token_position in account.active_token_positions() {
            let token_index = token_position.token_index;
            let token = self.mango_client.context.token(token_index);
            if token_index == quote_token.token_index
                || self.config.skip_tokens.contains(&token_index)
            {
                continue;
            }
            let token_mint = token.mint;
            let token_price = self.account_fetcher.fetch_bank_price(&token.first_bank())?;

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
            let dust_threshold = I80F48::from(2) / token_price;

            // Some rebalancing can actually change non-USDC positions (rebalancing to SOL)
            // So re-fetch the current token position amount
            let bank = token_bank(token, &self.account_fetcher)?;
            let fresh_amount = || -> anyhow::Result<I80F48> {
                Ok(self
                    .mango_account()?
                    .token_position_and_raw_index(token_index)
                    .map(|(position, _)| position.native(&bank))
                    .unwrap_or(I80F48::ZERO))
            };
            let mut amount = fresh_amount()?;

            trace!(token_index, %amount, %dust_threshold, "checking");
            if amount < 0 {
                // Buy
                let buy_amount =
                    amount.abs().ceil() + (dust_threshold - I80F48::ONE).max(I80F48::ZERO);
                let input_amount =
                    buy_amount * token_price * I80F48::from_num(self.config.borrow_settle_excess);
                let (txsig, route) = self
                    .token_swap_buy(token_mint, input_amount.to_num())
                    .await?;
                let in_token = self
                    .mango_client
                    .context
                    .token_by_mint(&route.input_mint)
                    .unwrap();
                info!(
                    %txsig,
                    "bought {} {} for {} {}",
                    token.native_to_ui(I80F48::from(route.out_amount)),
                    token.name,
                    in_token.native_to_ui(I80F48::from(route.in_amount)),
                    in_token.name,
                );
                if !self.refresh_mango_account_after_tx(txsig).await? {
                    return Ok(());
                }
                amount = fresh_amount()?;
            }

            if amount > dust_threshold {
                // Sell
                let (txsig, route) = self
                    .token_swap_sell(token_mint, amount.to_num::<u64>())
                    .await?;
                let out_token = self
                    .mango_client
                    .context
                    .token_by_mint(&route.output_mint)
                    .unwrap();
                info!(
                    %txsig,
                    "sold {} {} for {} {}",
                    token.native_to_ui(I80F48::from(route.in_amount)),
                    token.name,
                    out_token.native_to_ui(I80F48::from(route.out_amount)),
                    out_token.name,
                );
                if !self.refresh_mango_account_after_tx(txsig).await? {
                    return Ok(());
                }
                amount = fresh_amount()?;
            }

            // Any remainder that could not be sold just gets withdrawn to ensure the
            // TokenPosition is freed up
            if amount > 0
                && amount <= dust_threshold
                && !token_position.is_in_use()
                && self.config.allow_withdraws
            {
                let allow_borrow = false;
                let txsig = self
                    .mango_client
                    .token_withdraw(token_mint, u64::MAX, allow_borrow)
                    .await?;
                info!(
                    %txsig,
                    "withdrew {} {} to liqor wallet",
                    token.native_to_ui(amount),
                    token.name,
                );
                if !self.refresh_mango_account_after_tx(txsig).await? {
                    return Ok(());
                }
            } else if amount > dust_threshold {
                warn!(
                    "unexpected {} position after rebalance swap: {} native",
                    token.name, amount
                );
            }
        }

        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            perp_market_name = perp.name,
            base_lots = perp_position.base_position_lots(),
            effective_lots = perp_position.effective_base_position_lots(),
            quote_native = %perp_position.quote_position_native()
        )
    )]
    async fn rebalance_perp(
        &self,
        account: &MangoAccountValue,
        perp: &PerpMarketContext,
        perp_position: &PerpPosition,
    ) -> anyhow::Result<bool> {
        let now_ts: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let base_lots = perp_position.base_position_lots();
        let effective_lots = perp_position.effective_base_position_lots();
        let quote_native = perp_position.quote_position_native();
        let perp_market: PerpMarket = self.account_fetcher.fetch(&perp.address)?;

        if effective_lots != 0 {
            // send an ioc order to reduce the base position
            let oracle_account_data = self.account_fetcher.fetch_raw(&perp.oracle)?;
            let oracle_account = KeyedAccountSharedData::new(perp.oracle, oracle_account_data);
            let oracle_price = perp_market
                .oracle_price(&OracleAccountInfos::from_reader(&oracle_account), None)?;
            let oracle_price_lots = perp_market.native_price_to_lot(oracle_price);
            let (side, order_price, oo_lots) = if effective_lots > 0 {
                (
                    Side::Ask,
                    oracle_price * (I80F48::ONE - perp_market.base_liquidation_fee),
                    perp_position.asks_base_lots,
                )
            } else {
                (
                    Side::Bid,
                    oracle_price * (I80F48::ONE + perp_market.base_liquidation_fee),
                    perp_position.bids_base_lots,
                )
            };
            let price_lots = perp_market.native_price_to_lot(order_price);
            let max_base_lots = effective_lots.abs() - oo_lots;
            if max_base_lots <= 0 {
                warn!(?side, oo_lots, "cannot place reduce-only order",);
                return Ok(true);
            }

            // Check the orderbook before sending the ioc order to see if we could
            // even match anything. That way we don't need to pay the tx fee and
            // ioc penalty fee unnecessarily.
            let opposite_side_key = match side.invert_side() {
                Side::Bid => perp.bids,
                Side::Ask => perp.asks,
            };
            let bookside = Box::new(self.account_fetcher.fetch::<BookSide>(&opposite_side_key)?);
            if bookside.quantity_at_price(price_lots, now_ts, oracle_price_lots) <= 0 {
                warn!(
                    other_side = ?side.invert_side(),
                    %order_price,
                    %oracle_price,
                    "no liquidity",
                );
                return Ok(true);
            }

            let txsig = self
                .mango_client
                .perp_place_order(
                    perp_position.market_index,
                    side,
                    price_lots,
                    max_base_lots,
                    i64::MAX,
                    0,
                    PlaceOrderType::ImmediateOrCancel,
                    true, // reduce only
                    0,
                    10,
                    mango_v4::state::SelfTradeBehavior::DecrementTake,
                )
                .await?;
            info!(
                %txsig,
                %order_price,
                "attempt to ioc reduce perp base position"
            );
            if !self.refresh_mango_account_after_tx(txsig).await? {
                return Ok(false);
            }
        } else if base_lots == 0 && quote_native != 0 {
            // settle pnl
            let direction = if quote_native > 0 {
                perp_pnl::Direction::MaxNegative
            } else {
                perp_pnl::Direction::MaxPositive
            };
            let counters = perp_pnl::fetch_top(
                &self.mango_client.context,
                &self.mango_client.client.config().fallback_oracle_config,
                self.account_fetcher.as_ref(),
                perp_position.market_index,
                direction,
                2,
            )
            .await?;
            if counters.is_empty() {
                // If we can't settle some positive PNL because we're lacking a suitable counterparty,
                // then liquidation should continue, even though this step produced no transaction
                info!("could not settle perp pnl on perp market: no counterparty",);
                return Ok(true);
            }
            let (counter_key, counter_acc, _counter_pnl) = counters.first().unwrap();

            let (account_a, account_b) = if quote_native > 0 {
                (
                    (&self.mango_account_address, account),
                    (counter_key, counter_acc),
                )
            } else {
                (
                    (counter_key, counter_acc),
                    (&self.mango_account_address, account),
                )
            };
            let txsig = self
                .mango_client
                .perp_settle_pnl(perp_position.market_index, account_a, account_b)
                .await?;
            info!(%txsig, "settled perp pnl");
            if !self.refresh_mango_account_after_tx(txsig).await? {
                return Ok(false);
            }
        } else if base_lots == 0 && quote_native == 0 {
            // close perp position
            let txsig = self
                .mango_client
                .perp_deactivate_position(perp_position.market_index)
                .await?;
            info!(
                %txsig, "closed perp position"
            );
            if !self.refresh_mango_account_after_tx(txsig).await? {
                return Ok(false);
            }
        } else {
            // maybe we're still waiting for consume_events
            info!("cannot deactivate perp position, waiting for consume events?");
        }
        Ok(true)
    }

    async fn rebalance_perps(&self) -> anyhow::Result<()> {
        let account = self.mango_account()?;

        for perp_position in account.active_perp_positions() {
            let perp = self.mango_client.context.perp(perp_position.market_index);
            if !self.rebalance_perp(&account, perp, perp_position).await? {
                return Ok(());
            }
        }

        Ok(())
    }
}
