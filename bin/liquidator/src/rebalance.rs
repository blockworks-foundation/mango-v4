use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::state::{
    Bank, BookSide, PlaceOrderType, Side, TokenIndex, TokenPosition, QUOTE_TOKEN_INDEX,
};
use mango_v4_client::{
    chain_data, perp_pnl, AccountFetcher, AnyhowWrap, JupiterSwapMode, MangoClient, TokenContext,
};

use {fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

use futures::{stream, StreamExt, TryStreamExt};
use solana_sdk::signature::Signature;
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

#[derive(Clone)]
pub struct Config {
    /// Maximum slippage allowed in Jupiter
    pub slippage_bps: u64,
    /// When closing borrows, the rebalancer can't close token positions exactly.
    /// Instead it purchases too much and then gets rid of the excess in a second step.
    /// If this is 1.05, then it'll swap borrow_value * 1.05 quote token into borrow token.
    pub borrow_settle_excess: f64,
    pub refresh_timeout: Duration,
}

#[derive(Debug)]
struct TokenState {
    price: I80F48,
    native_position: I80F48,
}

impl TokenState {
    async fn new_position(
        token: &TokenContext,
        position: &TokenPosition,
        account_fetcher: &chain_data::AccountFetcher,
    ) -> anyhow::Result<Self> {
        let bank = Self::bank(token, account_fetcher)?;
        Ok(Self {
            price: Self::fetch_price(token, &bank, account_fetcher).await?,
            native_position: position.native(&bank),
        })
    }

    fn bank(
        token: &TokenContext,
        account_fetcher: &chain_data::AccountFetcher,
    ) -> anyhow::Result<Bank> {
        account_fetcher.fetch::<Bank>(&token.mint_info.first_bank())
    }

    async fn fetch_price(
        token: &TokenContext,
        bank: &Bank,
        account_fetcher: &chain_data::AccountFetcher,
    ) -> anyhow::Result<I80F48> {
        let oracle = account_fetcher
            .fetch_raw_account(&token.mint_info.oracle)
            .await?;
        bank.oracle_price(
            &KeyedAccountSharedData::new(token.mint_info.oracle, oracle.into()),
            None,
        )
        .map_err_anyhow()
    }
}

pub struct Rebalancer {
    pub mango_client: Arc<MangoClient>,
    pub account_fetcher: Arc<chain_data::AccountFetcher>,
    pub mango_account_address: Pubkey,
    pub config: Config,
}

impl Rebalancer {
    pub async fn zero_all_non_quote(&self) -> anyhow::Result<()> {
        log::trace!("checking for rebalance: {}", self.mango_account_address);

        self.rebalance_tokens().await?;
        self.rebalance_perps().await?;

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
            log::info!("could not refresh account data: {}", e);
            return Ok(false);
        }
        Ok(true)
    }

    async fn rebalance_tokens(&self) -> anyhow::Result<()> {
        let account = self
            .account_fetcher
            .fetch_mango_account(&self.mango_account_address)?;

        // TODO: configurable?
        let quote_token = self.mango_client.context.token(QUOTE_TOKEN_INDEX);

        let tokens: anyhow::Result<HashMap<TokenIndex, TokenState>> =
            stream::iter(account.active_token_positions())
                .then(|token_position| async {
                    let token = self.mango_client.context.token(token_position.token_index);
                    Ok((
                        token.token_index,
                        TokenState::new_position(token, token_position, &self.account_fetcher)
                            .await?,
                    ))
                })
                .try_collect()
                .await;
        let tokens = tokens?;
        log::trace!("account tokens: {:?}", tokens);

        for (token_index, token_state) in tokens {
            let token = self.mango_client.context.token(token_index);
            if token_index == quote_token.token_index {
                continue;
            }
            let token_mint = token.mint_info.mint;
            let quote_mint = quote_token.mint_info.mint;

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
            let dust_threshold = I80F48::from(2) / token_state.price;

            let mut amount = token_state.native_position;

            if amount < 0 {
                // Buy
                let buy_amount =
                    amount.abs().ceil() + (dust_threshold - I80F48::ONE).max(I80F48::ZERO);
                let input_amount = buy_amount
                    * token_state.price
                    * I80F48::from_num(self.config.borrow_settle_excess);
                let txsig = self
                    .mango_client
                    .jupiter_swap(
                        quote_mint,
                        token_mint,
                        input_amount.to_num::<u64>(),
                        self.config.slippage_bps,
                        JupiterSwapMode::ExactIn,
                    )
                    .await?;
                log::info!(
                    "bought {} {} for {} in tx {}",
                    token.native_to_ui(buy_amount),
                    token.name,
                    quote_token.name,
                    txsig
                );
                if !self.refresh_mango_account_after_tx(txsig).await? {
                    return Ok(());
                }
                let bank = TokenState::bank(token, &self.account_fetcher)?;
                amount = self
                    .mango_client
                    .mango_account()
                    .await?
                    .token_position_and_raw_index(token_index)
                    .map(|(position, _)| position.native(&bank))
                    .unwrap_or(I80F48::ZERO);
            }

            if amount > dust_threshold {
                // Sell
                let txsig = self
                    .mango_client
                    .jupiter_swap(
                        token_mint,
                        quote_mint,
                        amount.to_num::<u64>(),
                        self.config.slippage_bps,
                        JupiterSwapMode::ExactIn,
                    )
                    .await?;
                log::info!(
                    "sold {} {} for {} in tx {}",
                    token.native_to_ui(amount),
                    token.name,
                    quote_token.name,
                    txsig
                );
                if !self.refresh_mango_account_after_tx(txsig).await? {
                    return Ok(());
                }
                let bank = TokenState::bank(token, &self.account_fetcher)?;
                amount = self
                    .mango_client
                    .mango_account()
                    .await?
                    .token_position_and_raw_index(token_index)
                    .map(|(position, _)| position.native(&bank))
                    .unwrap_or(I80F48::ZERO);
            }

            // Any remainder that could not be sold just gets withdrawn to ensure the
            // TokenPosition is freed up
            if amount > 0 && amount <= dust_threshold {
                let allow_borrow = false;
                let txsig = self
                    .mango_client
                    .token_withdraw(token_mint, amount.to_num::<u64>(), allow_borrow)
                    .await?;
                log::info!(
                    "withdrew {} {} to liqor wallet in {}",
                    token.native_to_ui(amount),
                    token.name,
                    txsig
                );
                if !self.refresh_mango_account_after_tx(txsig).await? {
                    return Ok(());
                }
            } else if amount > dust_threshold {
                anyhow::bail!(
                    "unexpected {} position after rebalance swap: {} native",
                    token.name,
                    amount
                );
            }
        }

        Ok(())
    }

    async fn rebalance_perps(&self) -> anyhow::Result<()> {
        let now_ts: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
            .try_into()?;
        let account = self
            .account_fetcher
            .fetch_mango_account(&self.mango_account_address)?;

        for perp_position in account.active_perp_positions() {
            let perp = self.mango_client.context.perp(perp_position.market_index);
            let base_lots = perp_position.base_position_lots();
            let effective_lots = perp_position.effective_base_position_lots();
            let quote_native = perp_position.quote_position_native();
            log::info!(
                "active perp position on {}, base lots: {}, effective lots: {}, quote native: {}",
                perp.market.name(),
                base_lots,
                effective_lots,
                quote_native,
            );

            if effective_lots != 0 {
                // send an ioc order to reduce the base position
                let oracle_account_data = self.account_fetcher.fetch_raw(&perp.market.oracle)?;
                let oracle_account =
                    KeyedAccountSharedData::new(perp.market.oracle, oracle_account_data);
                let oracle_price = perp.market.oracle_price(&oracle_account, None)?;
                let oracle_price_lots = perp.market.native_price_to_lot(oracle_price);
                let (side, order_price, oo_lots) = if effective_lots > 0 {
                    (
                        Side::Ask,
                        oracle_price * (I80F48::ONE - perp.market.base_liquidation_fee),
                        perp_position.asks_base_lots,
                    )
                } else {
                    (
                        Side::Bid,
                        oracle_price * (I80F48::ONE + perp.market.base_liquidation_fee),
                        perp_position.bids_base_lots,
                    )
                };
                let price_lots = perp.market.native_price_to_lot(order_price);
                let max_base_lots = effective_lots.abs() - oo_lots;
                if max_base_lots <= 0 {
                    log::warn!(
                        "cannot place reduce-only order on {} {:?}, base pos: {}, in open orders: {}",
                        perp.market.name(),
                        side,
                        effective_lots,
                        oo_lots,
                    );
                    continue;
                }

                // Check the orderbook before sending the ioc order to see if we could
                // even match anything. That way we don't need to pay the tx fee and
                // ioc penalty fee unnecessarily.
                let opposite_side_key = match side.invert_side() {
                    Side::Bid => perp.market.bids,
                    Side::Ask => perp.market.asks,
                };
                let bookside = self.account_fetcher.fetch::<BookSide>(&opposite_side_key)?;
                if bookside.quantity_at_price(price_lots, now_ts, oracle_price_lots) <= 0 {
                    log::warn!(
                        "no liquidity on {} {:?} at price {}, oracle price {}",
                        perp.market.name(),
                        side.invert_side(),
                        order_price,
                        oracle_price,
                    );
                    continue;
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
                    )
                    .await?;
                log::info!(
                    "attempt to ioc reduce perp base position of {} {} at price {} in {}",
                    perp_position.base_position_native(&perp.market),
                    perp.market.name(),
                    order_price,
                    txsig
                );
                if !self.refresh_mango_account_after_tx(txsig).await? {
                    return Ok(());
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
                    self.account_fetcher.as_ref(),
                    perp_position.market_index,
                    direction,
                    2,
                )
                .await?;
                if counters.is_empty() {
                    // If we can't settle some positive PNL because we're lacking a suitable counterparty,
                    // then liquidation should continue, even though this step produced no transaction
                    log::info!(
                        "could not settle perp pnl on perp market {}: no counterparty",
                        perp.market.name()
                    );
                    continue;
                }
                let (counter_key, counter_acc, _counter_pnl) = counters.first().unwrap();

                let (account_a, account_b) = if quote_native > 0 {
                    (
                        (&self.mango_account_address, &account),
                        (counter_key, counter_acc),
                    )
                } else {
                    (
                        (counter_key, counter_acc),
                        (&self.mango_account_address, &account),
                    )
                };
                let txsig = self
                    .mango_client
                    .perp_settle_pnl(perp_position.market_index, account_a, account_b)
                    .await?;
                log::info!("settled perp {} pnl, tx sig {}", perp.market.name(), txsig);
                if !self.refresh_mango_account_after_tx(txsig).await? {
                    return Ok(());
                }
            } else if base_lots == 0 && quote_native == 0 {
                // close perp position
                let txsig = self
                    .mango_client
                    .perp_deactivate_position(perp_position.market_index)
                    .await?;
                log::info!(
                    "closed perp position on {} in {}",
                    perp.market.name(),
                    txsig
                );
                if !self.refresh_mango_account_after_tx(txsig).await? {
                    return Ok(());
                }
            } else {
                // maybe we're still waiting for consume_events
                log::info!(
                    "cannot deactivate perp {} position, base lots {}, effective lots {}, quote {}",
                    perp.market.name(),
                    perp_position.base_position_lots(),
                    effective_lots,
                    perp_position.quote_position_native()
                );
            }
        }

        Ok(())
    }
}
