use anchor_lang::prelude::*;

use fixed::types::I80F48;

use crate::error::*;
use crate::state::{
    Bank, MangoAccountRef, PerpMarket, PerpMarketIndex, PerpPosition, Serum3MarketIndex, TokenIndex,
};
use crate::util::checked_math as cm;

use super::*;

/// Information about prices for a bank or perp market.
#[derive(Clone, AnchorDeserialize, AnchorSerialize, Debug)]
pub struct Prices {
    /// The current oracle price
    pub oracle: I80F48, // native/native

    /// A "stable" price, provided by StablePriceModel
    pub stable: I80F48, // native/native
}

impl Prices {
    // intended for tests
    pub fn new_single_price(price: I80F48) -> Self {
        Self {
            oracle: price,
            stable: price,
        }
    }

    /// The liability price to use for the given health type
    #[inline(always)]
    pub fn liab(&self, health_type: HealthType) -> I80F48 {
        match health_type {
            HealthType::Maint | HealthType::LiquidationEnd => self.oracle,
            HealthType::Init => self.oracle.max(self.stable),
        }
    }

    /// The asset price to use for the given health type
    #[inline(always)]
    pub fn asset(&self, health_type: HealthType) -> I80F48 {
        match health_type {
            HealthType::Maint | HealthType::LiquidationEnd => self.oracle,
            HealthType::Init => self.oracle.min(self.stable),
        }
    }
}

/// There are three types of health:
/// - initial health ("init"): users can only open new positions if it's >= 0
/// - maintenance health ("maint"): users get liquidated if it's < 0
/// - liquidation end health: once liquidation started (see being_liquidated), it
///   only stops once this is >= 0
///
/// The ordering is
///   init health <= liquidation end health <= maint health
///
/// The different health types are realized by using different weights and prices:
/// - init health: init weights with scaling, stable-price adjusted prices
/// - liq end health: init weights without scaling, oracle prices
/// - maint health: maint weights, oracle prices
///
#[derive(PartialEq, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub enum HealthType {
    Init,
    Maint, // aka LiquidationStart
    LiquidationEnd,
}

/// Computes health for a mango account given a set of account infos
///
/// These account infos must fit the fixed layout defined by FixedOrderAccountRetriever.
pub fn compute_health_from_fixed_accounts(
    account: &MangoAccountRef,
    health_type: HealthType,
    ais: &[AccountInfo],
) -> Result<I80F48> {
    let retriever = new_fixed_order_account_retriever(ais, account)?;
    Ok(new_health_cache(account, &retriever)?.health(health_type))
}

/// Compute health with an arbitrary AccountRetriever
pub fn compute_health(
    account: &MangoAccountRef,
    health_type: HealthType,
    retriever: &impl AccountRetriever,
) -> Result<I80F48> {
    Ok(new_health_cache(account, retriever)?.health(health_type))
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize, Debug)]
pub struct TokenInfo {
    pub token_index: TokenIndex,
    pub maint_asset_weight: I80F48,
    pub init_asset_weight: I80F48,
    pub init_scaled_asset_weight: I80F48,
    pub maint_liab_weight: I80F48,
    pub init_liab_weight: I80F48,
    pub init_scaled_liab_weight: I80F48,
    pub prices: Prices,
    pub balance_native: I80F48,
}

impl TokenInfo {
    #[inline(always)]
    fn asset_weight(&self, health_type: HealthType) -> I80F48 {
        match health_type {
            HealthType::Init => self.init_scaled_asset_weight,
            HealthType::LiquidationEnd => self.init_asset_weight,
            HealthType::Maint => self.maint_asset_weight,
        }
    }

    #[inline(always)]
    fn liab_weight(&self, health_type: HealthType) -> I80F48 {
        match health_type {
            HealthType::Init => self.init_scaled_liab_weight,
            HealthType::LiquidationEnd => self.init_liab_weight,
            HealthType::Maint => self.maint_liab_weight,
        }
    }

    #[inline(always)]
    fn health_contribution(&self, health_type: HealthType) -> I80F48 {
        let (weight, price) = if self.balance_native.is_negative() {
            (self.liab_weight(health_type), self.prices.liab(health_type))
        } else {
            (
                self.asset_weight(health_type),
                self.prices.asset(health_type),
            )
        };
        cm!(self.balance_native * price * weight)
    }
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize, Debug)]
pub struct Serum3Info {
    // reserved amounts as stored on the open orders
    pub reserved_base: I80F48,
    pub reserved_quote: I80F48,

    pub base_index: usize,
    pub quote_index: usize,
    pub market_index: Serum3MarketIndex,
    /// The open orders account has no free or reserved funds
    pub has_zero_funds: bool,
}

impl Serum3Info {
    #[inline(always)]
    fn health_contribution(
        &self,
        health_type: HealthType,
        token_infos: &[TokenInfo],
        token_max_reserved: &[I80F48],
        market_reserved: &Serum3Reserved,
    ) -> I80F48 {
        if market_reserved.all_reserved_as_base.is_zero()
            || market_reserved.all_reserved_as_quote.is_zero()
        {
            return I80F48::ZERO;
        }

        let base_info = &token_infos[self.base_index];
        let quote_info = &token_infos[self.quote_index];
        let base_max_reserved = token_max_reserved[self.base_index];
        let quote_max_reserved = token_max_reserved[self.quote_index];

        // How much would health increase if the reserved balance were applied to the passed
        // token info?
        let compute_health_effect =
            |token_info: &TokenInfo, token_max_reserved: I80F48, market_reserved: I80F48| {
                // This balance includes all possible reserved funds from markets that relate to the
                // token, including this market itself: `market_reserved` is already included in `token_max_reserved`.
                let max_balance = cm!(token_info.balance_native + token_max_reserved);

                // For simplicity, we assume that `market_reserved` was added to `max_balance` last
                // (it underestimates health because that gives the smallest effects): how much did
                // health change because of it?
                let (asset_part, liab_part) = if max_balance >= market_reserved {
                    (market_reserved, I80F48::ZERO)
                } else if max_balance.is_negative() {
                    (I80F48::ZERO, market_reserved)
                } else {
                    (max_balance, cm!(market_reserved - max_balance))
                };

                let asset_weight = token_info.asset_weight(health_type);
                let liab_weight = token_info.liab_weight(health_type);
                let asset_price = token_info.prices.asset(health_type);
                let liab_price = token_info.prices.liab(health_type);
                cm!(asset_part * asset_weight * asset_price + liab_part * liab_weight * liab_price)
            };

        let health_base = compute_health_effect(
            base_info,
            base_max_reserved,
            market_reserved.all_reserved_as_base,
        );
        let health_quote = compute_health_effect(
            quote_info,
            quote_max_reserved,
            market_reserved.all_reserved_as_quote,
        );
        health_base.min(health_quote)
    }
}

#[derive(Clone)]
pub(crate) struct Serum3Reserved {
    /// base tokens when the serum3info.reserved_quote get converted to base and added to reserved_base
    all_reserved_as_base: I80F48,
    /// ditto the other way around
    all_reserved_as_quote: I80F48,
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize, Debug)]
pub struct PerpInfo {
    pub perp_market_index: PerpMarketIndex,
    pub maint_base_asset_weight: I80F48,
    pub init_base_asset_weight: I80F48,
    pub maint_base_liab_weight: I80F48,
    pub init_base_liab_weight: I80F48,
    pub maint_overall_asset_weight: I80F48,
    pub init_overall_asset_weight: I80F48,
    pub base_lot_size: i64,
    pub base_lots: i64,
    pub bids_base_lots: i64,
    pub asks_base_lots: i64,
    // in health-reference-token native units, no asset/liab factor needed
    pub quote: I80F48,
    pub prices: Prices,
    pub has_open_orders: bool,
    pub has_open_fills: bool,
}

impl PerpInfo {
    fn new(perp_position: &PerpPosition, perp_market: &PerpMarket, prices: Prices) -> Result<Self> {
        let base_lots = cm!(perp_position.base_position_lots() + perp_position.taker_base_lots);

        let unsettled_funding = perp_position.unsettled_funding(perp_market);
        let taker_quote = I80F48::from(cm!(
            perp_position.taker_quote_lots * perp_market.quote_lot_size
        ));
        let quote_current =
            cm!(perp_position.quote_position_native() - unsettled_funding + taker_quote);

        Ok(Self {
            perp_market_index: perp_market.perp_market_index,
            init_base_asset_weight: perp_market.init_base_asset_weight,
            init_base_liab_weight: perp_market.init_base_liab_weight,
            maint_base_asset_weight: perp_market.maint_base_asset_weight,
            maint_base_liab_weight: perp_market.maint_base_liab_weight,
            init_overall_asset_weight: perp_market.init_overall_asset_weight,
            maint_overall_asset_weight: perp_market.maint_overall_asset_weight,
            base_lot_size: perp_market.base_lot_size,
            base_lots,
            bids_base_lots: perp_position.bids_base_lots,
            asks_base_lots: perp_position.asks_base_lots,
            quote: quote_current,
            prices,
            has_open_orders: perp_position.has_open_orders(),
            has_open_fills: perp_position.has_open_taker_fills(),
        })
    }

    /// Total health contribution from perp balances
    ///
    /// For fully isolated perp markets, users may never borrow against unsettled
    /// positive perp pnl, there pnl_asset_weight == 0 and there can't be positive
    /// health contributions from these perp market. We sometimes call these markets
    /// "untrusted markets".
    ///
    /// Users need to settle their perp pnl with other perp market participants
    /// in order to realize their gains if they want to use them as collateral.
    ///
    /// This is because we don't trust the perp's base price to not suddenly jump to
    /// zero (if users could borrow against their perp balances they might now
    /// be bankrupt) or suddenly increase a lot (if users could borrow against perp
    /// balances they could now borrow other assets).
    ///
    /// Other markets may be liquid enough that we have enough confidence to allow
    /// users to borrow against unsettled positive pnl to some extend. In these cases,
    /// the pnl asset weights would be >0.
    #[inline(always)]
    pub fn health_contribution(&self, health_type: HealthType) -> I80F48 {
        let contribution = self.unweighted_health_contribution(health_type);
        self.weigh_health_contribution(contribution, health_type)
    }

    #[inline(always)]
    pub fn weigh_health_contribution(&self, unweighted: I80F48, health_type: HealthType) -> I80F48 {
        if unweighted > 0 {
            let asset_weight = match health_type {
                HealthType::Init | HealthType::LiquidationEnd => self.init_overall_asset_weight,
                HealthType::Maint => self.maint_overall_asset_weight,
            };

            cm!(asset_weight * unweighted)
        } else {
            unweighted
        }
    }

    #[inline(always)]
    pub fn unweighted_health_contribution(&self, health_type: HealthType) -> I80F48 {
        let order_execution_case = |orders_base_lots: i64, order_price: I80F48| {
            let net_base_native =
                I80F48::from(cm!((self.base_lots + orders_base_lots) * self.base_lot_size));
            let weight = match (health_type, net_base_native.is_negative()) {
                (HealthType::Init, true) | (HealthType::LiquidationEnd, true) => {
                    self.init_base_liab_weight
                }
                (HealthType::Init, false) | (HealthType::LiquidationEnd, false) => {
                    self.init_base_asset_weight
                }
                (HealthType::Maint, true) => self.maint_base_liab_weight,
                (HealthType::Maint, false) => self.maint_base_asset_weight,
            };
            let base_price = if net_base_native.is_negative() {
                self.prices.liab(health_type)
            } else {
                self.prices.asset(health_type)
            };

            // Total value of the order-execution adjusted base position
            let base_health = cm!(net_base_native * weight * base_price);

            let orders_base_native = I80F48::from(cm!(orders_base_lots * self.base_lot_size));
            // The quote change from executing the bids/asks
            let order_quote = cm!(-orders_base_native * order_price);

            cm!(base_health + order_quote)
        };

        // What is worse: Executing all bids at oracle_price.liab, or executing all asks at oracle_price.asset?
        let bids_case = order_execution_case(self.bids_base_lots, self.prices.liab(health_type));
        let asks_case = order_execution_case(-self.asks_base_lots, self.prices.asset(health_type));
        let worst_case = bids_case.min(asks_case);

        cm!(self.quote + worst_case)
    }
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize, Debug)]
pub struct HealthCache {
    pub(crate) token_infos: Vec<TokenInfo>,
    pub(crate) serum3_infos: Vec<Serum3Info>,
    pub(crate) perp_infos: Vec<PerpInfo>,
    pub(crate) being_liquidated: bool,
}

impl HealthCache {
    pub fn health(&self, health_type: HealthType) -> I80F48 {
        let mut health = I80F48::ZERO;
        let sum = |contrib| {
            cm!(health += contrib);
        };
        self.health_sum(health_type, sum);
        health
    }

    /// Sum of only the positive health components (assets) and
    /// sum of absolute values of all negative health components (liabs, always >= 0)
    pub fn health_assets_and_liabs(&self, health_type: HealthType) -> (I80F48, I80F48) {
        let mut assets = I80F48::ZERO;
        let mut liabs = I80F48::ZERO;
        let sum = |contrib| {
            if contrib > 0 {
                cm!(assets += contrib);
            } else {
                cm!(liabs -= contrib);
            }
        };
        self.health_sum(health_type, sum);
        (assets, liabs)
    }

    pub fn token_info(&self, token_index: TokenIndex) -> Result<&TokenInfo> {
        Ok(&self.token_infos[self.token_info_index(token_index)?])
    }

    fn token_info_index(&self, token_index: TokenIndex) -> Result<usize> {
        self.token_infos
            .iter()
            .position(|t| t.token_index == token_index)
            .ok_or_else(|| {
                error_msg_typed!(
                    MangoError::TokenPositionDoesNotExist,
                    "token index {} not found",
                    token_index
                )
            })
    }

    pub fn perp_info(&self, perp_market_index: PerpMarketIndex) -> Result<&PerpInfo> {
        Ok(&self.perp_infos[self.perp_info_index(perp_market_index)?])
    }

    pub(crate) fn perp_info_index(&self, perp_market_index: PerpMarketIndex) -> Result<usize> {
        self.perp_infos
            .iter()
            .position(|t| t.perp_market_index == perp_market_index)
            .ok_or_else(|| {
                error_msg_typed!(
                    MangoError::PerpPositionDoesNotExist,
                    "perp market index {} not found",
                    perp_market_index
                )
            })
    }

    /// Changes the cached user account token balance.
    pub fn adjust_token_balance(&mut self, bank: &Bank, change: I80F48) -> Result<()> {
        let entry_index = self.token_info_index(bank.token_index)?;
        let mut entry = &mut self.token_infos[entry_index];

        // Note: resetting the weights here assumes that the change has been applied to
        // the passed in bank already
        entry.init_scaled_asset_weight =
            bank.scaled_init_asset_weight(entry.prices.asset(HealthType::Init));
        entry.init_scaled_liab_weight =
            bank.scaled_init_liab_weight(entry.prices.liab(HealthType::Init));

        // Work around the fact that -((-x) * y) == x * y does not hold for I80F48:
        // We need to make sure that if balance is before * price, then change = -before
        // brings it to exactly zero.
        let removed_contribution = -change;
        cm!(entry.balance_native -= removed_contribution);
        Ok(())
    }

    /// Changes the cached user account token and serum balances.
    ///
    /// WARNING: You must also call recompute_token_weights() after all bank
    /// deposit/withdraw changes!
    #[allow(clippy::too_many_arguments)]
    pub fn adjust_serum3_reserved(
        &mut self,
        market_index: Serum3MarketIndex,
        base_token_index: TokenIndex,
        reserved_base_change: I80F48,
        free_base_change: I80F48,
        quote_token_index: TokenIndex,
        reserved_quote_change: I80F48,
        free_quote_change: I80F48,
    ) -> Result<()> {
        let base_entry_index = self.token_info_index(base_token_index)?;
        let quote_entry_index = self.token_info_index(quote_token_index)?;

        // Apply it to the tokens
        {
            let base_entry = &mut self.token_infos[base_entry_index];
            cm!(base_entry.balance_native += free_base_change);
        }
        {
            let quote_entry = &mut self.token_infos[quote_entry_index];
            cm!(quote_entry.balance_native += free_quote_change);
        }

        // Apply it to the serum3 info
        let market_entry = self
            .serum3_infos
            .iter_mut()
            .find(|m| m.market_index == market_index)
            .ok_or_else(|| error_msg!("serum3 market {} not found", market_index))?;
        cm!(market_entry.reserved_base += reserved_base_change);
        cm!(market_entry.reserved_quote += reserved_quote_change);
        Ok(())
    }

    pub fn recompute_perp_info(
        &mut self,
        perp_position: &PerpPosition,
        perp_market: &PerpMarket,
    ) -> Result<()> {
        let perp_entry = self
            .perp_infos
            .iter_mut()
            .find(|m| m.perp_market_index == perp_market.perp_market_index)
            .ok_or_else(|| error_msg!("perp market {} not found", perp_market.perp_market_index))?;
        *perp_entry = PerpInfo::new(perp_position, perp_market, perp_entry.prices.clone())?;
        Ok(())
    }

    pub fn has_spot_assets(&self) -> bool {
        self.token_infos.iter().any(|ti| {
            // can use token_liq_with_token
            ti.balance_native >= 1
        })
    }

    pub fn has_serum3_open_orders_funds(&self) -> bool {
        self.serum3_infos.iter().any(|si| !si.has_zero_funds)
    }

    pub fn has_perp_open_orders(&self) -> bool {
        self.perp_infos.iter().any(|p| p.has_open_orders)
    }

    pub fn has_perp_base_positions(&self) -> bool {
        self.perp_infos.iter().any(|p| p.base_lots != 0)
    }

    pub fn has_perp_open_fills(&self) -> bool {
        self.perp_infos.iter().any(|p| p.has_open_fills)
    }

    pub fn has_perp_positive_pnl_no_base(&self) -> bool {
        self.perp_infos
            .iter()
            .any(|p| p.base_lots == 0 && p.quote > 0)
    }

    pub fn has_perp_negative_pnl(&self) -> bool {
        self.perp_infos.iter().any(|p| p.quote < 0)
    }

    /// Phase1 is spot/perp order cancellation and spot settlement since
    /// neither of these come at a cost to the liqee
    pub fn has_phase1_liquidatable(&self) -> bool {
        self.has_serum3_open_orders_funds() || self.has_perp_open_orders()
    }

    pub fn require_after_phase1_liquidation(&self) -> Result<()> {
        require!(
            !self.has_serum3_open_orders_funds(),
            MangoError::HasOpenOrUnsettledSerum3Orders
        );
        require!(!self.has_perp_open_orders(), MangoError::HasOpenPerpOrders);
        Ok(())
    }

    pub fn in_phase1_liquidation(&self) -> bool {
        self.has_phase1_liquidatable()
    }

    /// Phase2 is for:
    /// - token-token liquidation
    /// - liquidation of perp base positions (an open fill isn't liquidatable, but
    ///   it changes the base position, so need to wait for it to be processed...)
    /// - bringing positive trusted perp pnl into the spot realm
    pub fn has_phase2_liquidatable(&self) -> bool {
        self.has_spot_assets() && self.has_spot_borrows()
            || self.has_perp_base_positions()
            || self.has_perp_open_fills()
            || self.has_perp_positive_pnl_no_base()
    }

    pub fn require_after_phase2_liquidation(&self) -> Result<()> {
        self.require_after_phase1_liquidation()?;
        require!(
            !self.has_spot_assets() || !self.has_spot_borrows(),
            MangoError::HasLiquidatableTokenPosition
        );
        require!(
            !self.has_perp_base_positions(),
            MangoError::HasLiquidatablePerpBasePosition
        );
        require!(
            !self.has_perp_open_fills(),
            MangoError::HasOpenPerpTakerFills
        );
        require!(
            !self.has_perp_positive_pnl_no_base(),
            MangoError::HasLiquidatablePositivePerpPnl
        );
        Ok(())
    }

    pub fn in_phase2_liquidation(&self) -> bool {
        !self.has_phase1_liquidatable() && self.has_phase2_liquidatable()
    }

    /// Phase3 is bankruptcy:
    /// - token bankruptcy
    /// - perp bankruptcy
    pub fn has_phase3_liquidatable(&self) -> bool {
        self.has_spot_borrows() || self.has_perp_negative_pnl()
    }

    pub fn in_phase3_liquidation(&self) -> bool {
        !self.has_phase1_liquidatable()
            && !self.has_phase2_liquidatable()
            && self.has_phase3_liquidatable()
    }

    pub fn has_spot_borrows(&self) -> bool {
        self.token_infos.iter().any(|ti| ti.balance_native < 0)
    }

    pub(crate) fn compute_serum3_reservations(
        &self,
        health_type: HealthType,
    ) -> (Vec<I80F48>, Vec<Serum3Reserved>) {
        // For each token, compute the sum of serum-reserved amounts over all markets.
        let mut token_max_reserved = vec![I80F48::ZERO; self.token_infos.len()];

        // For each serum market, compute what happened if reserved_base was converted to quote
        // or reserved_quote was converted to base.
        let mut serum3_reserved = Vec::with_capacity(self.serum3_infos.len());

        for info in self.serum3_infos.iter() {
            let quote = &self.token_infos[info.quote_index];
            let base = &self.token_infos[info.base_index];

            let reserved_base = info.reserved_base;
            let reserved_quote = info.reserved_quote;

            let quote_asset = quote.prices.asset(health_type);
            let base_liab = base.prices.liab(health_type);
            // OPTIMIZATION: These divisions can be extremely expensive (up to 5k CU each)
            let all_reserved_as_base =
                cm!(reserved_base + reserved_quote * quote_asset / base_liab);

            let base_asset = base.prices.asset(health_type);
            let quote_liab = quote.prices.liab(health_type);
            let all_reserved_as_quote =
                cm!(reserved_quote + reserved_base * base_asset / quote_liab);

            let base_max_reserved = &mut token_max_reserved[info.base_index];
            // note: cm!() does not work with mutable references
            *base_max_reserved = base_max_reserved.checked_add(all_reserved_as_base).unwrap();
            let quote_max_reserved = &mut token_max_reserved[info.quote_index];
            *quote_max_reserved = quote_max_reserved
                .checked_add(all_reserved_as_quote)
                .unwrap();

            serum3_reserved.push(Serum3Reserved {
                all_reserved_as_base,
                all_reserved_as_quote,
            });
        }

        (token_max_reserved, serum3_reserved)
    }

    pub(crate) fn health_sum(&self, health_type: HealthType, mut action: impl FnMut(I80F48)) {
        for token_info in self.token_infos.iter() {
            let contrib = token_info.health_contribution(health_type);
            action(contrib);
        }

        let (token_max_reserved, serum3_reserved) = self.compute_serum3_reservations(health_type);
        for (serum3_info, reserved) in self.serum3_infos.iter().zip(serum3_reserved.iter()) {
            let contrib = serum3_info.health_contribution(
                health_type,
                &self.token_infos,
                &token_max_reserved,
                reserved,
            );
            action(contrib);
        }

        for perp_info in self.perp_infos.iter() {
            let contrib = perp_info.health_contribution(health_type);
            action(contrib);
        }
    }

    /// Compute the health when it comes to settling perp pnl
    ///
    /// Examples:
    /// - An account may have maint_health < 0, but settling perp pnl could still be allowed.
    ///   (+100 USDC health, -50 USDT health, -50 perp health -> allow settling 50 health worth)
    /// - Positive health from trusted pnl markets counts
    /// - If overall health is 0 with two trusted perp pnl < 0, settling may still be possible.
    ///   (+100 USDC health, -150 perp1 health, -150 perp2 health -> allow settling 100 health worth)
    /// - Positive trusted perp pnl can enable settling.
    ///   (+100 trusted perp1 health, -100 perp2 health -> allow settling of 100 health worth)
    pub fn perp_settle_health(&self) -> I80F48 {
        let health_type = HealthType::Maint;
        let mut health = I80F48::ZERO;
        for token_info in self.token_infos.iter() {
            let contrib = token_info.health_contribution(health_type);
            cm!(health += contrib);
        }

        let (token_max_reserved, serum3_reserved) = self.compute_serum3_reservations(health_type);
        for (serum3_info, reserved) in self.serum3_infos.iter().zip(serum3_reserved.iter()) {
            let contrib = serum3_info.health_contribution(
                health_type,
                &self.token_infos,
                &token_max_reserved,
                reserved,
            );
            cm!(health += contrib);
        }

        for perp_info in self.perp_infos.iter() {
            let positive_contrib = perp_info.health_contribution(health_type).max(I80F48::ZERO);
            cm!(health += positive_contrib);
        }
        health
    }
}

pub(crate) fn find_token_info_index(infos: &[TokenInfo], token_index: TokenIndex) -> Result<usize> {
    infos
        .iter()
        .position(|ti| ti.token_index == token_index)
        .ok_or_else(|| {
            error_msg_typed!(
                MangoError::TokenPositionDoesNotExist,
                "token index {} not found",
                token_index
            )
        })
}

/// Generate a HealthCache for an account and its health accounts.
pub fn new_health_cache(
    account: &MangoAccountRef,
    retriever: &impl AccountRetriever,
) -> Result<HealthCache> {
    // token contribution from token accounts
    let mut token_infos = vec![];

    for (i, position) in account.active_token_positions().enumerate() {
        let (bank, oracle_price) =
            retriever.bank_and_oracle(&account.fixed.group, i, position.token_index)?;

        let native = position.native(bank);
        let prices = Prices {
            oracle: oracle_price,
            stable: bank.stable_price(),
        };
        // Use the liab price for computing weight scaling, because it's pessimistic and
        // causes the most unfavorable scaling.
        let liab_price = prices.liab(HealthType::Init);
        token_infos.push(TokenInfo {
            token_index: bank.token_index,
            maint_asset_weight: bank.maint_asset_weight,
            init_asset_weight: bank.init_asset_weight,
            init_scaled_asset_weight: bank.scaled_init_asset_weight(liab_price),
            maint_liab_weight: bank.maint_liab_weight,
            init_liab_weight: bank.init_liab_weight,
            init_scaled_liab_weight: bank.scaled_init_liab_weight(liab_price),
            prices,
            balance_native: native,
        });
    }

    // Fill the TokenInfo balance with free funds in serum3 oo accounts and build Serum3Infos.
    let mut serum3_infos = vec![];
    for (i, serum_account) in account.active_serum3_orders().enumerate() {
        let oo = retriever.serum_oo(i, &serum_account.open_orders)?;

        // find the TokenInfos for the market's base and quote tokens
        let base_index = find_token_info_index(&token_infos, serum_account.base_token_index)?;
        let quote_index = find_token_info_index(&token_infos, serum_account.quote_token_index)?;

        // add the amounts that are freely settleable immediately to token balances
        let base_free = I80F48::from(oo.native_coin_free);
        let quote_free = I80F48::from(cm!(oo.native_pc_free + oo.referrer_rebates_accrued));
        let base_info = &mut token_infos[base_index];
        cm!(base_info.balance_native += base_free);
        let quote_info = &mut token_infos[quote_index];
        cm!(quote_info.balance_native += quote_free);

        // track the reserved amounts
        let reserved_base = I80F48::from(cm!(oo.native_coin_total - oo.native_coin_free));
        let reserved_quote = I80F48::from(cm!(oo.native_pc_total - oo.native_pc_free));

        serum3_infos.push(Serum3Info {
            reserved_base,
            reserved_quote,
            base_index,
            quote_index,
            market_index: serum_account.market_index,
            has_zero_funds: oo.native_coin_total == 0
                && oo.native_pc_total == 0
                && oo.referrer_rebates_accrued == 0,
        });
    }

    // health contribution from perp accounts
    let mut perp_infos = Vec::with_capacity(account.active_perp_positions().count());
    for (i, perp_position) in account.active_perp_positions().enumerate() {
        let (perp_market, oracle_price) = retriever.perp_market_and_oracle_price(
            &account.fixed.group,
            i,
            perp_position.market_index,
        )?;
        perp_infos.push(PerpInfo::new(
            perp_position,
            perp_market,
            Prices {
                oracle: oracle_price,
                stable: perp_market.stable_price(),
            },
        )?);
    }

    Ok(HealthCache {
        token_infos,
        serum3_infos,
        perp_infos,
        being_liquidated: account.fixed.being_liquidated(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::test::*;
    use super::*;
    use crate::state::*;
    use serum_dex::state::OpenOrders;
    use std::str::FromStr;

    #[test]
    fn test_precision() {
        // I80F48 can only represent until 1/2^48
        assert_ne!(
            I80F48::from_num(1_u128) / I80F48::from_num(2_u128.pow(48)),
            0
        );
        assert_eq!(
            I80F48::from_num(1_u128) / I80F48::from_num(2_u128.pow(49)),
            0
        );

        // I80F48 can only represent until 14 decimal points
        assert_ne!(
            I80F48::from_str(format!("0.{}1", "0".repeat(13)).as_str()).unwrap(),
            0
        );
        assert_eq!(
            I80F48::from_str(format!("0.{}1", "0".repeat(14)).as_str()).unwrap(),
            0
        );
    }

    fn health_eq(a: I80F48, b: f64) -> bool {
        if (a - I80F48::from_num(b)).abs() < 0.001 {
            true
        } else {
            println!("health is {}, but expected {}", a, b);
            false
        }
    }

    // Run a health test that includes all the side values (like referrer_rebates_accrued)
    #[test]
    fn test_health0() {
        let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut account = MangoAccountValue::from_bytes(&buffer).unwrap();

        let group = Pubkey::new_unique();

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 1, 1.0, 0.2, 0.1);
        let (mut bank2, mut oracle2) = mock_bank_and_oracle(group, 4, 5.0, 0.5, 0.3);
        bank1
            .data()
            .deposit(
                account.ensure_token_position(1).unwrap().0,
                I80F48::from(100),
                DUMMY_NOW_TS,
            )
            .unwrap();
        bank2
            .data()
            .withdraw_without_fee(
                account.ensure_token_position(4).unwrap().0,
                I80F48::from(10),
                DUMMY_NOW_TS,
                DUMMY_PRICE,
            )
            .unwrap();

        let mut oo1 = TestAccount::<OpenOrders>::new_zeroed();
        let serum3account = account.create_serum3_orders(2).unwrap();
        serum3account.open_orders = oo1.pubkey;
        serum3account.base_token_index = 4;
        serum3account.quote_token_index = 1;
        oo1.data().native_pc_total = 21;
        oo1.data().native_coin_total = 18;
        oo1.data().native_pc_free = 1;
        oo1.data().native_coin_free = 3;
        oo1.data().referrer_rebates_accrued = 2;

        let mut perp1 = mock_perp_market(group, oracle2.pubkey, 5.0, 9, (0.2, 0.1), (0.05, 0.02));
        let perpaccount = account.ensure_perp_position(9, 1).unwrap().0;
        perpaccount.record_trade(perp1.data(), 3, -I80F48::from(310u16));
        perpaccount.bids_base_lots = 7;
        perpaccount.asks_base_lots = 11;
        perpaccount.taker_base_lots = 1;
        perpaccount.taker_quote_lots = 2;

        let oracle2_ai = oracle2.as_account_info();

        let ais = vec![
            bank1.as_account_info(),
            bank2.as_account_info(),
            oracle1.as_account_info(),
            oracle2_ai.clone(),
            perp1.as_account_info(),
            oracle2_ai,
            oo1.as_account_info(),
        ];

        let retriever = ScanningAccountRetriever::new_with_staleness(&ais, &group, None).unwrap();

        // for bank1/oracle1, including open orders (scenario: bids execute)
        let health1 = (100.0 + 1.0 + 2.0 + (20.0 + 15.0 * 5.0)) * 0.8;
        // for bank2/oracle2
        let health2 = (-10.0 + 3.0) * 5.0 * 1.5;
        // for perp (scenario: bids execute)
        let health3 =
            (3.0 + 7.0 + 1.0) * 10.0 * 5.0 * 0.8 + (-310.0 + 2.0 * 100.0 - 7.0 * 10.0 * 5.0);
        assert!(health_eq(
            compute_health(&account.borrow(), HealthType::Init, &retriever).unwrap(),
            health1 + health2 + health3
        ));
    }

    #[derive(Default)]
    struct BankSettings {
        deposits: u64,
        borrows: u64,
        deposit_weight_scale_start_quote: u64,
        borrow_weight_scale_start_quote: u64,
    }

    #[derive(Default)]
    struct TestHealth1Case {
        token1: i64,
        token2: i64,
        token3: i64,
        oo_1_2: (u64, u64),
        oo_1_3: (u64, u64),
        perp1: (i64, i64, i64, i64),
        expected_health: f64,
        bank_settings: [BankSettings; 3],
    }
    fn test_health1_runner(testcase: &TestHealth1Case) {
        let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut account = MangoAccountValue::from_bytes(&buffer).unwrap();

        let group = Pubkey::new_unique();

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 1, 1.0, 0.2, 0.1);
        let (mut bank2, mut oracle2) = mock_bank_and_oracle(group, 4, 5.0, 0.5, 0.3);
        let (mut bank3, mut oracle3) = mock_bank_and_oracle(group, 5, 10.0, 0.5, 0.3);
        bank1
            .data()
            .change_without_fee(
                account.ensure_token_position(1).unwrap().0,
                I80F48::from(testcase.token1),
                DUMMY_NOW_TS,
                DUMMY_PRICE,
            )
            .unwrap();
        bank2
            .data()
            .change_without_fee(
                account.ensure_token_position(4).unwrap().0,
                I80F48::from(testcase.token2),
                DUMMY_NOW_TS,
                DUMMY_PRICE,
            )
            .unwrap();
        bank3
            .data()
            .change_without_fee(
                account.ensure_token_position(5).unwrap().0,
                I80F48::from(testcase.token3),
                DUMMY_NOW_TS,
                DUMMY_PRICE,
            )
            .unwrap();
        for (settings, bank) in testcase
            .bank_settings
            .iter()
            .zip([&mut bank1, &mut bank2, &mut bank3].iter_mut())
        {
            let bank = bank.data();
            bank.indexed_deposits = I80F48::from(settings.deposits) / bank.deposit_index;
            bank.indexed_borrows = I80F48::from(settings.borrows) / bank.borrow_index;
            if settings.deposit_weight_scale_start_quote > 0 {
                bank.deposit_weight_scale_start_quote =
                    settings.deposit_weight_scale_start_quote as f64;
            }
            if settings.borrow_weight_scale_start_quote > 0 {
                bank.borrow_weight_scale_start_quote =
                    settings.borrow_weight_scale_start_quote as f64;
            }
        }

        let mut oo1 = TestAccount::<OpenOrders>::new_zeroed();
        let serum3account1 = account.create_serum3_orders(2).unwrap();
        serum3account1.open_orders = oo1.pubkey;
        serum3account1.base_token_index = 4;
        serum3account1.quote_token_index = 1;
        oo1.data().native_pc_total = testcase.oo_1_2.0;
        oo1.data().native_coin_total = testcase.oo_1_2.1;

        let mut oo2 = TestAccount::<OpenOrders>::new_zeroed();
        let serum3account2 = account.create_serum3_orders(3).unwrap();
        serum3account2.open_orders = oo2.pubkey;
        serum3account2.base_token_index = 5;
        serum3account2.quote_token_index = 1;
        oo2.data().native_pc_total = testcase.oo_1_3.0;
        oo2.data().native_coin_total = testcase.oo_1_3.1;

        let mut perp1 = mock_perp_market(group, oracle2.pubkey, 5.0, 9, (0.2, 0.1), (0.05, 0.02));
        let perpaccount = account.ensure_perp_position(9, 1).unwrap().0;
        perpaccount.record_trade(
            perp1.data(),
            testcase.perp1.0,
            I80F48::from(testcase.perp1.1),
        );
        perpaccount.bids_base_lots = testcase.perp1.2;
        perpaccount.asks_base_lots = testcase.perp1.3;

        let oracle2_ai = oracle2.as_account_info();
        let ais = vec![
            bank1.as_account_info(),
            bank2.as_account_info(),
            bank3.as_account_info(),
            oracle1.as_account_info(),
            oracle2_ai.clone(),
            oracle3.as_account_info(),
            perp1.as_account_info(),
            oracle2_ai,
            oo1.as_account_info(),
            oo2.as_account_info(),
        ];

        let retriever = ScanningAccountRetriever::new_with_staleness(&ais, &group, None).unwrap();

        assert!(health_eq(
            compute_health(&account.borrow(), HealthType::Init, &retriever).unwrap(),
            testcase.expected_health
        ));
    }

    // Check some specific health constellations
    #[test]
    fn test_health1() {
        let base_price = 5.0;
        let base_lots_to_quote = 10.0 * base_price;
        let testcases = vec![
            TestHealth1Case { // 0
                token1: 100,
                token2: -10,
                oo_1_2: (20, 15),
                perp1: (3, -131, 7, 11),
                expected_health:
                    // for token1, including open orders (scenario: bids execute)
                    (100.0 + (20.0 + 15.0 * base_price)) * 0.8
                    // for token2
                    - 10.0 * base_price * 1.5
                    // for perp (scenario: bids execute)
                    + (3.0 + 7.0) * base_lots_to_quote * 0.8 + (-131.0 - 7.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case { // 1
                token1: -100,
                token2: 10,
                oo_1_2: (20, 15),
                perp1: (-10, -131, 7, 11),
                expected_health:
                    // for token1
                    -100.0 * 1.2
                    // for token2, including open orders (scenario: asks execute)
                    + (10.0 * base_price + (20.0 + 15.0 * base_price)) * 0.5
                    // for perp (scenario: asks execute)
                    + (-10.0 - 11.0) * base_lots_to_quote * 1.2 + (-131.0 + 11.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case {
                // 2: weighted positive perp pnl
                perp1: (-1, 100, 0, 0),
                expected_health: 0.95 * (100.0 - 1.2 * 1.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case {
                // 3: negative perp pnl is not weighted
                perp1: (1, -100, 0, 0),
                expected_health: -100.0 + 0.8 * 1.0 * base_lots_to_quote,
                ..Default::default()
            },
            TestHealth1Case {
                // 4: perp health
                perp1: (10, 100, 0, 0),
                expected_health: 0.95 * (100.0 + 0.8 * 10.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case {
                // 5: perp health
                perp1: (30, -100, 0, 0),
                expected_health: 0.95 * (-100.0 + 0.8 * 30.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case { // 6, reserved oo funds
                token1: -100,
                token2: -10,
                token3: -10,
                oo_1_2: (1, 1),
                oo_1_3: (1, 1),
                expected_health:
                    // tokens
                    -100.0 * 1.2 - 10.0 * 5.0 * 1.5 - 10.0 * 10.0 * 1.5
                    // oo_1_2 (-> token1)
                    + (1.0 + 5.0) * 1.2
                    // oo_1_3 (-> token1)
                    + (1.0 + 10.0) * 1.2,
                ..Default::default()
            },
            TestHealth1Case { // 7, reserved oo funds cross the zero balance level
                token1: -14,
                token2: -10,
                token3: -10,
                oo_1_2: (1, 1),
                oo_1_3: (1, 1),
                expected_health:
                    // tokens
                    -14.0 * 1.2 - 10.0 * 5.0 * 1.5 - 10.0 * 10.0 * 1.5
                    // oo_1_2 (-> token1)
                    + 3.0 * 1.2 + 3.0 * 0.8
                    // oo_1_3 (-> token1)
                    + 8.0 * 1.2 + 3.0 * 0.8,
                ..Default::default()
            },
            TestHealth1Case { // 8, reserved oo funds in a non-quote currency
                token1: -100,
                token2: -100,
                token3: -1,
                oo_1_2: (0, 0),
                oo_1_3: (10, 1),
                expected_health:
                    // tokens
                    -100.0 * 1.2 - 100.0 * 5.0 * 1.5 - 10.0 * 1.5
                    // oo_1_3 (-> token3)
                    + 10.0 * 1.5 + 10.0 * 0.5,
                ..Default::default()
            },
            TestHealth1Case { // 9, like 8 but oo_1_2 flips the oo_1_3 target
                token1: -100,
                token2: -100,
                token3: -1,
                oo_1_2: (100, 0),
                oo_1_3: (10, 1),
                expected_health:
                    // tokens
                    -100.0 * 1.2 - 100.0 * 5.0 * 1.5 - 10.0 * 1.5
                    // oo_1_2 (-> token1)
                    + 80.0 * 1.2 + 20.0 * 0.8
                    // oo_1_3 (-> token1)
                    + 20.0 * 0.8,
                ..Default::default()
            },
            TestHealth1Case { // 10, checking collateral limit
                token1: 100,
                token2: 100,
                token3: 100,
                bank_settings: [
                    BankSettings {
                        deposits: 100,
                        deposit_weight_scale_start_quote: 1000,
                        ..BankSettings::default()
                    },
                    BankSettings {
                        deposits: 1500,
                        deposit_weight_scale_start_quote: 1000 * 5,
                        ..BankSettings::default()
                    },
                    BankSettings {
                        deposits: 10000,
                        deposit_weight_scale_start_quote: 1000 * 10,
                        ..BankSettings::default()
                    },
                ],
                expected_health:
                    // token1
                    0.8 * 100.0
                    // token2
                    + 0.5 * 100.0 * 5.0 * (5000.0 / (1500.0 * 5.0))
                    // token3
                    + 0.5 * 100.0 * 10.0 * (10000.0 / (10000.0 * 10.0)),
                ..Default::default()
            },
            TestHealth1Case { // 11, checking borrow limit
                token1: -100,
                token2: -100,
                token3: -100,
                bank_settings: [
                    BankSettings {
                        borrows: 100,
                        borrow_weight_scale_start_quote: 1000,
                        ..BankSettings::default()
                    },
                    BankSettings {
                        borrows: 1500,
                        borrow_weight_scale_start_quote: 1000 * 5,
                        ..BankSettings::default()
                    },
                    BankSettings {
                        borrows: 10000,
                        borrow_weight_scale_start_quote: 1000 * 10,
                        ..BankSettings::default()
                    },
                ],
                expected_health:
                    // token1
                    -1.2 * 100.0
                    // token2
                    - 1.5 * 100.0 * 5.0 * (1500.0 * 5.0 / 5000.0)
                    // token3
                    - 1.5 * 100.0 * 10.0 * (10000.0 * 10.0 / 10000.0),
                ..Default::default()
            },
        ];

        for (i, testcase) in testcases.iter().enumerate() {
            println!("checking testcase {}", i);
            test_health1_runner(testcase);
        }
    }
}
