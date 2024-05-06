use itertools::Itertools;
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::state::{
    Bank, BookSide, MangoAccountValue, OracleAccountInfos, PerpMarket, PerpPosition,
    PlaceOrderType, Serum3MarketIndex, Side, TokenIndex, QUOTE_TOKEN_INDEX,
};
use mango_v4_client::{
    chain_data, perp_pnl, swap, MangoClient, MangoGroupContext, PerpMarketContext,
    PreparedInstructions, Serum3MarketContext, TokenContext, TransactionBuilder, TransactionSize,
};
use solana_client::nonblocking::rpc_client::RpcClient;

use {fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

use fixed::types::extra::U48;
use fixed::FixedI128;
use mango_v4::accounts_ix::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};
use mango_v4::serum3_cpi;
use mango_v4::serum3_cpi::{OpenOrdersAmounts, OpenOrdersSlim};
use solana_sdk::account::ReadableAccount;
use solana_sdk::signature::Signature;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::*;

#[derive(Clone)]
pub struct Config {
    pub enabled: bool,
    /// Maximum slippage allowed in Jupiter
    pub slippage_bps: u64,
    /// Maximum slippage from oracle price for limit orders
    pub limit_order_distance_from_oracle_price_bps: u64,
    /// When closing borrows, the rebalancer can't close token positions exactly.
    /// Instead it purchases too much and then gets rid of the excess in a second step.
    /// If this is 1.05, then it'll swap borrow_value * 1.05 quote token into borrow token.
    pub borrow_settle_excess: f64,
    pub refresh_timeout: Duration,
    pub jupiter_version: swap::Version,
    pub skip_tokens: Vec<TokenIndex>,
    pub alternate_jupiter_route_tokens: Vec<TokenIndex>,
    pub alternate_sanctum_route_tokens: Vec<TokenIndex>,
    pub allow_withdraws: bool,
    pub use_sanctum: bool,
    pub use_limit_order: bool,
}

impl Config {
    // panics on failure
    pub fn validate(&self, context: &MangoGroupContext) {
        self.skip_tokens.iter().for_each(|&ti| {
            context.token(ti);
        });
        self.alternate_jupiter_route_tokens.iter().for_each(|&ti| {
            context.token(ti);
        });
    }
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
    pub sanctum_supported_mints: HashSet<Pubkey>,
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

        let rebalance_perps_res = self.rebalance_perps().await;
        let rebalance_tokens_res = self.rebalance_tokens().await;

        if rebalance_perps_res.is_err() && rebalance_tokens_res.is_err() {
            anyhow::bail!(
                "Failed to rebalance perps ({}) and tokens ({})",
                rebalance_perps_res.unwrap_err(),
                rebalance_tokens_res.unwrap_err()
            )
        }

        rebalance_perps_res?;
        rebalance_tokens_res?;
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

    async fn swap_quote(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
        only_direct_routes: bool,
        jupiter_version: swap::Version,
    ) -> anyhow::Result<swap::Quote> {
        self.mango_client
            .swap()
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

    /// Grab multiples possible routes:
    /// 1. USDC -> output (complex routes)
    /// 2. USDC -> output (direct route only)
    /// 3. if enabled, sanctum routes - might generate 0, 1 or more routes
    /// 4. input -> alternate_jupiter_route_tokens (direct route only) - might generate 0, 1 or more routes
    /// Use best of 1/2/3. if it fits into a tx,
    /// Otherwise use the best of 4.
    async fn token_swap_buy(
        &self,
        account: &MangoAccountValue,
        output_mint: Pubkey,
        in_amount_quote: u64,
    ) -> anyhow::Result<(Signature, swap::Quote)> {
        let quote_token = self.mango_client.context.token(QUOTE_TOKEN_INDEX);
        let quote_mint = quote_token.mint;
        let jupiter_version = self.config.jupiter_version;

        let full_route_job = self.swap_quote(
            quote_mint,
            output_mint,
            in_amount_quote,
            false,
            jupiter_version,
        );
        let direct_quote_route_job = self.swap_quote(
            quote_mint,
            output_mint,
            in_amount_quote,
            true,
            jupiter_version,
        );
        let mut jobs = vec![full_route_job, direct_quote_route_job];

        if self.can_use_sanctum_for_token(output_mint)? {
            for in_token_index in &self.config.alternate_sanctum_route_tokens {
                let (alt_mint, alt_in_amount) =
                    self.get_alternative_token_amount(in_token_index, in_amount_quote)?;
                let sanctum_alt_route_job = self.swap_quote(
                    alt_mint,
                    output_mint,
                    alt_in_amount,
                    false,
                    swap::Version::Sanctum,
                );
                jobs.push(sanctum_alt_route_job);
            }
        }

        let results = futures::future::join_all(jobs).await;
        let routes: Vec<_> = results.into_iter().filter_map(|v| v.ok()).collect_vec();

        let best_route_res = self
            .determine_best_swap_tx(routes, quote_mint, output_mint)
            .await;

        let (mut tx_builder, route) = match best_route_res {
            Ok(x) => x,
            Err(e) => {
                warn!("could not use simple routes because of {}, trying with an alternative one (if configured)", e);

                self.get_jupiter_alt_route(
                    |in_token_index| {
                        let (alt_mint, alt_in_amount) =
                            self.get_alternative_token_amount(in_token_index, in_amount_quote)?;
                        let swap = self.swap_quote(
                            alt_mint,
                            output_mint,
                            alt_in_amount,
                            true,
                            jupiter_version,
                        );
                        Ok(swap)
                    },
                    quote_mint,
                    output_mint,
                )
                .await?
            }
        };

        let seq_check_ix = self
            .mango_client
            .sequence_check_instruction(&self.mango_account_address, account)
            .await?;
        tx_builder.append(seq_check_ix);

        let sig = tx_builder
            .send_and_confirm(&self.mango_client.client)
            .await?;
        Ok((sig, route))
    }

    /// Grab multiples possible routes:
    /// 1. input -> USDC (complex routes)
    /// 2. input -> USDC (direct route only)
    /// 3. if enabled, sanctum routes - might generate 0, 1 or more routes
    /// 4. input -> alternate_jupiter_route_tokens (direct route only) - might generate 0, 1 or more routes
    /// Use best of 1/2/3. if it fits into a tx,
    /// Otherwise use the best of 4.
    async fn token_swap_sell(
        &self,
        account: &MangoAccountValue,
        input_mint: Pubkey,
        in_amount: u64,
    ) -> anyhow::Result<(Signature, swap::Quote)> {
        let quote_token = self.mango_client.context.token(QUOTE_TOKEN_INDEX);
        let quote_mint = quote_token.mint;
        let jupiter_version = self.config.jupiter_version;

        let full_route_job =
            self.swap_quote(input_mint, quote_mint, in_amount, false, jupiter_version);
        let direct_quote_route_job =
            self.swap_quote(input_mint, quote_mint, in_amount, true, jupiter_version);
        let mut jobs = vec![full_route_job, direct_quote_route_job];

        if self.can_use_sanctum_for_token(input_mint)? {
            for out_token_index in &self.config.alternate_sanctum_route_tokens {
                let out_token = self.mango_client.context.token(*out_token_index);
                let sanctum_job = self.swap_quote(
                    input_mint,
                    out_token.mint,
                    in_amount,
                    false,
                    swap::Version::Sanctum,
                );
                jobs.push(sanctum_job);
            }
        }

        let results = futures::future::join_all(jobs).await;
        let routes: Vec<_> = results.into_iter().filter_map(|v| v.ok()).collect_vec();

        let best_route_res = self
            .determine_best_swap_tx(routes, input_mint, quote_mint)
            .await;

        let (mut tx_builder, route) = match best_route_res {
            Ok(x) => x,
            Err(e) => {
                warn!("could not use simple routes because of {}, trying with an alternative one (if configured)", e);

                self.get_jupiter_alt_route(
                    |out_token_index| {
                        let out_token = self.mango_client.context.token(*out_token_index);
                        Ok(self.swap_quote(
                            input_mint,
                            out_token.mint,
                            in_amount,
                            true,
                            jupiter_version,
                        ))
                    },
                    input_mint,
                    quote_mint,
                )
                .await?
            }
        };

        let seq_check_ix = self
            .mango_client
            .sequence_check_instruction(&self.mango_account_address, account)
            .await?;
        tx_builder.append(seq_check_ix);

        let sig = tx_builder
            .send_and_confirm(&self.mango_client.client)
            .await?;
        Ok((sig, route))
    }

    fn get_alternative_token_amount(
        &self,
        in_token_index: &u16,
        in_amount_quote: u64,
    ) -> anyhow::Result<(Pubkey, u64)> {
        let in_token: &TokenContext = self.mango_client.context.token(*in_token_index);
        let in_price = self
            .account_fetcher
            .fetch_bank_price(&in_token.first_bank())?;
        let in_amount = (I80F48::from(in_amount_quote) / in_price)
            .ceil()
            .to_num::<u64>();

        Ok((in_token.mint, in_amount))
    }

    fn can_use_sanctum_for_token(&self, mint: Pubkey) -> anyhow::Result<bool> {
        if !self.config.use_sanctum {
            return Ok(false);
        }

        let token = self.mango_client.context.token_by_mint(&mint)?;

        let can_swap_on_sanctum = self.can_swap_on_sanctum(mint);

        // forbid swapping to something that could be used in another sanctum swap, creating a cycle
        let is_an_alt_for_sanctum = self
            .config
            .alternate_sanctum_route_tokens
            .contains(&token.token_index);

        Ok(can_swap_on_sanctum && !is_an_alt_for_sanctum)
    }

    async fn get_jupiter_alt_route<T: Future<Output = anyhow::Result<swap::Quote>>>(
        &self,
        quote_fetcher: impl Fn(&u16) -> anyhow::Result<T>,
        original_input_mint: Pubkey,
        original_output_mint: Pubkey,
    ) -> anyhow::Result<(TransactionBuilder, swap::Quote)> {
        let mut alt_jobs = vec![];
        for in_token_index in &self.config.alternate_jupiter_route_tokens {
            alt_jobs.push(quote_fetcher(in_token_index)?);
        }
        let alt_results = futures::future::join_all(alt_jobs).await;
        let alt_routes: Vec<_> = alt_results.into_iter().filter_map(|v| v.ok()).collect_vec();

        let best_route = self
            .determine_best_swap_tx(alt_routes, original_input_mint, original_output_mint)
            .await?;
        Ok(best_route)
    }

    async fn determine_best_swap_tx(
        &self,
        mut routes: Vec<swap::Quote>,
        original_input_mint: Pubkey,
        original_output_mint: Pubkey,
    ) -> anyhow::Result<(TransactionBuilder, swap::Quote)> {
        let mut prices = HashMap::<Pubkey, I80F48>::new();
        let mut get_or_fetch_price = |m| {
            let entry = prices.entry(m).or_insert_with(|| {
                let token = self
                    .mango_client
                    .context
                    .token_by_mint(&m)
                    .expect("token for mint not found");
                self.account_fetcher
                    .fetch_bank_price(&token.first_bank())
                    .expect("failed to fetch price")
            });
            *entry
        };

        routes.sort_by_cached_key(|r| {
            let in_price = get_or_fetch_price(r.input_mint);
            let out_price = get_or_fetch_price(r.output_mint);
            let amount = out_price * I80F48::from_num(r.out_amount)
                - in_price * I80F48::from_num(r.in_amount);

            let t = match r.raw {
                swap::RawQuote::Mock => "mock",
                swap::RawQuote::V6(_) => "jupiter",
                swap::RawQuote::Sanctum(_) => "sanctum",
            };

            debug!(
                "quote for {} vs {} [using {}] is {}@{} vs {}@{} -> amount={}",
                r.input_mint,
                r.output_mint,
                t,
                r.in_amount,
                in_price,
                r.out_amount,
                out_price,
                amount
            );

            std::cmp::Reverse(amount)
        });

        for route in routes {
            let builder = self
                .mango_client
                .swap()
                .prepare_swap_transaction(&route)
                .await?;
            let tx_size = builder.transaction_size()?;
            if tx_size.is_within_limit() {
                return Ok((builder, route.clone()));
            }

            trace!(
                route_label = route.first_route_label(),
                %route.input_mint,
                %route.output_mint,
                ?tx_size,
                limit = ?TransactionSize::limit(),
                "route does not fit in a tx",
            );
        }

        anyhow::bail!(
            "no routes from {} to {}",
            original_input_mint,
            original_output_mint
        );
    }

    fn mango_account(&self) -> anyhow::Result<Box<MangoAccountValue>> {
        Ok(Box::new(
            self.account_fetcher
                .fetch_mango_account(&self.mango_account_address)?,
        ))
    }

    async fn rebalance_tokens(&self) -> anyhow::Result<()> {
        if self.config.use_limit_order {
            self.settle_and_close_all_openbook_orders().await?;
        }
        let account = self.mango_account()?;

        // TODO: configurable?
        let quote_token = self.mango_client.context.token(QUOTE_TOKEN_INDEX);

        for token_position in Self::shuffle(account.active_token_positions()) {
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
            // To avoid errors, we consider all amounts below 1000 * (1/oracle) dust and don't try
            // to sell them. Instead they will be withdrawn at the end.
            // Purchases will aim to purchase slightly more than is needed, such that we can
            // again withdraw the dust at the end.
            let dust_threshold_res = if self.config.use_limit_order {
                self.dust_threshold_for_limit_order(token)
                    .await
                    .map(|x| I80F48::from(x))
            } else {
                Ok(I80F48::from(2) / token_price)
            };

            let Ok(dust_threshold) = dust_threshold_res
            else {
                let e = dust_threshold_res.unwrap_err();
                error!("Cannot rebalance token {}, probably missing USDC market ? - error: {}", token.name, e);
                continue;
            };

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

            trace!(token_index, token.name, %amount, %dust_threshold, "checking");

            if self.config.use_limit_order {
                self.unwind_using_limit_orders(
                    &account,
                    token,
                    token_price,
                    dust_threshold,
                    amount,
                )
                .await?;
            } else {
                amount = self
                    .unwind_using_swap(
                        &account,
                        token,
                        token_mint,
                        token_price,
                        dust_threshold,
                        fresh_amount,
                        amount,
                    )
                    .await?;
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

    async fn dust_threshold_for_limit_order(&self, token: &TokenContext) -> anyhow::Result<u64> {
        let (_, market) = self
            .mango_client
            .context
            .serum3_markets
            .iter()
            .find(|(_, context)| {
                context.base_token_index == token.token_index
                    && context.quote_token_index == QUOTE_TOKEN_INDEX
            })
            .ok_or(anyhow::format_err!(
                "could not find market for token {}",
                token.name
            ))?;

        Ok(market.coin_lot_size - 1)
    }

    async fn unwind_using_limit_orders(
        &self,
        account: &Box<MangoAccountValue>,
        token: &TokenContext,
        token_price: I80F48,
        dust_threshold: FixedI128<U48>,
        native_amount: I80F48,
    ) -> anyhow::Result<()> {
        if native_amount >= 0 && native_amount < dust_threshold {
            return Ok(());
        }

        let (market_index, market) = self
            .mango_client
            .context
            .serum3_markets
            .iter()
            .find(|(_, context)| {
                context.base_token_index == token.token_index
                    && context.quote_token_index == QUOTE_TOKEN_INDEX
            })
            .ok_or(anyhow::format_err!(
                "could not find market for token {}",
                token.name
            ))?;

        let side = if native_amount < 0 {
            Serum3Side::Bid
        } else {
            Serum3Side::Ask
        };

        let distance_from_oracle_price_bp =
            I80F48::from_num(self.config.limit_order_distance_from_oracle_price_bps)
                * match side {
                    Serum3Side::Bid => 1,
                    Serum3Side::Ask => -1,
                };
        let price_adjustment_factor =
            (I80F48::from_num(10_000) + distance_from_oracle_price_bp) / I80F48::from_num(10_000);

        let limit_price =
            (token_price * price_adjustment_factor * I80F48::from_num(market.coin_lot_size))
                .to_num::<u64>()
                / market.pc_lot_size;
        let mut max_base_lots =
            (native_amount.abs() / I80F48::from_num(market.coin_lot_size)).to_num::<u64>();

        debug!(
            side = match side {
                Serum3Side::Bid => "Buy",
                Serum3Side::Ask => "Sell",
            },
            token = token.name,
            oracle_price = token_price.to_num::<f64>(),
            price_adjustment_factor = price_adjustment_factor.to_num::<f64>(),
            coin_lot_size = market.coin_lot_size,
            pc_lot_size = market.pc_lot_size,
            limit_price,
            native_amount = native_amount.to_num::<f64>(),
            max_base_lots = max_base_lots,
            "building order for rebalancing"
        );

        // Try to buy enough to close the borrow
        if max_base_lots == 0 && native_amount < 0 {
            info!(
                "Buying a whole lot for token {} to cover borrow of {}",
                token.name, native_amount
            );
            max_base_lots = 1;
        }

        if max_base_lots == 0 {
            warn!("Could not rebalance token '{}' (native_amount={}) using limit order, below base lot size", token.name, native_amount);
            return Ok(());
        }

        let mut account = account.clone();
        let create_or_replace_account_ixs = self
            .mango_client
            .serum3_create_or_replace_account_instruction(&mut account, *market_index, side)
            .await?;
        let cancel_ixs =
            self.mango_client
                .serum3_cancel_all_orders_instruction(&account, *market_index, 10)?;
        let place_order_ixs = self
            .mango_client
            .serum3_place_order_instruction(
                &account,
                *market_index,
                side,
                limit_price,
                max_base_lots,
                ((limit_price * max_base_lots * market.pc_lot_size) as f64 * 1.01) as u64,
                Serum3SelfTradeBehavior::CancelProvide,
                Serum3OrderType::Limit,
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
                10,
            )
            .await?;

        let seq_check_ixs = self
            .mango_client
            .sequence_check_instruction(&self.mango_account_address, &account)
            .await?;

        let mut ixs = PreparedInstructions::new();
        ixs.append(create_or_replace_account_ixs);
        ixs.append(cancel_ixs);
        ixs.append(place_order_ixs);
        ixs.append(seq_check_ixs);

        let txsig = self
            .mango_client
            .send_and_confirm_authority_tx(ixs.to_instructions())
            .await?;

        info!(
            %txsig,
            "placed order for {} {} at price = {}",
            token.native_to_ui(I80F48::from(native_amount)),
            token.name,
            limit_price,
        );
        if !self.refresh_mango_account_after_tx(txsig).await? {
            return Ok(());
        }

        Ok(())
    }

    async fn settle_and_close_all_openbook_orders(&self) -> anyhow::Result<()> {
        let account = self.mango_account()?;

        for x in Self::shuffle(account.active_serum3_orders()) {
            let token = self.mango_client.context.token(x.base_token_index);
            let quote = self.mango_client.context.token(x.quote_token_index);
            let market_index = x.market_index;
            let market = self
                .mango_client
                .context
                .serum3_markets
                .get(&market_index)
                .expect("no openbook market found");
            self.settle_and_close_openbook_orders(&account, token, &market_index, market, quote)
                .await?;
        }
        Ok(())
    }

    /// This will only settle funds when there is no more active orders (avoid doing too many settle tx)
    async fn settle_and_close_openbook_orders(
        &self,
        account: &Box<MangoAccountValue>,
        token: &TokenContext,
        market_index: &Serum3MarketIndex,
        market: &Serum3MarketContext,
        quote: &TokenContext,
    ) -> anyhow::Result<()> {
        let Ok(open_orders) = account.serum3_orders(*market_index).map(|x| x.open_orders)
        else {
            return Ok(());
        };

        let oo_acc = self.account_fetcher.fetch_raw(&open_orders)?;
        let oo = serum3_cpi::load_open_orders_bytes(oo_acc.data())?;
        let oo_slim = OpenOrdersSlim::from_oo(oo);

        if oo_slim.native_base_reserved() != 0 || oo_slim.native_quote_reserved() != 0 {
            return Ok(());
        }

        let settle_ixs =
            self.mango_client
                .serum3_settle_funds_instruction(market, token, quote, open_orders);

        let close_ixs = self
            .mango_client
            .serum3_close_open_orders_instruction(*market_index);

        let mut ixs = PreparedInstructions::new();
        ixs.append(settle_ixs);
        ixs.append(close_ixs);

        let txsig = self
            .mango_client
            .send_and_confirm_authority_tx(ixs.to_instructions())
            .await?;

        info!(
            %txsig,
            "settle spot funds for {}",
            token.name,
        );
        if !self.refresh_mango_account_after_tx(txsig).await? {
            return Ok(());
        }

        Ok(())
    }

    async fn unwind_using_swap(
        &self,
        account: &Box<MangoAccountValue>,
        token: &TokenContext,
        token_mint: Pubkey,
        token_price: I80F48,
        dust_threshold: FixedI128<U48>,
        fresh_amount: impl Fn() -> anyhow::Result<I80F48>,
        amount: I80F48,
    ) -> anyhow::Result<I80F48> {
        if amount < 0 {
            // Buy
            let buy_amount = amount.abs().ceil() + (dust_threshold - I80F48::ONE).max(I80F48::ZERO);
            let input_amount =
                buy_amount * token_price * I80F48::from_num(self.config.borrow_settle_excess);
            let (txsig, route) = self
                .token_swap_buy(&account, token_mint, input_amount.to_num())
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
                return Ok(amount);
            }
        }

        if amount > dust_threshold {
            // Sell
            let (txsig, route) = self
                .token_swap_sell(&account, token_mint, amount.to_num::<u64>())
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
                return Ok(amount);
            }
        }

        Ok(fresh_amount()?)
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

            let mut ixs = self
                .mango_client
                .perp_place_order_instruction(
                    account,
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

            let seq_check_ix = self
                .mango_client
                .sequence_check_instruction(&self.mango_account_address, account)
                .await?;
            ixs.append(seq_check_ix);

            let tx_builder = TransactionBuilder {
                instructions: ixs.to_instructions(),
                signers: vec![self.mango_client.authority.clone()],
                ..self.mango_client.transaction_builder().await?
            };

            let txsig = tx_builder
                .send_and_confirm(&self.mango_client.client)
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

        for perp_position in Self::shuffle(account.active_perp_positions()) {
            let perp = self.mango_client.context.perp(perp_position.market_index);
            if !self.rebalance_perp(&account, perp, perp_position).await? {
                return Ok(());
            }
        }

        Ok(())
    }

    fn shuffle<T>(iterator: impl Iterator<Item = T>) -> Vec<T> {
        use rand::seq::SliceRandom;

        let mut result = iterator.collect::<Vec<T>>();
        {
            let mut rng = rand::thread_rng();
            result.shuffle(&mut rng);
        }

        result
    }

    fn can_swap_on_sanctum(&self, mint: Pubkey) -> bool {
        self.sanctum_supported_mints.contains(&mint)
    }

    pub async fn init(&mut self, live_rpc_client: &RpcClient) {
        match swap::sanctum::load_supported_token_mints(live_rpc_client).await {
            Err(e) => warn!("Could not load list of sanctum supported mint: {}", e),
            Ok(mint) => self.sanctum_supported_mints.extend(mint),
        }
    }
}
