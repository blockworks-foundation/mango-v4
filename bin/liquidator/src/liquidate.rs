use std::collections::HashSet;
use std::time::Duration;

use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::health::{HealthCache, HealthType};
use mango_v4::state::{
    Bank, MangoAccountValue, PerpMarketIndex, Side, TokenIndex, QUOTE_TOKEN_INDEX,
};
use mango_v4_client::{chain_data, health_cache, AccountFetcher, JupiterSwapMode, MangoClient};
use solana_sdk::signature::Signature;

use futures::{stream, StreamExt, TryStreamExt};
use rand::seq::SliceRandom;
use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

pub struct Config {
    pub min_health_ratio: f64,
    pub refresh_timeout: Duration,
}

pub async fn jupiter_market_can_buy(
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
    let slippage = 100;
    mango_client
        .jupiter_route(
            quote_token_mint,
            token_mint,
            quote_amount,
            slippage,
            JupiterSwapMode::ExactIn,
        )
        .await
        .is_ok()
}

pub async fn jupiter_market_can_sell(
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
    let slippage = 100;
    mango_client
        .jupiter_route(
            token_mint,
            quote_token_mint,
            quote_amount,
            slippage,
            JupiterSwapMode::ExactOut,
        )
        .await
        .is_ok()
}

struct LiquidateHelper<'a> {
    client: &'a MangoClient,
    account_fetcher: &'a chain_data::AccountFetcher,
    pubkey: &'a Pubkey,
    liqee: &'a MangoAccountValue,
    health_cache: &'a HealthCache,
    maint_health: I80F48,
    liqor_min_health_ratio: I80F48,
    allowed_asset_tokens: HashSet<Pubkey>,
    allowed_liab_tokens: HashSet<Pubkey>,
}

impl<'a> LiquidateHelper<'a> {
    async fn serum3_close_orders(&self) -> anyhow::Result<Option<Signature>> {
        // look for any open serum orders or settleable balances
        let serum_oos: anyhow::Result<Vec<_>> = stream::iter(self.liqee.active_serum3_orders())
            .then(|orders| async {
                let open_orders_account = self
                    .account_fetcher
                    .fetch_raw_account(&orders.open_orders)
                    .await?;
                let open_orders = mango_v4::serum3_cpi::load_open_orders(&open_orders_account)?;
                Ok((*orders, *open_orders))
            })
            .try_collect()
            .await;
        let serum_force_cancels = serum_oos?
            .into_iter()
            .filter_map(|(orders, open_orders)| {
                let can_force_cancel = open_orders.native_coin_total > 0
                    || open_orders.native_pc_total > 0
                    || open_orders.referrer_rebates_accrued > 0;
                if can_force_cancel {
                    Some(orders)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if serum_force_cancels.is_empty() {
            return Ok(None);
        }
        // Cancel all orders on a random serum market
        let serum_orders = serum_force_cancels.choose(&mut rand::thread_rng()).unwrap();
        let sig = self
            .client
            .serum3_liq_force_cancel_orders(
                (self.pubkey, &self.liqee),
                serum_orders.market_index,
                &serum_orders.open_orders,
            )
            .await?;
        log::info!(
            "Force cancelled serum orders on account {}, market index {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            serum_orders.market_index,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    async fn perp_close_orders(&self) -> anyhow::Result<Option<Signature>> {
        let perp_force_cancels = self
            .liqee
            .active_perp_positions()
            .filter_map(|pp| pp.has_open_orders().then(|| pp.market_index))
            .collect::<Vec<PerpMarketIndex>>();
        if perp_force_cancels.is_empty() {
            return Ok(None);
        }

        // Cancel all orders on a random perp market
        let perp_market_index = *perp_force_cancels.choose(&mut rand::thread_rng()).unwrap();
        let sig = self
            .client
            .perp_liq_force_cancel_orders((self.pubkey, &self.liqee), perp_market_index)
            .await?;
        log::info!(
            "Force cancelled perp orders on account {}, market index {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            perp_market_index,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    async fn perp_liq_base_or_positive_pnl(&self) -> anyhow::Result<Option<Signature>> {
        let all_perp_base_positions: anyhow::Result<
            Vec<Option<(PerpMarketIndex, i64, I80F48, I80F48)>>,
        > = stream::iter(self.liqee.active_perp_positions())
            .then(|pp| async {
                let base_lots = pp.base_position_lots();
                if (base_lots == 0 && pp.quote_position_native() <= 0) || pp.has_open_taker_fills()
                {
                    return Ok(None);
                }
                let perp = self.client.context.perp(pp.market_index);
                let oracle = self
                    .account_fetcher
                    .fetch_raw_account(&perp.market.oracle)
                    .await?;
                let price = perp.market.oracle_price(
                    &KeyedAccountSharedData::new(perp.market.oracle, oracle.into()),
                    None,
                )?;
                Ok(Some((
                    pp.market_index,
                    base_lots,
                    price,
                    I80F48::from(base_lots.abs()) * price,
                )))
            })
            .try_collect()
            .await;
        let mut perp_base_positions = all_perp_base_positions?
            .into_iter()
            .filter_map(|x| x)
            .collect::<Vec<_>>();
        perp_base_positions.sort_by(|a, b| a.3.cmp(&b.3));

        if perp_base_positions.is_empty() {
            return Ok(None);
        }

        // Liquidate the highest-value perp base position
        let (perp_market_index, base_lots, price, _) = perp_base_positions.last().unwrap();
        let perp = self.client.context.perp(*perp_market_index);

        let (side, side_signum) = if *base_lots > 0 {
            (Side::Bid, 1)
        } else {
            (Side::Ask, -1)
        };

        // Compute the max number of base_lots and positive pnl the liqor is willing to take
        // TODO: This is risky for the liqor. It should track how much pnl is usually settleable
        // in the market before agreeding to take it over. Also, the liqor should check how much
        // settle limit it's going to get along with the unsettled pnl.
        let (max_base_transfer_abs, max_pnl_transfer) = {
            let mut liqor = self
                .account_fetcher
                .fetch_fresh_mango_account(&self.client.mango_account_address)
                .await
                .context("getting liquidator account")?;
            liqor.ensure_perp_position(*perp_market_index, QUOTE_TOKEN_INDEX)?;
            let mut health_cache =
                health_cache::new(&self.client.context, self.account_fetcher, &liqor)
                    .await
                    .expect("always ok");
            let quote_bank = self
                .client
                .first_bank(QUOTE_TOKEN_INDEX)
                .await
                .context("getting quote bank")?;
            let max_usdc_borrow = health_cache.max_borrow_for_health_ratio(
                &liqor,
                &quote_bank,
                self.liqor_min_health_ratio,
            )?;
            // Ideally we'd predict how much positive pnl we're going to take over and then allocate
            // the base and quote amount accordingly. This just goes with allocating a fraction of the
            // available amount to quote and the rest to base.
            let allowed_usdc_borrow = I80F48::from_num(0.25) * max_usdc_borrow;
            // Perp overall asset weights > 0 mean that we get some health back for every unit of unsettled pnl
            // and hence we can take over more than the pure-borrow amount.
            let max_perp_unsettled_leverage = I80F48::from_num(0.95);
            let perp_unsettled_cost = I80F48::ONE
                - perp
                    .market
                    .init_overall_asset_weight
                    .min(max_perp_unsettled_leverage);
            let max_pnl_transfer = allowed_usdc_borrow / perp_unsettled_cost;

            // Update the health cache so we can determine how many base lots the liqor can take on,
            // assuming that the max_quote_transfer amount of positive unsettled pnl was taken over.
            health_cache.adjust_token_balance(&quote_bank, -allowed_usdc_borrow)?;

            let max_base_transfer = health_cache.max_perp_for_health_ratio(
                *perp_market_index,
                *price,
                side,
                self.liqor_min_health_ratio,
            )?;

            (max_base_transfer, max_pnl_transfer.floor().to_num::<u64>())
        };
        log::info!("computed max_base_transfer: {max_base_transfer_abs}, max_pnl_transfer: {max_pnl_transfer}");

        let sig = self
            .client
            .perp_liq_base_or_positive_pnl(
                (self.pubkey, &self.liqee),
                *perp_market_index,
                side_signum * max_base_transfer_abs,
                max_pnl_transfer,
            )
            .await?;
        log::info!(
            "Liquidated base position for perp market on account {}, market index {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            perp_market_index,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    /*
    async fn perp_settle_pnl(&self) -> anyhow::Result<Option<Signature>> {
        let perp_settle_health = self.health_cache.perp_settle_health();
        let mut perp_settleable_pnl = self
            .liqee
            .active_perp_positions()
            .filter_map(|pp| {
                if pp.base_position_lots() != 0 {
                    return None;
                }
                let pnl = pp.quote_position_native();
                // TODO: outdated: must account for perp settle limit
                let settleable_pnl = if pnl > 0 {
                    pnl
                } else if pnl < 0 && perp_settle_health > 0 {
                    pnl.max(-perp_settle_health)
                } else {
                    return None;
                };
                if settleable_pnl.abs() < 1 {
                    return None;
                }
                Some((pp.market_index, settleable_pnl))
            })
            .collect::<Vec<(PerpMarketIndex, I80F48)>>();
        // sort by pnl, descending
        perp_settleable_pnl.sort_by(|a, b| b.1.cmp(&a.1));

        if perp_settleable_pnl.is_empty() {
            return Ok(None);
        }

        for (perp_index, pnl) in perp_settleable_pnl {
            let direction = if pnl > 0 {
                client::perp_pnl::Direction::MaxNegative
            } else {
                client::perp_pnl::Direction::MaxPositive
            };
            let counters = client::perp_pnl::fetch_top(
                &self.client.context,
                self.account_fetcher,
                perp_index,
                direction,
                2,
            )
            .await?;
            if counters.is_empty() {
                // If we can't settle some positive PNL because we're lacking a suitable counterparty,
                // then liquidation should continue, even though this step produced no transaction
                log::info!("Could not settle perp pnl {pnl} for account {}, perp market {perp_index}: no counterparty",
            self.pubkey);
                continue;
            }
            let (counter_key, counter_acc, counter_pnl) = counters.first().unwrap();

            log::info!("Trying to settle perp pnl account: {} market_index: {perp_index} amount: {pnl} against {counter_key} with pnl: {counter_pnl}", self.pubkey);

            let (account_a, account_b) = if pnl > 0 {
                ((self.pubkey, self.liqee), (counter_key, counter_acc))
            } else {
                ((counter_key, counter_acc), (self.pubkey, self.liqee))
            };
            let sig = self
                .client
                .perp_settle_pnl(perp_index, account_a, account_b)
                .await?;
            log::info!(
                "Settled perp pnl for perp market on account {}, market index {perp_index}, maint_health was {}, tx sig {sig:?}",
                self.pubkey,
                self.maint_health,
            );
            return Ok(Some(sig));
        }
        return Ok(None);
    }
    */

    async fn perp_liq_negative_pnl_or_bankruptcy(&self) -> anyhow::Result<Option<Signature>> {
        if !self.health_cache.in_phase3_liquidation() {
            return Ok(None);
        }
        let mut perp_negative_pnl = self
            .liqee
            .active_perp_positions()
            .filter_map(|pp| {
                let quote = pp.quote_position_native();
                if quote >= 0 {
                    return None;
                }
                Some((pp.market_index, quote))
            })
            .collect::<Vec<(PerpMarketIndex, I80F48)>>();
        perp_negative_pnl.sort_by(|a, b| a.1.cmp(&b.1));

        if perp_negative_pnl.is_empty() {
            return Ok(None);
        }
        let (perp_market_index, _) = perp_negative_pnl.first().unwrap();

        let sig = self
            .client
            .perp_liq_negative_pnl_or_bankruptcy(
                (self.pubkey, &self.liqee),
                *perp_market_index,
                // Always use the max amount, since the health effect is >= 0
                u64::MAX,
            )
            .await?;
        log::info!(
            "Liquidated negative perp pnl on account {}, market index {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            perp_market_index,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    async fn tokens(&self) -> anyhow::Result<Vec<(TokenIndex, I80F48, I80F48)>> {
        let tokens_maybe: anyhow::Result<Vec<(TokenIndex, I80F48, I80F48)>> =
            stream::iter(self.liqee.active_token_positions())
                .then(|token_position| async {
                    let token = self.client.context.token(token_position.token_index);
                    let bank = self
                        .account_fetcher
                        .fetch::<Bank>(&token.mint_info.first_bank())?;
                    let oracle = self
                        .account_fetcher
                        .fetch_raw_account(&token.mint_info.oracle)
                        .await?;
                    let price = bank.oracle_price(
                        &KeyedAccountSharedData::new(token.mint_info.oracle, oracle.into()),
                        None,
                    )?;
                    Ok((
                        token_position.token_index,
                        price,
                        token_position.native(&bank) * price,
                    ))
                })
                .try_collect()
                .await;
        let mut tokens = tokens_maybe?;
        tokens.sort_by(|a, b| a.2.cmp(&b.2));
        Ok(tokens)
    }

    async fn max_token_liab_transfer(
        &self,
        source: TokenIndex,
        target: TokenIndex,
    ) -> anyhow::Result<I80F48> {
        let mut liqor = self
            .account_fetcher
            .fetch_fresh_mango_account(&self.client.mango_account_address)
            .await
            .context("getting liquidator account")?;

        // Ensure the tokens are activated, so they appear in the health cache and
        // max_swap_source() will work.
        liqor.ensure_token_position(source)?;
        liqor.ensure_token_position(target)?;

        let health_cache = health_cache::new(&self.client.context, self.account_fetcher, &liqor)
            .await
            .expect("always ok");

        let source_bank = self.client.first_bank(source).await?;
        let target_bank = self.client.first_bank(target).await?;

        let source_price = health_cache.token_info(source).unwrap().prices.oracle;
        let target_price = health_cache.token_info(target).unwrap().prices.oracle;
        // TODO: This is where we could multiply in the liquidation fee factors
        let oracle_swap_price = source_price / target_price;

        let amount = health_cache
            .max_swap_source_for_health_ratio(
                &liqor,
                &source_bank,
                source_price,
                &target_bank,
                oracle_swap_price,
                self.liqor_min_health_ratio,
            )
            .context("getting max_swap_source")?;
        Ok(amount)
    }

    async fn token_liq(&self) -> anyhow::Result<Option<Signature>> {
        if !self.health_cache.has_spot_assets() || !self.health_cache.has_spot_borrows() {
            return Ok(None);
        }

        let tokens = self.tokens().await?;

        let asset_token_index = tokens
            .iter()
            .rev()
            .find(|(asset_token_index, _asset_price, asset_usdc_equivalent)| {
                asset_usdc_equivalent.is_positive()
                    && self
                        .allowed_asset_tokens
                        .contains(&self.client.context.token(*asset_token_index).mint_info.mint)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "mango account {}, has no asset tokens that are sellable for USDC: {:?}",
                    self.pubkey,
                    tokens
                )
            })?
            .0;
        let liab_token_index = tokens
            .iter()
            .find(|(liab_token_index, _liab_price, liab_usdc_equivalent)| {
                liab_usdc_equivalent.is_negative()
                    && self
                        .allowed_liab_tokens
                        .contains(&self.client.context.token(*liab_token_index).mint_info.mint)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "mango account {}, has no liab tokens that are purchasable for USDC: {:?}",
                    self.pubkey,
                    tokens
                )
            })?
            .0;

        let max_liab_transfer = self
            .max_token_liab_transfer(liab_token_index, asset_token_index)
            .await
            .context("getting max_liab_transfer")?;

        //
        // TODO: log liqor's assets in UI form
        // TODO: log liquee's liab_needed, need to refactor program code to be able to be accessed from client side
        //
        let sig = self
            .client
            .token_liq_with_token(
                (self.pubkey, &self.liqee),
                asset_token_index,
                liab_token_index,
                max_liab_transfer,
            )
            .await
            .context("sending liq_token_with_token")?;
        log::info!(
            "Liquidated token with token for {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    async fn token_liq_bankruptcy(&self) -> anyhow::Result<Option<Signature>> {
        if !self.health_cache.in_phase3_liquidation() || !self.health_cache.has_spot_borrows() {
            return Ok(None);
        }

        let tokens = self.tokens().await?;

        if tokens.is_empty() {
            anyhow::bail!(
                "mango account {}, is bankrupt has no active tokens",
                self.pubkey
            );
        }
        let liab_token_index = tokens
            .iter()
            .find(|(liab_token_index, _liab_price, liab_usdc_equivalent)| {
                liab_usdc_equivalent.is_negative()
                    && self
                        .allowed_liab_tokens
                        .contains(&self.client.context.token(*liab_token_index).mint_info.mint)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "mango account {}, has no liab tokens that are purchasable for USDC: {:?}",
                    self.pubkey,
                    tokens
                )
            })?
            .0;

        let quote_token_index = 0;
        let max_liab_transfer = self
            .max_token_liab_transfer(liab_token_index, quote_token_index)
            .await?;

        let sig = self
            .client
            .token_liq_bankruptcy(
                (self.pubkey, &self.liqee),
                liab_token_index,
                max_liab_transfer,
            )
            .await
            .context("sending liq_token_bankruptcy")?;
        log::info!(
            "Liquidated bankruptcy for {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    async fn send_liq_tx(&self) -> anyhow::Result<Option<Signature>> {
        // TODO: Should we make an attempt to settle positive PNL first?
        // The problem with it is that small market movements can continuously create
        // small amounts of new positive PNL while base_position > 0.
        // We shouldn't get stuck on this step, particularly if it's of limited value
        // to the liquidators.
        // if let Some(txsig) = self.perp_settle_positive_pnl()? {
        //     return Ok(txsig);
        // }

        //
        // Phase 1: Try to close orders before touching the user's positions
        //
        // TODO: All these close ix could be in one transaction.
        if let Some(txsig) = self.perp_close_orders().await? {
            return Ok(Some(txsig));
        }
        if let Some(txsig) = self.serum3_close_orders().await? {
            return Ok(Some(txsig));
        }

        if self.health_cache.has_phase1_liquidatable() {
            anyhow::bail!(
                "Don't know what to do with phase1 liquidatable account {}, maint_health was {}",
                self.pubkey,
                self.maint_health
            );
        }

        //
        // Phase 2: token, perp base, perp positive pnl
        //

        if let Some(txsig) = self.perp_liq_base_or_positive_pnl().await? {
            return Ok(Some(txsig));
        }

        if let Some(txsig) = self.token_liq().await? {
            return Ok(Some(txsig));
        }

        if self.health_cache.has_perp_open_fills() {
            log::info!(
                "Account {} has open perp fills, maint_health {}, waiting...",
                self.pubkey,
                self.maint_health
            );
            return Ok(None);
        }

        if self.health_cache.has_phase2_liquidatable() {
            anyhow::bail!(
                "Don't know what to do with phase2 liquidatable account {}, maint_health was {}",
                self.pubkey,
                self.maint_health
            );
        }

        //
        // Phase 3: perp and token bankruptcy
        //

        // Negative pnl: take over (paid by liqee or insurance) or socialize the loss
        if let Some(txsig) = self.perp_liq_negative_pnl_or_bankruptcy().await? {
            return Ok(Some(txsig));
        }

        // Socialize/insurance fund unliquidatable borrows
        if let Some(txsig) = self.token_liq_bankruptcy().await? {
            return Ok(Some(txsig));
        }

        // TODO: What about unliquidatable positive perp pnl?

        anyhow::bail!(
            "Don't know what to do with liquidatable account {}, maint_health was {}",
            self.pubkey,
            self.maint_health
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn maybe_liquidate_account(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    pubkey: &Pubkey,
    config: &Config,
) -> anyhow::Result<bool> {
    let liqor_min_health_ratio = I80F48::from_num(config.min_health_ratio);

    let account = account_fetcher.fetch_mango_account(pubkey)?;
    let health_cache = health_cache::new(&mango_client.context, account_fetcher, &account)
        .await
        .context("creating health cache 1")?;
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
    let account = account_fetcher.fetch_fresh_mango_account(pubkey).await?;
    let health_cache = health_cache::new(&mango_client.context, account_fetcher, &account)
        .await
        .context("creating health cache 2")?;
    if !health_cache.is_liquidatable() {
        return Ok(false);
    }

    let maint_health = health_cache.health(HealthType::Maint);

    let all_token_mints = HashSet::from_iter(
        mango_client
            .context
            .tokens
            .values()
            .map(|c| c.mint_info.mint),
    );

    // try liquidating
    let maybe_txsig = LiquidateHelper {
        client: mango_client,
        account_fetcher,
        pubkey,
        liqee: &account,
        health_cache: &health_cache,
        maint_health,
        liqor_min_health_ratio,
        allowed_asset_tokens: all_token_mints.clone(),
        allowed_liab_tokens: all_token_mints,
    }
    .send_liq_tx()
    .await?;

    if let Some(txsig) = maybe_txsig {
        let slot = account_fetcher.transaction_max_slot(&[txsig]).await?;
        if let Err(e) = account_fetcher
            .refresh_accounts_via_rpc_until_slot(
                &[*pubkey, mango_client.mango_account_address],
                slot,
                config.refresh_timeout,
            )
            .await
        {
            log::info!("could not refresh after liquidation: {}", e);
        }
    }

    Ok(true)
}
