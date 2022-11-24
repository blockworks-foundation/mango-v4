use std::time::Duration;

use client::{chain_data, health_cache, AccountFetcher, MangoClient, MangoClientError};
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::state::{
    Bank, HealthCache, HealthType, MangoAccountValue, PerpMarketIndex, Serum3Orders, Side,
    TokenIndex, QUOTE_TOKEN_INDEX,
};
use solana_sdk::signature::Signature;

use itertools::Itertools;
use rand::seq::SliceRandom;
use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

pub struct Config {
    pub min_health_ratio: f64,
    pub refresh_timeout: Duration,
}

pub fn jupiter_market_can_buy(
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
    let slippage = 1.0;
    mango_client
        .jupiter_route(
            quote_token_mint,
            token_mint,
            quote_amount,
            slippage,
            client::JupiterSwapMode::ExactIn,
        )
        .is_ok()
}

pub fn jupiter_market_can_sell(
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
    let slippage = 1.0;
    mango_client
        .jupiter_route(
            token_mint,
            quote_token_mint,
            quote_amount,
            slippage,
            client::JupiterSwapMode::ExactOut,
        )
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
}

impl<'a> LiquidateHelper<'a> {
    fn serum3_close_orders(&self) -> anyhow::Result<Option<Signature>> {
        // look for any open serum orders or settleable balances
        let serum_force_cancels = self
            .liqee
            .active_serum3_orders()
            .map(|orders| {
                let open_orders_account = self
                    .account_fetcher
                    .fetch_raw_account(&orders.open_orders)?;
                let open_orders = mango_v4::serum3_cpi::load_open_orders(&open_orders_account)?;
                let can_force_cancel = open_orders.native_coin_total > 0
                    || open_orders.native_pc_total > 0
                    || open_orders.referrer_rebates_accrued > 0;
                if can_force_cancel {
                    Ok(Some(*orders))
                } else {
                    Ok(None)
                }
            })
            .filter_map_ok(|v| v)
            .collect::<anyhow::Result<Vec<Serum3Orders>>>()?;
        if serum_force_cancels.is_empty() {
            return Ok(None);
        }
        // Cancel all orders on a random serum market
        let serum_orders = serum_force_cancels.choose(&mut rand::thread_rng()).unwrap();
        let sig = self.client.serum3_liq_force_cancel_orders(
            (self.pubkey, &self.liqee),
            serum_orders.market_index,
            &serum_orders.open_orders,
        )?;
        log::info!(
            "Force cancelled serum orders on account {}, market index {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            serum_orders.market_index,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    fn perp_close_orders(&self) -> anyhow::Result<Option<Signature>> {
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
            .perp_liq_force_cancel_orders((self.pubkey, &self.liqee), perp_market_index)?;
        log::info!(
            "Force cancelled perp orders on account {}, market index {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            perp_market_index,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    fn perp_liq_base_position(&self) -> anyhow::Result<Option<Signature>> {
        let mut perp_base_positions = self
            .liqee
            .active_perp_positions()
            .map(|pp| {
                let base_lots = pp.base_position_lots();
                if base_lots == 0 {
                    return Ok(None);
                }
                let perp = self.client.context.perp(pp.market_index);
                let oracle = self
                    .account_fetcher
                    .fetch_raw_account(&perp.market.oracle)?;
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
            .filter_map_ok(|v| v)
            .collect::<anyhow::Result<Vec<(PerpMarketIndex, i64, I80F48, I80F48)>>>()?;
        perp_base_positions.sort_by(|a, b| a.3.cmp(&b.3));

        if perp_base_positions.is_empty() {
            return Ok(None);
        }

        // Liquidate the highest-value perp base position
        let (perp_market_index, base_lots, price, _) = perp_base_positions.last().unwrap();

        let (side, side_signum) = if *base_lots > 0 {
            (Side::Bid, 1)
        } else {
            (Side::Ask, -1)
        };

        // Compute the max number of base_lots the liqor is willing to take
        let max_base_transfer_abs = {
            let mut liqor = self
                .account_fetcher
                .fetch_fresh_mango_account(&self.client.mango_account_address)
                .context("getting liquidator account")?;
            liqor.ensure_perp_position(*perp_market_index, QUOTE_TOKEN_INDEX)?;
            let health_cache =
                health_cache::new(&self.client.context, self.account_fetcher, &liqor)
                    .expect("always ok");
            health_cache.max_perp_for_health_ratio(
                *perp_market_index,
                *price,
                side,
                self.liqor_min_health_ratio,
            )?
        };
        log::info!("computed max_base_transfer to be {max_base_transfer_abs}");

        let sig = self.client.perp_liq_base_position(
            (self.pubkey, &self.liqee),
            *perp_market_index,
            side_signum * max_base_transfer_abs,
        )?;
        log::info!(
            "Liquidated base position for perp market on account {}, market index {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            perp_market_index,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    fn perp_settle_pnl(&self) -> anyhow::Result<Option<Signature>> {
        let perp_settle_health = self.health_cache.perp_settle_health();
        let mut perp_settleable_pnl = self
            .liqee
            .active_perp_positions()
            .filter_map(|pp| {
                if pp.base_position_lots() != 0 {
                    return None;
                }
                let pnl = pp.quote_position_native();
                let settleable_pnl = if pnl > 0 {
                    pnl
                } else if pnl < 0 && perp_settle_health > 0 {
                    pnl.max(-perp_settle_health)
                } else {
                    return None;
                };
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
            )?;
            if counters.is_empty() {
                // If we can't settle some positive PNL because we're lacking a suitable counterparty,
                // then liquidation should continue, even though this step produced no transaction
                log::info!("Could not settle perp pnl {pnl} for account {}, perp market {perp_index}: no counterparty",
            self.pubkey);
                continue;
            }
            let (counter_key, counter_acc, _) = counters.first().unwrap();

            let (account_a, account_b) = if pnl > 0 {
                ((self.pubkey, self.liqee), (counter_key, counter_acc))
            } else {
                ((counter_key, counter_acc), (self.pubkey, self.liqee))
            };
            let sig = self
                .client
                .perp_settle_pnl(perp_index, account_a, account_b)?;
            log::info!(
                "Settled perp pnl for perp market on account {}, market index {perp_index}, maint_health was {}, tx sig {sig:?}",
                self.pubkey,
                self.maint_health,
            );
            return Ok(Some(sig));
        }
        return Ok(None);
    }

    fn perp_liq_bankruptcy(&self) -> anyhow::Result<Option<Signature>> {
        if self.health_cache.has_liquidatable_assets() {
            return Ok(None);
        }
        let mut perp_bankruptcies = self
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
        perp_bankruptcies.sort_by(|a, b| a.1.cmp(&b.1));

        if perp_bankruptcies.is_empty() {
            return Ok(None);
        }
        let (perp_market_index, _) = perp_bankruptcies.first().unwrap();

        let sig = self.client.perp_liq_bankruptcy(
            (self.pubkey, &self.liqee),
            *perp_market_index,
            // Always use the max amount, since the health effect is always positive
            u64::MAX,
        )?;
        log::info!(
            "Liquidated bankruptcy for perp market on account {}, market index {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            perp_market_index,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    fn tokens(&self) -> anyhow::Result<Vec<(TokenIndex, I80F48, I80F48)>> {
        let mut tokens = self
            .liqee
            .active_token_positions()
            .map(|token_position| {
                let token = self.client.context.token(token_position.token_index);
                let bank = self
                    .account_fetcher
                    .fetch::<Bank>(&token.mint_info.first_bank())?;
                let oracle = self
                    .account_fetcher
                    .fetch_raw_account(&token.mint_info.oracle)?;
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
            .collect::<anyhow::Result<Vec<(TokenIndex, I80F48, I80F48)>>>()?;
        tokens.sort_by(|a, b| a.2.cmp(&b.2));
        Ok(tokens)
    }

    fn max_token_liab_transfer(
        &self,
        source: TokenIndex,
        target: TokenIndex,
    ) -> anyhow::Result<I80F48> {
        let mut liqor = self
            .account_fetcher
            .fetch_fresh_mango_account(&self.client.mango_account_address)
            .context("getting liquidator account")?;

        // Ensure the tokens are activated, so they appear in the health cache and
        // max_swap_source() will work.
        liqor.ensure_token_position(source)?;
        liqor.ensure_token_position(target)?;

        let health_cache = health_cache::new(&self.client.context, self.account_fetcher, &liqor)
            .expect("always ok");

        let source_price = health_cache.token_info(source).unwrap().prices.oracle;
        let target_price = health_cache.token_info(target).unwrap().prices.oracle;
        // TODO: This is where we could multiply in the liquidation fee factors
        let oracle_swap_price = source_price / target_price;

        let amount = health_cache
            .max_swap_source_for_health_ratio(
                source,
                target,
                oracle_swap_price,
                self.liqor_min_health_ratio,
            )
            .context("getting max_swap_source")?;
        Ok(amount)
    }

    fn token_liq(&self) -> anyhow::Result<Option<Signature>> {
        if !self.health_cache.has_borrows() || self.health_cache.can_call_spot_bankruptcy() {
            return Ok(None);
        }

        let tokens = self.tokens()?;

        let asset_token_index = tokens
            .iter()
            .rev()
            .find(|(asset_token_index, _asset_price, asset_usdc_equivalent)| {
                asset_usdc_equivalent.is_positive()
                    && jupiter_market_can_sell(self.client, *asset_token_index, QUOTE_TOKEN_INDEX)
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
                    && jupiter_market_can_buy(self.client, *liab_token_index, QUOTE_TOKEN_INDEX)
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
            .context("sending liq_token_with_token")?;
        log::info!(
            "Liquidated token with token for {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    fn token_liq_bankruptcy(&self) -> anyhow::Result<Option<Signature>> {
        if !self.health_cache.can_call_spot_bankruptcy() {
            return Ok(None);
        }

        let tokens = self.tokens()?;

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
                    && jupiter_market_can_buy(self.client, *liab_token_index, QUOTE_TOKEN_INDEX)
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
        let max_liab_transfer =
            self.max_token_liab_transfer(liab_token_index, quote_token_index)?;

        let sig = self
            .client
            .token_liq_bankruptcy(
                (self.pubkey, &self.liqee),
                liab_token_index,
                max_liab_transfer,
            )
            .context("sending liq_token_bankruptcy")?;
        log::info!(
            "Liquidated bankruptcy for {}, maint_health was {}, tx sig {:?}",
            self.pubkey,
            self.maint_health,
            sig
        );
        Ok(Some(sig))
    }

    fn send_liq_tx(&self) -> anyhow::Result<Signature> {
        // TODO: Should we make an attempt to settle positive PNL first?
        // The problem with it is that small market movements can continuously create
        // small amounts of new positive PNL while base_position > 0.
        // We shouldn't get stuck on this step, particularly if it's of limited value
        // to the liquidators.
        // if let Some(txsig) = self.perp_settle_positive_pnl()? {
        //     return Ok(txsig);
        // }

        // Try to close orders before touching the user's positions
        if let Some(txsig) = self.perp_close_orders()? {
            return Ok(txsig);
        }
        if let Some(txsig) = self.serum3_close_orders()? {
            return Ok(txsig);
        }

        if let Some(txsig) = self.perp_liq_base_position()? {
            return Ok(txsig);
        }

        // Now that the perp base positions are zeroed the perp pnl won't
        // fluctuate with the oracle price anymore.
        // It's possible that some positive pnl can't be settled (if there's
        // no liquid counterparty) and that some negative pnl can't be settled
        // (if the liqee isn't liquid enough).
        if let Some(txsig) = self.perp_settle_pnl()? {
            return Ok(txsig);
        }

        if let Some(txsig) = self.token_liq()? {
            return Ok(txsig);
        }

        // Socialize/insurance fund unsettleable negative pnl
        if let Some(txsig) = self.perp_liq_bankruptcy()? {
            return Ok(txsig);
        }

        // Socialize/insurance fund unliquidatable borrows
        if let Some(txsig) = self.token_liq_bankruptcy()? {
            return Ok(txsig);
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
pub fn maybe_liquidate_account(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    pubkey: &Pubkey,
    config: &Config,
) -> anyhow::Result<bool> {
    let liqor_min_health_ratio = I80F48::from_num(config.min_health_ratio);

    let account = account_fetcher.fetch_mango_account(pubkey)?;
    let health_cache =
        health_cache::new(&mango_client.context, account_fetcher, &account).expect("always ok");
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
    let account = account_fetcher.fetch_fresh_mango_account(pubkey)?;
    let health_cache =
        health_cache::new(&mango_client.context, account_fetcher, &account).expect("always ok");
    if !health_cache.is_liquidatable() {
        return Ok(false);
    }

    let maint_health = health_cache.health(HealthType::Maint);

    // try liquidating
    let txsig = LiquidateHelper {
        client: mango_client,
        account_fetcher,
        pubkey,
        liqee: &account,
        health_cache: &health_cache,
        maint_health,
        liqor_min_health_ratio,
    }
    .send_liq_tx()?;

    let slot = account_fetcher.transaction_max_slot(&[txsig])?;
    if let Err(e) = account_fetcher.refresh_accounts_via_rpc_until_slot(
        &[*pubkey, mango_client.mango_account_address],
        slot,
        config.refresh_timeout,
    ) {
        log::info!("could not refresh after liquidation: {}", e);
    }

    Ok(true)
}

#[allow(clippy::too_many_arguments)]
pub fn maybe_liquidate_one<'a>(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    accounts: impl Iterator<Item = &'a Pubkey>,
    config: &Config,
) -> bool {
    for pubkey in accounts {
        match maybe_liquidate_account(mango_client, account_fetcher, pubkey, config) {
            Err(err) => {
                // Not all errors need to be raised to the user's attention.
                let mut log_level = log::Level::Error;

                // Simulation errors due to liqee precondition failures on the liquidation instructions
                // will commonly happen if our liquidator is late or if there are chain forks.
                match err.downcast_ref::<MangoClientError>() {
                    Some(MangoClientError::SendTransactionPreflightFailure { logs }) => {
                        if logs.contains("HealthMustBeNegative") || logs.contains("IsNotBankrupt") {
                            log_level = log::Level::Trace;
                        }
                    }
                    _ => {}
                };
                log::log!(log_level, "liquidating account {}: {:?}", pubkey, err);
            }
            Ok(true) => return true,
            _ => {}
        };
    }

    false
}
