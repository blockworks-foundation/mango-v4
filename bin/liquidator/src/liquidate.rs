use std::collections::HashSet;
use std::time::Duration;

use mango_v4::health::{HealthCache, HealthType};
use mango_v4::state::{
    MangoAccountValue, PerpMarketIndex, Side, TokenConditionalSwap, TokenIndex, QUOTE_TOKEN_INDEX,
};
use mango_v4_client::{chain_data, health_cache, AccountFetcher, JupiterSwapMode, MangoClient};
use solana_sdk::signature::Signature;

use futures::{stream, StreamExt, TryStreamExt};
use rand::seq::SliceRandom;
use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

pub struct Config {
    pub liq_min_health_ratio: f64,
    pub tcs_min_health_ratio: f64,
    pub refresh_timeout: Duration,
    pub mock_jupiter: bool,
}

async fn jupiter_route(
    mango_client: &MangoClient,
    input_mint: Pubkey,
    output_mint: Pubkey,
    amount: u64,
    slippage: u64,
    swap_mode: JupiterSwapMode,
    config: &Config,
) -> anyhow::Result<mango_v4_client::jupiter::QueryRoute> {
    if !config.mock_jupiter {
        return mango_client
            .jupiter_route(input_mint, output_mint, amount, slippage, swap_mode)
            .await;
    }

    let input_price = mango_client
        .bank_oracle_price(mango_client.context.token_by_mint(&input_mint)?.token_index)
        .await?;
    let output_price = mango_client
        .bank_oracle_price(
            mango_client
                .context
                .token_by_mint(&output_mint)?
                .token_index,
        )
        .await?;
    let in_amount: u64;
    let out_amount: u64;
    let other_amount_threshold: u64;
    let swap_mode_str;
    match swap_mode {
        JupiterSwapMode::ExactIn => {
            in_amount = amount;
            out_amount = (I80F48::from(amount) * input_price / output_price).to_num();
            other_amount_threshold = out_amount;
            swap_mode_str = "ExactIn".to_string();
        }
        JupiterSwapMode::ExactOut => {
            in_amount = (I80F48::from(amount) * output_price / input_price).to_num();
            out_amount = amount;
            other_amount_threshold = in_amount;
            swap_mode_str = "ExactOut".to_string();
        }
    }

    Ok(mango_v4_client::jupiter::QueryRoute {
        in_amount: in_amount.to_string(),
        out_amount: out_amount.to_string(),
        price_impact_pct: 0.1,
        market_infos: vec![],
        amount: amount.to_string(),
        slippage_bps: 1,
        other_amount_threshold: other_amount_threshold.to_string(),
        swap_mode: swap_mode_str,
        fees: None,
    })
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
                let price = self.client.perp_oracle_price(pp.market_index).await?;
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
                    let token_index = token_position.token_index;
                    let price = self.client.bank_oracle_price(token_index).await?;
                    let bank = self.client.first_bank(token_index).await?;
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
        let liqor = self
            .account_fetcher
            .fetch_fresh_mango_account(&self.client.mango_account_address)
            .await
            .context("getting liquidator account")?;

        let source_price = self.client.bank_oracle_price(source).await?;
        let target_price = self.client.bank_oracle_price(target).await?;

        // TODO: This is where we could multiply in the liquidation fee factors
        let price = source_price / target_price;

        Self::max_swap_source(
            self.client,
            self.account_fetcher,
            &liqor,
            source,
            target,
            price,
            self.liqor_min_health_ratio,
        )
        .await
    }

    async fn max_swap_source(
        client: &MangoClient,
        account_fetcher: &chain_data::AccountFetcher,
        account: &MangoAccountValue,
        source: TokenIndex,
        target: TokenIndex,
        price: I80F48,
        min_health_ratio: I80F48,
    ) -> anyhow::Result<I80F48> {
        let mut account = account.clone();

        // Ensure the tokens are activated, so they appear in the health cache and
        // max_swap_source() will work.
        account.ensure_token_position(source)?;
        account.ensure_token_position(target)?;

        let health_cache = health_cache::new(&client.context, account_fetcher, &account)
            .await
            .expect("always ok");

        let source_bank = client.first_bank(source).await?;
        let target_bank = client.first_bank(target).await?;

        let source_price = health_cache.token_info(source).unwrap().prices.oracle;

        let amount = health_cache
            .max_swap_source_for_health_ratio(
                &account,
                &source_bank,
                source_price,
                &target_bank,
                price,
                min_health_ratio,
            )
            .context("getting max_swap_source")?;
        Ok(amount)
    }

    async fn token_liq(&self) -> anyhow::Result<Option<Signature>> {
        if !self.health_cache.has_possible_spot_liquidations() {
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
        if !self.health_cache.in_phase3_liquidation() || !self.health_cache.has_liq_spot_borrows() {
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
    let liqor_min_health_ratio = I80F48::from_num(config.liq_min_health_ratio);

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

    // TODO: requirements on premium
    if tcs.price_premium_bps < 100 {
        return Ok(false);
    }

    return Ok(true);
}

#[allow(clippy::too_many_arguments)]
pub async fn maybe_execute_token_conditional_swap(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
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
            if tcs_is_executable(mango_client, tcs).await? {
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
    if !tcs_is_executable(mango_client, tcs).await? {
        return Ok(false);
    }

    let health_cache = health_cache::new(&mango_client.context, account_fetcher, &liqee)
        .await
        .context("creating health cache 1")?;
    if health_cache.is_liquidatable() {
        return Ok(false);
    }

    // TODO: if it's expired, just trigger it to close it?

    let liqor_min_health_ratio = I80F48::from_num(config.tcs_min_health_ratio);

    // Compute the max viable swap (for liqor and liqee) and min it
    let buy_token_price = mango_client.bank_oracle_price(tcs.buy_token_index).await?;
    let sell_token_price = mango_client.bank_oracle_price(tcs.sell_token_index).await?;

    let base_price = buy_token_price / sell_token_price;
    let premium_price = tcs.premium_price(base_price.to_num());
    let maker_price = I80F48::from_num(tcs.maker_price(premium_price));
    let taker_price = I80F48::from_num(tcs.taker_price(premium_price));

    // TODO: configurable
    let max_take_quote = I80F48::from(1_000_000_000);

    let max_sell_token_to_liqor = LiquidateHelper::max_swap_source(
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

    let max_buy_token_to_liqee = LiquidateHelper::max_swap_source(
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
    // TODO: doing this every time is hugely expensive, there needs to be a layer
    // in front, that rejects nonsensical tcs based on cached slippage values.
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
        let route = jupiter_route(
            mango_client,
            sell_mint,
            buy_mint,
            input_amount,
            slippage,
            swap_mode,
            config,
        )
        .await?;
        log::info!("tcs pre execution jupiter query: {:#?}", route);

        // TODO: check if the output_amount is large enough
        // TODO: technically, we could just execute this at the same time?
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
