use std::collections::HashSet;
use std::time::Duration;

use itertools::Itertools;
use mango_v4::health::{HealthCache, HealthType};
use mango_v4::state::{MangoAccountValue, PerpMarketIndex, Side, TokenIndex, QUOTE_TOKEN_INDEX};
use mango_v4_client::{chain_data, health_cache, MangoClient};
use solana_sdk::signature::Signature;

use futures::{stream, StreamExt, TryStreamExt};
use rand::seq::SliceRandom;
use tracing::*;
use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

use crate::util;

#[derive(Clone)]
pub struct Config {
    pub min_health_ratio: f64,
    pub refresh_timeout: Duration,
    pub compute_limit_for_liq_ix: u32,
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
    config: Config,
}

impl<'a> LiquidateHelper<'a> {
    async fn serum3_close_orders(&self) -> anyhow::Result<Option<Signature>> {
        // look for any open serum orders or settleable balances
        let serum_oos: anyhow::Result<Vec<_>> = self
            .liqee
            .active_serum3_orders()
            .map(|orders| {
                let open_orders_account = self.account_fetcher.fetch_raw(&orders.open_orders)?;
                let open_orders = mango_v4::serum3_cpi::load_open_orders(&open_orders_account)?;
                Ok((*orders, *open_orders))
            })
            .try_collect();
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
        let txsig = self
            .client
            .serum3_liq_force_cancel_orders(
                (self.pubkey, &self.liqee),
                serum_orders.market_index,
                &serum_orders.open_orders,
            )
            .await?;
        info!(
            market_index = serum_orders.market_index,
            %txsig,
            "Force cancelled serum orders",
        );
        Ok(Some(txsig))
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
        let txsig = self
            .client
            .perp_liq_force_cancel_orders((self.pubkey, &self.liqee), perp_market_index)
            .await?;
        info!(
            perp_market_index,
            %txsig,
            "Force cancelled perp orders",
        );
        Ok(Some(txsig))
    }

    fn liq_compute_limit_instruction(&self) -> solana_sdk::instruction::Instruction {
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            self.config.compute_limit_for_liq_ix,
        )
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
        trace!(
            max_base_transfer_abs,
            max_pnl_transfer,
            "computed transfer maximums"
        );

        let liq_ix = self
            .client
            .perp_liq_base_or_positive_pnl_instruction(
                (self.pubkey, &self.liqee),
                *perp_market_index,
                side_signum * max_base_transfer_abs,
                max_pnl_transfer,
            )
            .await
            .context("creating perp_liq_base_or_positive_pnl_instruction")?;
        let txsig = self
            .client
            .send_and_confirm_owner_tx(vec![self.liq_compute_limit_instruction(), liq_ix])
            .await
            .context("sending perp_liq_base_or_positive_pnl_instruction")?;
        info!(
            perp_market_index,
            %txsig,
            "Liquidated base position for perp market",
        );
        Ok(Some(txsig))
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

        let liq_ix = self
            .client
            .perp_liq_negative_pnl_or_bankruptcy_instruction(
                (self.pubkey, &self.liqee),
                *perp_market_index,
                // Always use the max amount, since the health effect is >= 0
                u64::MAX,
            )
            .await
            .context("creating perp_liq_negative_pnl_or_bankruptcy_instruction")?;
        let txsig = self
            .client
            .send_and_confirm_owner_tx(vec![self.liq_compute_limit_instruction(), liq_ix])
            .await
            .context("sending perp_liq_negative_pnl_or_bankruptcy_instruction")?;
        info!(
            perp_market_index,
            %txsig,
            "Liquidated negative perp pnl",
        );
        Ok(Some(txsig))
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

        util::max_swap_source(
            self.client,
            self.account_fetcher,
            &liqor,
            source,
            target,
            price,
            self.liqor_min_health_ratio,
        )
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
        let liq_ix = self
            .client
            .token_liq_with_token_instruction(
                (self.pubkey, &self.liqee),
                asset_token_index,
                liab_token_index,
                max_liab_transfer,
            )
            .await
            .context("creating liq_token_with_token ix")?;
        let txsig = self
            .client
            .send_and_confirm_owner_tx(vec![self.liq_compute_limit_instruction(), liq_ix])
            .await
            .context("sending liq_token_with_token")?;
        info!(
            asset_token_index,
            liab_token_index,
            %txsig,
            "Liquidated token with token",
        );
        Ok(Some(txsig))
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

        let liq_ix = self
            .client
            .token_liq_bankruptcy_instruction(
                (self.pubkey, &self.liqee),
                liab_token_index,
                max_liab_transfer,
            )
            .await
            .context("creating liq_token_bankruptcy")?;
        let txsig = self
            .client
            .send_and_confirm_owner_tx(vec![self.liq_compute_limit_instruction(), liq_ix])
            .await
            .context("sending liq_token_with_token")?;
        info!(
            liab_token_index,
            %txsig,
            "Liquidated token bankruptcy",
        );
        Ok(Some(txsig))
    }

    #[instrument(skip(self), fields(pubkey = %*self.pubkey, maint = %self.maint_health))]
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
            info!("there are open perp fills, waiting...",);
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

    trace!(
        %pubkey,
        %maint_health,
        "possible candidate",
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
        config: config.clone(),
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
            info!("could not refresh after liquidation: {}", e);
        }
    }

    Ok(true)
}
