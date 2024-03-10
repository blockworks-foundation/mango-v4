/*!
 * This module deals with computing different types of health for a mango account.
 *
 * Health is a number in USD and represents a risk-engine assessment of the account's
 * positions and open orders. The larger the health the better. Negative health
 * often means some action is necessary or a limitation is placed on the user.
 *
 * The different types of health are described in the HealthType enum.
 *
 * The key struct in this module is HealthCache, typically constructed by the
 * new_health_cache() function. With it, the different health types can be
 * computed.
 *
 * The HealthCache holds the data it needs in TokenInfo, Serum3Info and PerpInfo.
 */

use anchor_lang::prelude::*;

use fixed::types::I80F48;

use crate::error::*;
use crate::i80f48::LowPrecisionDivision;
use crate::serum3_cpi::{OpenOrdersAmounts, OpenOrdersSlim};
use crate::state::{
    Bank, MangoAccountRef, PerpMarket, PerpMarketIndex, PerpPosition, Serum3MarketIndex,
    Serum3Orders, TokenIndex,
};

use super::*;

/// Information about prices for a bank or perp market.
#[derive(Clone, Debug)]
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
    now_ts: u64,
) -> Result<I80F48> {
    let retriever = new_fixed_order_account_retriever(ais, account)?;
    Ok(new_health_cache(account, &retriever, now_ts)?.health(health_type))
}

/// Compute health with an arbitrary AccountRetriever
pub fn compute_health(
    account: &MangoAccountRef,
    health_type: HealthType,
    retriever: &impl AccountRetriever,
    now_ts: u64,
) -> Result<I80F48> {
    Ok(new_health_cache(account, retriever, now_ts)?.health(health_type))
}

/// How much of a token can be taken away before health decreases to zero?
///
/// If health is negative, returns 0.
pub fn spot_amount_taken_for_health_zero(
    mut health: I80F48,
    starting_spot: I80F48,
    asset_weighted_price: I80F48,
    liab_weighted_price: I80F48,
) -> Result<I80F48> {
    if health <= 0 {
        return Ok(I80F48::ZERO);
    }

    let mut taken_spot = I80F48::ZERO;
    if starting_spot > 0 {
        if asset_weighted_price > 0 {
            let asset_max = health / asset_weighted_price;
            if asset_max <= starting_spot {
                return Ok(asset_max);
            }
        }
        taken_spot = starting_spot;
        health -= starting_spot * asset_weighted_price;
    }
    if health > 0 {
        require_gt!(liab_weighted_price, 0);
        taken_spot += health / liab_weighted_price;
    }
    Ok(taken_spot)
}

/// How much of a token can be gained before health increases to zero?
///
/// Returns 0 if health is positive.
pub fn spot_amount_given_for_health_zero(
    health: I80F48,
    starting_spot: I80F48,
    asset_weighted_price: I80F48,
    liab_weighted_price: I80F48,
) -> Result<I80F48> {
    // asset/liab prices are reversed intentionally
    spot_amount_taken_for_health_zero(
        -health,
        -starting_spot,
        liab_weighted_price,
        asset_weighted_price,
    )
}

#[derive(Clone, Debug)]
pub struct TokenInfo {
    pub token_index: TokenIndex,
    pub maint_asset_weight: I80F48,
    pub init_asset_weight: I80F48,
    pub init_scaled_asset_weight: I80F48,
    pub maint_liab_weight: I80F48,
    pub init_liab_weight: I80F48,
    pub init_scaled_liab_weight: I80F48,
    pub prices: Prices,

    /// Freely available spot balance for the token.
    ///
    /// Includes TokenPosition and free Serum3OpenOrders balances.
    /// Does not include perp upnl or Serum3 reserved amounts.
    pub balance_spot: I80F48,

    pub allow_asset_liquidation: bool,
}

/// Temporary value used during health computations
#[derive(Clone, Default)]
pub struct TokenBalance {
    /// Sum of token_info.balance_spot and perp health_unsettled_pnl balances
    pub spot_and_perp: I80F48,
}

#[derive(Clone, Default)]
pub struct TokenMaxReserved {
    /// The sum of serum-reserved amounts over all markets
    pub max_serum_reserved: I80F48,
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
    pub fn asset_weighted_price(&self, health_type: HealthType) -> I80F48 {
        self.asset_weight(health_type) * self.prices.asset(health_type)
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
    pub fn liab_weighted_price(&self, health_type: HealthType) -> I80F48 {
        self.liab_weight(health_type) * self.prices.liab(health_type)
    }

    #[inline(always)]
    pub fn health_contribution(&self, health_type: HealthType, balance: I80F48) -> I80F48 {
        let weighted_price = if balance.is_negative() {
            self.liab_weighted_price(health_type)
        } else {
            self.asset_weighted_price(health_type)
        };
        balance * weighted_price
    }
}

/// Information about reserved funds on Serum3 open orders accounts.
///
/// Note that all "free" funds on open orders accounts are added directly
/// to the token info. This is only about dealing with the reserved funds
/// that might end up as base OR quote tokens, depending on whether the
/// open orders execute on not.
#[derive(Clone, Debug)]
pub struct Serum3Info {
    // reserved amounts as stored on the open orders
    pub reserved_base: I80F48,
    pub reserved_quote: I80F48,

    // Reserved amounts, converted to the opposite token, while using the most extreme order price
    // May be zero if the extreme bid/ask price is not available (for orders placed in the past)
    pub reserved_base_as_quote_lowest_ask: I80F48,
    pub reserved_quote_as_base_highest_bid: I80F48,

    // Index into TokenInfos _not_ a TokenIndex
    pub base_info_index: usize,
    pub quote_info_index: usize,

    pub market_index: Serum3MarketIndex,

    /// The open orders account has no free or reserved funds
    pub has_zero_funds: bool,
}

impl Serum3Info {
    fn new(
        serum_account: &Serum3Orders,
        open_orders: &impl OpenOrdersAmounts,
        base_info_index: usize,
        quote_info_index: usize,
    ) -> Self {
        // track the reserved amounts
        let reserved_base = I80F48::from(open_orders.native_base_reserved());
        let reserved_quote = I80F48::from(open_orders.native_quote_reserved());

        let reserved_base_as_quote_lowest_ask =
            reserved_base * I80F48::from_num(serum_account.lowest_placed_ask);
        let reserved_quote_as_base_highest_bid =
            reserved_quote * I80F48::from_num(serum_account.highest_placed_bid_inv);

        Self {
            reserved_base,
            reserved_quote,
            reserved_base_as_quote_lowest_ask,
            reserved_quote_as_base_highest_bid,
            base_info_index,
            quote_info_index,
            market_index: serum_account.market_index,
            has_zero_funds: open_orders.native_base_total() == 0
                && open_orders.native_quote_total() == 0
                && open_orders.native_rebates() == 0,
        }
    }

    #[inline(always)]
    fn all_reserved_as_base(
        &self,
        health_type: HealthType,
        quote_info: &TokenInfo,
        base_info: &TokenInfo,
    ) -> I80F48 {
        let quote_asset = quote_info.prices.asset(health_type);
        let base_liab = base_info.prices.liab(health_type);
        let reserved_quote_as_base_oracle = (self.reserved_quote * quote_asset)
            .checked_div_f64_precision(base_liab)
            .unwrap();
        if self.reserved_quote_as_base_highest_bid != 0 {
            self.reserved_base
                + reserved_quote_as_base_oracle.min(self.reserved_quote_as_base_highest_bid)
        } else {
            self.reserved_base + reserved_quote_as_base_oracle
        }
    }

    #[inline(always)]
    fn all_reserved_as_quote(
        &self,
        health_type: HealthType,
        quote_info: &TokenInfo,
        base_info: &TokenInfo,
    ) -> I80F48 {
        let base_asset = base_info.prices.asset(health_type);
        let quote_liab = quote_info.prices.liab(health_type);
        let reserved_base_as_quote_oracle = (self.reserved_base * base_asset)
            .checked_div_f64_precision(quote_liab)
            .unwrap();
        if self.reserved_base_as_quote_lowest_ask != 0 {
            self.reserved_quote
                + reserved_base_as_quote_oracle.min(self.reserved_base_as_quote_lowest_ask)
        } else {
            self.reserved_quote + reserved_base_as_quote_oracle
        }
    }

    /// Compute the health contribution from active open orders.
    ///
    /// For open orders, health is about the worst-case outcome: Consider the scenarios:
    /// - all reserved base tokens convert to quote tokens
    /// - all reserved quote tokens convert to base tokens
    /// Which would lead to the smaller token health?
    ///
    /// Answering this question isn't straightforward for two reasons:
    /// 1. We don't have information about the actual open orders here. Just about the amount
    ///    of reserved tokens. Hence we assume base/quote conversion would happen at current
    ///    asset/liab prices.
    /// 2. Technically, there are interaction effects between multiple spot markets. If the
    ///    account has open orders on SOL/USDC, BTC/USDC and SOL/BTC, then the worst case for
    ///    SOL/USDC might be dependent on what happens with the open orders on the other two
    ///    markets.
    ///
    /// To simplify 2, we give up on computing the actual worst-case and instead compute something
    /// that's guaranteed to be less: Get the worst case for each market independently while
    /// assuming all other market open orders resolved maximally unfavorably.
    ///
    /// To be able to do that, we compute `token_max_reserved` for each token, which is the maximum
    /// token amount that would be generated if open orders in all markets that deal with the token
    /// turn its way. (in the example above: the open orders in the SOL/USDC and SOL/BTC market
    /// both produce SOL) See `compute_serum3_reservations()` below.
    #[inline(always)]
    fn health_contribution(
        &self,
        health_type: HealthType,
        token_infos: &[TokenInfo],
        token_balances: &[TokenBalance],
        token_max_reserved: &[TokenMaxReserved],
        market_reserved: &Serum3Reserved,
    ) -> I80F48 {
        if market_reserved.all_reserved_as_base.is_zero()
            || market_reserved.all_reserved_as_quote.is_zero()
        {
            return I80F48::ZERO;
        }

        let base_info = &token_infos[self.base_info_index];
        let quote_info = &token_infos[self.quote_info_index];

        // How much would health increase if the reserved balance were applied to the passed
        // token info?
        let compute_health_effect = |token_info: &TokenInfo,
                                     balance: &TokenBalance,
                                     max_reserved: &TokenMaxReserved,
                                     market_reserved: I80F48| {
            // This balance includes all possible reserved funds from markets that relate to the
            // token, including this market itself: `market_reserved` is already included in `max_serum_reserved`.
            let max_balance = balance.spot_and_perp + max_reserved.max_serum_reserved;

            // For simplicity, we assume that `market_reserved` was added to `max_balance` last
            // (it underestimates health because that gives the smallest effects): how much did
            // health change because of it?
            let (asset_part, liab_part) = if max_balance >= market_reserved {
                (market_reserved, I80F48::ZERO)
            } else if max_balance.is_negative() {
                (I80F48::ZERO, market_reserved)
            } else {
                (max_balance, market_reserved - max_balance)
            };

            let asset_weight = token_info.asset_weight(health_type);
            let liab_weight = token_info.liab_weight(health_type);
            let asset_price = token_info.prices.asset(health_type);
            let liab_price = token_info.prices.liab(health_type);
            asset_part * asset_weight * asset_price + liab_part * liab_weight * liab_price
        };

        let health_base = compute_health_effect(
            base_info,
            &token_balances[self.base_info_index],
            &token_max_reserved[self.base_info_index],
            market_reserved.all_reserved_as_base,
        );
        let health_quote = compute_health_effect(
            quote_info,
            &token_balances[self.quote_info_index],
            &token_max_reserved[self.quote_info_index],
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

/// Stores information about perp market positions and their open orders.
///
/// Perp markets affect account health indirectly, though the token balance in the
/// perp market's settle token. See `effective_token_balances()`.
#[derive(Clone, Debug)]
pub struct PerpInfo {
    pub perp_market_index: PerpMarketIndex,
    pub settle_token_index: TokenIndex,
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
    pub base_prices: Prices,
    pub has_open_orders: bool,
    pub has_open_fills: bool,
}

impl PerpInfo {
    fn new(
        perp_position: &PerpPosition,
        perp_market: &PerpMarket,
        base_prices: Prices,
    ) -> Result<Self> {
        let base_lots = perp_position.base_position_lots() + perp_position.taker_base_lots;

        let unsettled_funding = perp_position.unsettled_funding(perp_market);
        let taker_quote = I80F48::from(perp_position.taker_quote_lots * perp_market.quote_lot_size);
        let quote_current = perp_position.quote_position_native() - unsettled_funding + taker_quote;

        Ok(Self {
            perp_market_index: perp_market.perp_market_index,
            settle_token_index: perp_market.settle_token_index,
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
            base_prices,
            has_open_orders: perp_position.has_open_orders(),
            has_open_fills: perp_position.has_open_taker_fills(),
        })
    }

    /// The perp-risk (but not token-risk) adjusted upnl. Also called "hupnl".
    ///
    /// In settle token native units.
    ///
    /// This is what gets added to effective_token_balances() and then contributes
    /// to account health.
    ///
    /// For fully isolated perp markets, users may never borrow against unsettled
    /// positive perp pnl, there overall_asset_weight == 0 and there can't be positive
    /// health contributions from these perp market. We sometimes call these markets
    /// "untrusted markets".
    ///
    /// In these, users need to settle their perp pnl with other perp market participants
    /// in order to realize their gains if they want to use them as collateral.
    ///
    /// This is because we don't trust the perp's base price to not suddenly jump to
    /// zero (if users could borrow against their perp balances they might now
    /// be bankrupt) or suddenly increase a lot (if users could borrow against perp
    /// balances they could now borrow other assets).
    ///
    /// Other markets may be liquid enough that we have enough confidence to allow
    /// users to borrow against unsettled positive pnl to some extend. In these cases,
    /// the overall asset weights would be >0.
    #[inline(always)]
    pub fn health_unsettled_pnl(&self, health_type: HealthType) -> I80F48 {
        let contribution = self.unweighted_health_unsettled_pnl(health_type);
        self.weigh_uhupnl_overall(contribution, health_type)
    }

    /// Convert uhupnl to hupnl by applying the overall weight. In settle token native units.
    #[inline(always)]
    fn weigh_uhupnl_overall(&self, unweighted: I80F48, health_type: HealthType) -> I80F48 {
        if unweighted > 0 {
            let overall_weight = match health_type {
                HealthType::Init | HealthType::LiquidationEnd => self.init_overall_asset_weight,
                HealthType::Maint => self.maint_overall_asset_weight,
            };
            overall_weight * unweighted
        } else {
            unweighted
        }
    }

    /// Settle token native provided by perp position and open orders, without the overall asset weight.
    ///
    /// Also called "uhupnl".
    ///
    /// For open orders, this computes the worst-case amount by considering the scenario where all
    /// bids execute and the one where all asks execute.
    ///
    /// It's always less than the PerpPosition's `unsettled_pnl()` for two reasons: The open orders
    /// are taken into account and the base weight is applied to the base position.
    ///
    /// Generally: hupnl <= uhupnl <= upnl
    #[inline(always)]
    pub fn unweighted_health_unsettled_pnl(&self, health_type: HealthType) -> I80F48 {
        let order_execution_case = |orders_base_lots: i64, order_price: I80F48| {
            let net_base_native =
                I80F48::from((self.base_lots + orders_base_lots) * self.base_lot_size);
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
                self.base_prices.liab(health_type)
            } else {
                self.base_prices.asset(health_type)
            };

            // Total value of the order-execution adjusted base position
            let base_health = net_base_native * weight * base_price;

            let orders_base_native = I80F48::from(orders_base_lots * self.base_lot_size);
            // The quote change from executing the bids/asks
            let order_quote = -orders_base_native * order_price;

            base_health + order_quote
        };

        // What is worse: Executing all bids at oracle_price.liab, or executing all asks at oracle_price.asset?
        let bids_case =
            order_execution_case(self.bids_base_lots, self.base_prices.liab(health_type));
        let asks_case =
            order_execution_case(-self.asks_base_lots, self.base_prices.asset(health_type));
        let worst_case = bids_case.min(asks_case);

        self.quote + worst_case
    }
}

/// Store information needed to compute account health
///
/// This is called a cache, because it extracts information from a MangoAccount and
/// the Bank, Perp, oracle accounts once and then allows computing different types
/// of health.
///
/// For compute-saving reasons, it also allows applying adjustments to the extracted
/// positions. That's often helpful for instructions that want to re-compute health
/// after having made small, well-known changes to an account. Recomputing the
/// HealthCache from scratch would be significantly more expensive.
///
/// However, there's a real risk of getting the adjustments wrong and computing an
/// inconsistent result, so particular care needs to be taken when this is done.
#[allow(unused)]
#[derive(Clone, Debug)]
pub struct HealthCache {
    pub token_infos: Vec<TokenInfo>,
    pub(crate) serum3_infos: Vec<Serum3Info>,
    pub(crate) perp_infos: Vec<PerpInfo>,
    #[allow(unused)]
    pub(crate) being_liquidated: bool,
}

impl HealthCache {
    pub fn health(&self, health_type: HealthType) -> I80F48 {
        let token_balances = self.effective_token_balances(health_type);
        let mut health = I80F48::ZERO;
        let sum = |contrib| {
            health += contrib;
        };
        self.health_sum(health_type, sum, &token_balances);
        health
    }

    /// The health ratio is
    /// - 0 if health is 0 - meaning assets = liabs
    /// - 100 if there's 2x as many assets as liabs
    /// - 200 if there's 3x as many assets as liabs
    /// - MAX if liabs = 0
    ///
    /// Maybe talking about the collateralization ratio assets/liabs is more intuitive?
    pub fn health_ratio(&self, health_type: HealthType) -> I80F48 {
        let (assets, liabs) = self.health_assets_and_liabs_stable_liabs(health_type);
        let hundred = I80F48::from(100);
        if liabs > 0 {
            // feel free to saturate to MAX for tiny liabs
            (hundred * (assets - liabs)).saturating_div(liabs)
        } else {
            I80F48::MAX
        }
    }

    pub fn health_assets_and_liabs_stable_assets(
        &self,
        health_type: HealthType,
    ) -> (I80F48, I80F48) {
        self.health_assets_and_liabs(health_type, true)
    }
    pub fn health_assets_and_liabs_stable_liabs(
        &self,
        health_type: HealthType,
    ) -> (I80F48, I80F48) {
        self.health_assets_and_liabs(health_type, false)
    }

    /// Loop over the token, perp, serum contributions and add up all positive values into `assets`
    /// and (the abs) of negative values separately into `liabs`. Return (assets, liabs).
    ///
    /// Due to the way token and perp positions sum before being weighted, there's some flexibility
    /// in how the sum is split up. It can either be split up such that the amount of liabs stays
    /// constant when assets change, or the other way around.
    ///
    /// For example, if assets are held stable: An account with $10 in SOL and -$12 hupnl in a
    /// SOL-settled perp market would have:
    /// - assets: $10 * SOL_asset_weight
    /// - liabs: $10 * SOL_asset_weight + $2 * SOL_liab_weight
    /// because some of the liabs are weighted lower as they are just compensating the assets.
    ///
    /// Same example if liabs are held stable:
    /// - liabs: $12 * SOL_liab_weight
    /// - assets: $10 * SOL_liab_weight
    ///
    /// The value `assets - liabs` is the health and the same in both cases.
    fn health_assets_and_liabs(
        &self,
        health_type: HealthType,
        stable_assets: bool,
    ) -> (I80F48, I80F48) {
        let mut total_assets = I80F48::ZERO;
        let mut total_liabs = I80F48::ZERO;
        let add = |assets: &mut I80F48, liabs: &mut I80F48, value: I80F48| {
            if value > 0 {
                *assets += value;
            } else {
                *liabs += -value;
            }
        };

        for token_info in self.token_infos.iter() {
            // For each token, health only considers the effective token position. But for
            // this function we want to distinguish the contribution from token deposits from
            // contributions by perp markets.
            // However, the overall weight is determined by the sum, so first collect all
            // assets parts and all liab parts and then determine the actual values.
            let mut asset_balance = I80F48::ZERO;
            let mut liab_balance = I80F48::ZERO;

            add(
                &mut asset_balance,
                &mut liab_balance,
                token_info.balance_spot,
            );

            for perp_info in self.perp_infos.iter() {
                if perp_info.settle_token_index != token_info.token_index {
                    continue;
                }
                let health_unsettled = perp_info.health_unsettled_pnl(health_type);
                add(&mut asset_balance, &mut liab_balance, health_unsettled);
            }

            // The assignment to total_assets and total_liabs is a bit arbitrary.
            // As long as the (added_assets - added_liabs) = weighted(asset_balance - liab_balance),
            // the result will be consistent.
            if stable_assets {
                let asset_weighted_price = token_info.asset_weighted_price(health_type);
                let assets = asset_balance * asset_weighted_price;
                total_assets += assets;
                if asset_balance >= liab_balance {
                    // liabs partially compensate
                    total_liabs += liab_balance * asset_weighted_price;
                } else {
                    let liab_weighted_price = token_info.liab_weighted_price(health_type);
                    // the liabs fully compensate the assets and even add something extra
                    total_liabs += assets + (liab_balance - asset_balance) * liab_weighted_price;
                }
            } else {
                let liab_weighted_price = token_info.liab_weighted_price(health_type);
                let liabs = liab_balance * liab_weighted_price;
                total_liabs += liabs;
                if asset_balance >= liab_balance {
                    let asset_weighted_price = token_info.asset_weighted_price(health_type);
                    // the assets fully compensate the liabs and even add something extra
                    total_assets += liabs + (asset_balance - liab_balance) * asset_weighted_price;
                } else {
                    // assets partially compensate
                    total_assets += asset_balance * liab_weighted_price;
                }
            }
        }

        let token_balances = self.effective_token_balances(health_type);
        let (token_max_reserved, serum3_reserved) = self.compute_serum3_reservations(health_type);
        for (serum3_info, reserved) in self.serum3_infos.iter().zip(serum3_reserved.iter()) {
            let contrib = serum3_info.health_contribution(
                health_type,
                &self.token_infos,
                &token_balances,
                &token_max_reserved,
                reserved,
            );
            add(&mut total_assets, &mut total_liabs, contrib);
        }

        (total_assets, total_liabs)
    }

    /// Computes the account assets and liabilities marked to market.
    ///
    /// Contrary to health_assets_and_liabs, there's no health weighing or adjustment
    /// for stable prices. It uses oracle prices directly.
    ///
    /// Returns (assets, liabilities)
    pub fn assets_and_liabs(&self) -> (I80F48, I80F48) {
        let mut assets = I80F48::ZERO;
        let mut liabs = I80F48::ZERO;

        for token_info in self.token_infos.iter() {
            if token_info.balance_spot.is_negative() {
                liabs -= token_info.balance_spot * token_info.prices.oracle;
            } else {
                assets += token_info.balance_spot * token_info.prices.oracle;
            }
        }

        for serum_info in self.serum3_infos.iter() {
            let quote = &self.token_infos[serum_info.quote_info_index];
            let base = &self.token_infos[serum_info.base_info_index];
            assets += serum_info.reserved_base * base.prices.oracle;
            assets += serum_info.reserved_quote * quote.prices.oracle;
        }

        for perp_info in self.perp_infos.iter() {
            let quote_price = self.token_infos[perp_info.settle_token_index as usize]
                .prices
                .oracle;
            let quote_position_value = perp_info.quote * quote_price;
            if perp_info.quote.is_negative() {
                liabs -= quote_position_value;
            } else {
                assets += quote_position_value;
            }

            let base_position_value = I80F48::from(perp_info.base_lots * perp_info.base_lot_size)
                * perp_info.base_prices.oracle
                * quote_price;
            if base_position_value.is_negative() {
                liabs -= base_position_value;
            } else {
                assets += base_position_value;
            }
        }

        return (assets, liabs);
    }

    /// Computes the account leverage as ratio of liabs / (assets - liabs)
    ///
    /// The goal of this function is to provide a quick overview over the accounts balance sheet.
    /// It's not actually used to make any margin decisions internally and doesn't account for
    /// open orders or stable / oracle price differences. Use health_ratio to make risk decisions.
    pub fn leverage(&self) -> I80F48 {
        let (assets, liabs) = self.assets_and_liabs();
        let equity = assets - liabs;
        liabs / equity.max(I80F48::from_num(0.001))
    }

    pub fn token_info(&self, token_index: TokenIndex) -> Result<&TokenInfo> {
        Ok(&self.token_infos[self.token_info_index(token_index)?])
    }

    pub fn token_info_index(&self, token_index: TokenIndex) -> Result<usize> {
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
        entry.balance_spot -= removed_contribution;
        Ok(())
    }

    /// Recompute the cached information about a serum market.
    ///
    /// WARNING: You must also call recompute_token_weights() after all bank
    /// deposit/withdraw changes!
    pub fn recompute_serum3_info(
        &mut self,
        serum_account: &Serum3Orders,
        open_orders: &OpenOrdersSlim,
        free_base_change: I80F48,
        free_quote_change: I80F48,
    ) -> Result<()> {
        let serum_info_index = self
            .serum3_infos
            .iter_mut()
            .position(|m| m.market_index == serum_account.market_index)
            .ok_or_else(|| error_msg!("serum3 market {} not found", serum_account.market_index))?;

        let serum_info = &self.serum3_infos[serum_info_index];
        {
            let base_entry = &mut self.token_infos[serum_info.base_info_index];
            base_entry.balance_spot += free_base_change;
        }
        {
            let quote_entry = &mut self.token_infos[serum_info.quote_info_index];
            quote_entry.balance_spot += free_quote_change;
        }

        let serum_info = &mut self.serum3_infos[serum_info_index];
        *serum_info = Serum3Info::new(
            serum_account,
            open_orders,
            serum_info.base_info_index,
            serum_info.quote_info_index,
        );
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
        *perp_entry = PerpInfo::new(perp_position, perp_market, perp_entry.base_prices.clone())?;
        Ok(())
    }

    /// Liquidatable spot assets mean: actual token deposits and also a positive effective token balance
    /// and is available for asset liquidation
    pub fn has_liq_spot_assets(&self) -> bool {
        let health_token_balances = self.effective_token_balances(HealthType::LiquidationEnd);
        self.token_infos
            .iter()
            .zip(health_token_balances.iter())
            .any(|(ti, b)| {
                // need 1 native token to use token_liq_with_token
                ti.balance_spot >= 1 && b.spot_and_perp >= 1 && ti.allow_asset_liquidation
            })
    }

    /// Liquidatable spot borrows mean: actual token borrows plus a negative effective token balance
    pub fn has_liq_spot_borrows(&self) -> bool {
        let health_token_balances = self.effective_token_balances(HealthType::LiquidationEnd);
        self.token_infos
            .iter()
            .zip(health_token_balances.iter())
            .any(|(ti, b)| ti.balance_spot < 0 && b.spot_and_perp < 0)
    }

    // This function exists separately from has_liq_spot_assets and has_liq_spot_borrows for performance reasons
    pub fn has_possible_spot_liquidations(&self) -> bool {
        let health_token_balances = self.effective_token_balances(HealthType::LiquidationEnd);
        let all_iter = || self.token_infos.iter().zip(health_token_balances.iter());
        all_iter().any(|(ti, b)| ti.balance_spot < 0 && b.spot_and_perp < 0)
            && all_iter().any(|(ti, b)| {
                ti.balance_spot >= 1 && b.spot_and_perp >= 1 && ti.allow_asset_liquidation
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

    pub fn has_perp_negative_pnl_no_base(&self) -> bool {
        self.perp_infos
            .iter()
            .any(|p| p.base_lots == 0 && p.quote < 0)
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
        self.has_possible_spot_liquidations()
            || self.has_perp_base_positions()
            || self.has_perp_open_fills()
            || self.has_perp_positive_pnl_no_base()
    }

    pub fn require_after_phase2_liquidation(&self) -> Result<()> {
        self.require_after_phase1_liquidation()?;
        require!(
            !self.has_possible_spot_liquidations(),
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
        self.has_liq_spot_borrows() || self.has_perp_negative_pnl_no_base()
    }

    pub fn in_phase3_liquidation(&self) -> bool {
        !self.has_phase1_liquidatable()
            && !self.has_phase2_liquidatable()
            && self.has_phase3_liquidatable()
    }

    pub(crate) fn compute_serum3_reservations(
        &self,
        health_type: HealthType,
    ) -> (Vec<TokenMaxReserved>, Vec<Serum3Reserved>) {
        let mut token_max_reserved = vec![TokenMaxReserved::default(); self.token_infos.len()];

        // For each serum market, compute what happened if reserved_base was converted to quote
        // or reserved_quote was converted to base.
        let mut serum3_reserved = Vec::with_capacity(self.serum3_infos.len());

        for info in self.serum3_infos.iter() {
            let quote_info = &self.token_infos[info.quote_info_index];
            let base_info = &self.token_infos[info.base_info_index];

            let all_reserved_as_base =
                info.all_reserved_as_base(health_type, quote_info, base_info);
            let all_reserved_as_quote =
                info.all_reserved_as_quote(health_type, quote_info, base_info);

            token_max_reserved[info.base_info_index].max_serum_reserved += all_reserved_as_base;
            token_max_reserved[info.quote_info_index].max_serum_reserved += all_reserved_as_quote;

            serum3_reserved.push(Serum3Reserved {
                all_reserved_as_base,
                all_reserved_as_quote,
            });
        }

        (token_max_reserved, serum3_reserved)
    }

    /// Returns token balances that account for spot and perp contributions
    ///
    /// Spot contributions are just the regular deposits or borrows, as well as from free
    /// funds on serum3 open orders accounts.
    ///
    /// Perp contributions come from perp positions in markets that use the token as a settle token:
    /// For these the hupnl is added to the total because that's the risk-adjusted expected to be
    /// gained or lost from settlement.
    pub fn effective_token_balances(&self, health_type: HealthType) -> Vec<TokenBalance> {
        self.effective_token_balances_internal(health_type, false)
    }

    /// Implementation of effective_token_balances()
    ///
    /// The ignore_negative_perp flag exists for perp_max_settle(). When it is enabled, all negative
    /// token contributions from perp markets are ignored. That's useful for knowing how much token
    /// collateral is available when limiting negative upnl settlement.
    fn effective_token_balances_internal(
        &self,
        health_type: HealthType,
        ignore_negative_perp: bool,
    ) -> Vec<TokenBalance> {
        let mut token_balances = vec![TokenBalance::default(); self.token_infos.len()];

        for perp_info in self.perp_infos.iter() {
            let settle_token_index = self.token_info_index(perp_info.settle_token_index).unwrap();
            let perp_settle_token = &mut token_balances[settle_token_index];
            let health_unsettled = perp_info.health_unsettled_pnl(health_type);
            if !ignore_negative_perp || health_unsettled > 0 {
                perp_settle_token.spot_and_perp += health_unsettled;
            }
        }

        for (token_info, token_balance) in self.token_infos.iter().zip(token_balances.iter_mut()) {
            token_balance.spot_and_perp += token_info.balance_spot;
        }

        token_balances
    }

    pub(crate) fn health_sum(
        &self,
        health_type: HealthType,
        mut action: impl FnMut(I80F48),
        token_balances: &[TokenBalance],
    ) {
        for (token_info, token_balance) in self.token_infos.iter().zip(token_balances.iter()) {
            let contrib = token_info.health_contribution(health_type, token_balance.spot_and_perp);
            action(contrib);
        }

        let (token_max_reserved, serum3_reserved) = self.compute_serum3_reservations(health_type);
        for (serum3_info, reserved) in self.serum3_infos.iter().zip(serum3_reserved.iter()) {
            let contrib = serum3_info.health_contribution(
                health_type,
                &self.token_infos,
                &token_balances,
                &token_max_reserved,
                reserved,
            );
            action(contrib);
        }
    }

    /// Returns how much pnl is settleable for a given settle token.
    ///
    /// The idea of this limit is that settlement is only permissible as long as there are
    /// non-perp assets that back it. If an account with 1 USD deposited somehow gets
    /// a large negative perp upnl, it should not be allowed to settle that perp loss into
    /// the spot world fully (because of perp/spot isolation, translating perp losses and
    /// gains into tokens is restricted). Only 1 USD worth would be allowed.
    ///
    /// Effectively, there's a health variant "perp settle health" that ignores negative
    /// token contributions from perp markets. Settlement is allowed as long as perp settle
    /// health remains >= 0.
    ///
    /// For example, if perp_settle_health is 50 USD, then the settleable amount in SOL
    /// would depend on the SOL price, the user's current spot balance and the SOL weights:
    /// We need to compute how much the user's spot SOL balance may decrease before the
    /// perp_settle_health becomes zero.
    ///
    /// Note that the account's actual health would not change during settling negative upnl:
    /// the spot balance goes down but the perp hupnl goes up accordingly.
    ///
    /// Examples:
    /// - An account may have maint_health < 0, but settling perp pnl could still be allowed.
    ///   (+100 USDC health, -50 USDT health, -50 perp health -> allow settling 50 health worth)
    /// - Positive health from trusted pnl markets counts
    /// - If overall health is 0 with two trusted perp pnl < 0, settling may still be possible.
    ///   (+100 USDC health, -150 perp1 health, -150 perp2 health -> allow settling 100 health worth)
    /// - Positive trusted perp pnl can enable settling.
    ///   (+100 trusted perp1 health, -100 perp2 health -> allow settling of 100 health worth)
    pub fn perp_max_settle(&self, settle_token_index: TokenIndex) -> Result<I80F48> {
        let maint_type = HealthType::Maint;

        let token_balances = self.effective_token_balances_internal(maint_type, true);
        let mut perp_settle_health = I80F48::ZERO;
        let sum = |contrib| {
            perp_settle_health += contrib;
        };
        self.health_sum(maint_type, sum, &token_balances);

        let token_info_index = self.token_info_index(settle_token_index)?;
        let token = &self.token_infos[token_info_index];
        spot_amount_taken_for_health_zero(
            perp_settle_health,
            token_balances[token_info_index].spot_and_perp,
            token.asset_weighted_price(maint_type),
            token.liab_weighted_price(maint_type),
        )
    }

    pub fn total_serum3_potential(
        &self,
        health_type: HealthType,
        token_index: TokenIndex,
    ) -> Result<I80F48> {
        let target_token_info_index = self.token_info_index(token_index)?;
        let total_reserved = self
            .serum3_infos
            .iter()
            .filter_map(|info| {
                if info.quote_info_index == target_token_info_index {
                    Some(info.all_reserved_as_quote(
                        health_type,
                        &self.token_infos[info.quote_info_index],
                        &self.token_infos[info.base_info_index],
                    ))
                } else if info.base_info_index == target_token_info_index {
                    Some(info.all_reserved_as_base(
                        health_type,
                        &self.token_infos[info.quote_info_index],
                        &self.token_infos[info.base_info_index],
                    ))
                } else {
                    None
                }
            })
            .sum();
        Ok(total_reserved)
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
    now_ts: u64,
) -> Result<HealthCache> {
    new_health_cache_impl(account, retriever, now_ts, false)
}

/// Generate a special HealthCache for an account and its health accounts
/// where nonnegative token positions for bad oracles are skipped.
///
/// This health cache must be used carefully, since it doesn't provide the actual
/// account health, just a value that is guaranteed to be less than it.
pub fn new_health_cache_skipping_bad_oracles(
    account: &MangoAccountRef,
    retriever: &impl AccountRetriever,
    now_ts: u64,
) -> Result<HealthCache> {
    new_health_cache_impl(account, retriever, now_ts, true)
}

fn new_health_cache_impl(
    account: &MangoAccountRef,
    retriever: &impl AccountRetriever,
    now_ts: u64,
    // If an oracle is stale or inconfident and the health contribution would
    // not be negative, skip it. This decreases health, but maybe overall it's
    // still positive?
    skip_bad_oracles: bool,
) -> Result<HealthCache> {
    // token contribution from token accounts
    let mut token_infos = Vec::with_capacity(account.active_token_positions().count());

    for (i, position) in account.active_token_positions().enumerate() {
        let bank_oracle_result =
            retriever.bank_and_oracle(&account.fixed.group, i, position.token_index);
        if skip_bad_oracles
            && bank_oracle_result.is_oracle_error()
            && position.indexed_position >= 0
        {
            // Ignore the asset because the oracle is bad, decreasing total health
            continue;
        }
        let (bank, oracle_price) = bank_oracle_result?;

        let native = position.native(bank);
        let prices = Prices {
            oracle: oracle_price,
            stable: bank.stable_price(),
        };
        // Use the liab price for computing weight scaling, because it's pessimistic and
        // causes the most unfavorable scaling.
        let liab_price = prices.liab(HealthType::Init);

        let (maint_asset_weight, maint_liab_weight) = bank.maint_weights(now_ts);

        token_infos.push(TokenInfo {
            token_index: bank.token_index,
            maint_asset_weight,
            init_asset_weight: bank.init_asset_weight,
            init_scaled_asset_weight: bank.scaled_init_asset_weight(liab_price),
            maint_liab_weight,
            init_liab_weight: bank.init_liab_weight,
            init_scaled_liab_weight: bank.scaled_init_liab_weight(liab_price),
            prices,
            balance_spot: native,
            allow_asset_liquidation: bank.allows_asset_liquidation(),
        });
    }

    // Fill the TokenInfo balance with free funds in serum3 oo accounts and build Serum3Infos.
    let mut serum3_infos = Vec::with_capacity(account.active_serum3_orders().count());
    for (i, serum_account) in account.active_serum3_orders().enumerate() {
        let oo = retriever.serum_oo(i, &serum_account.open_orders)?;

        // find the TokenInfos for the market's base and quote tokens
        let base_info_index = find_token_info_index(&token_infos, serum_account.base_token_index)?;
        let quote_info_index =
            find_token_info_index(&token_infos, serum_account.quote_token_index)?;

        // add the amounts that are freely settleable immediately to token balances
        let base_free = I80F48::from(oo.native_coin_free);
        let quote_free = I80F48::from(oo.native_pc_free);
        let base_info = &mut token_infos[base_info_index];
        base_info.balance_spot += base_free;
        let quote_info = &mut token_infos[quote_info_index];
        quote_info.balance_spot += quote_free;

        serum3_infos.push(Serum3Info::new(
            serum_account,
            oo,
            base_info_index,
            quote_info_index,
        ));
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

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 0, 1.0, 0.2, 0.1);
        let (mut bank2, mut oracle2) = mock_bank_and_oracle(group, 4, 5.0, 0.5, 0.3);
        bank1
            .data()
            .deposit(
                account.ensure_token_position(0).unwrap().0,
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
            )
            .unwrap();

        let mut oo1 = TestAccount::<OpenOrders>::new_zeroed();
        let serum3account = account.create_serum3_orders(2).unwrap();
        serum3account.open_orders = oo1.pubkey;
        serum3account.base_token_index = 4;
        serum3account.quote_token_index = 0;
        oo1.data().native_pc_total = 21;
        oo1.data().native_coin_total = 18;
        oo1.data().native_pc_free = 1;
        oo1.data().native_coin_free = 3;
        oo1.data().referrer_rebates_accrued = 2;

        let mut perp1 = mock_perp_market(group, oracle2.pubkey, 5.0, 9, (0.2, 0.1), (0.05, 0.02));
        let perpaccount = account.ensure_perp_position(9, 0).unwrap().0;
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

        // for bank1/oracle1
        // including open orders (scenario: bids execute)
        let serum1 = 1.0 + (20.0 + 15.0 * 5.0);
        // and perp (scenario: bids execute)
        let perp1 =
            (3.0 + 7.0 + 1.0) * 10.0 * 5.0 * 0.8 + (-310.0 + 2.0 * 100.0 - 7.0 * 10.0 * 5.0);
        let health1 = (100.0 + serum1 + perp1) * 0.8;
        // for bank2/oracle2
        let health2 = (-10.0 + 3.0) * 5.0 * 1.5;
        assert!(health_eq(
            compute_health(&account.borrow(), HealthType::Init, &retriever, 0).unwrap(),
            health1 + health2
        ));
    }

    #[derive(Default)]
    struct BankSettings {
        deposits: u64,
        borrows: u64,
        deposit_weight_scale_start_quote: u64,
        borrow_weight_scale_start_quote: u64,
        potential_serum_tokens: u64,
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
        extra: Option<fn(&mut MangoAccountValue)>,
    }
    fn test_health1_runner(testcase: &TestHealth1Case) {
        let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut account = MangoAccountValue::from_bytes(&buffer).unwrap();

        let group = Pubkey::new_unique();

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 0, 1.0, 0.2, 0.1);
        let (mut bank2, mut oracle2) = mock_bank_and_oracle(group, 4, 5.0, 0.5, 0.3);
        let (mut bank3, mut oracle3) = mock_bank_and_oracle(group, 5, 10.0, 0.5, 0.3);
        bank1
            .data()
            .change_without_fee(
                account.ensure_token_position(0).unwrap().0,
                I80F48::from(testcase.token1),
                DUMMY_NOW_TS,
            )
            .unwrap();
        bank2
            .data()
            .change_without_fee(
                account.ensure_token_position(4).unwrap().0,
                I80F48::from(testcase.token2),
                DUMMY_NOW_TS,
            )
            .unwrap();
        bank3
            .data()
            .change_without_fee(
                account.ensure_token_position(5).unwrap().0,
                I80F48::from(testcase.token3),
                DUMMY_NOW_TS,
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
            bank.potential_serum_tokens = settings.potential_serum_tokens;
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
        serum3account1.quote_token_index = 0;
        oo1.data().native_pc_total = testcase.oo_1_2.0;
        oo1.data().native_coin_total = testcase.oo_1_2.1;

        let mut oo2 = TestAccount::<OpenOrders>::new_zeroed();
        let serum3account2 = account.create_serum3_orders(3).unwrap();
        serum3account2.open_orders = oo2.pubkey;
        serum3account2.base_token_index = 5;
        serum3account2.quote_token_index = 0;
        oo2.data().native_pc_total = testcase.oo_1_3.0;
        oo2.data().native_coin_total = testcase.oo_1_3.1;

        let mut perp1 = mock_perp_market(group, oracle2.pubkey, 5.0, 9, (0.2, 0.1), (0.05, 0.02));
        let perpaccount = account.ensure_perp_position(9, 0).unwrap().0;
        perpaccount.record_trade(
            perp1.data(),
            testcase.perp1.0,
            I80F48::from(testcase.perp1.1),
        );
        perpaccount.bids_base_lots = testcase.perp1.2;
        perpaccount.asks_base_lots = testcase.perp1.3;

        if let Some(extra_fn) = testcase.extra {
            extra_fn(&mut account);
        }

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
            compute_health(&account.borrow(), HealthType::Init, &retriever, 0).unwrap(),
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
                    // for token1
                    0.8 * (100.0
                    // including open orders (scenario: bids execute)
                    + (20.0 + 15.0 * base_price)
                    // including perp (scenario: bids execute)
                    + (3.0 + 7.0) * base_lots_to_quote * 0.8 + (-131.0 - 7.0 * base_lots_to_quote))
                    // for token2
                    - 10.0 * base_price * 1.5,
                ..Default::default()
            },
            TestHealth1Case { // 1
                token1: -100,
                token2: 10,
                oo_1_2: (20, 15),
                perp1: (-10, -131, 7, 11),
                expected_health:
                    // for token1
                    1.2 * (-100.0
                    // for perp (scenario: asks execute)
                    + (-10.0 - 11.0) * base_lots_to_quote * 1.2 + (-131.0 + 11.0 * base_lots_to_quote))
                    // for token2, including open orders (scenario: asks execute)
                    + (10.0 * base_price + (20.0 + 15.0 * base_price)) * 0.5,
                ..Default::default()
            },
            TestHealth1Case {
                // 2: weighted positive perp pnl
                perp1: (-1, 100, 0, 0),
                expected_health: 0.8 * 0.95 * (100.0 - 1.2 * 1.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case {
                // 3: negative perp pnl is not weighted (only the settle token weight)
                perp1: (1, -100, 0, 0),
                expected_health: 1.2 * (-100.0 + 0.8 * 1.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case {
                // 4: perp health
                perp1: (10, 100, 0, 0),
                expected_health: 0.8 * 0.95 * (100.0 + 0.8 * 10.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case {
                // 5: perp health
                perp1: (30, -100, 0, 0),
                expected_health: 0.8 * 0.95 * (-100.0 + 0.8 * 30.0 * base_lots_to_quote),
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
            TestHealth1Case {
                // 12: positive perp health offsets token borrow
                token1: -100,
                perp1: (1, 100, 0, 0),
                expected_health: 0.8 * (-100.0 + 0.95 * (100.0 + 0.8 * 1.0 * base_lots_to_quote)),
                ..Default::default()
            },
            TestHealth1Case {
                // 13: negative perp health offsets token deposit
                token1: 100,
                perp1: (-1, -100, 0, 0),
                expected_health: 1.2 * (100.0 - 100.0 - 1.2 * 1.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case {
                // 14, reserved oo funds with max bid/min ask
                token1: -100,
                token2: -10,
                token3: 0,
                oo_1_2: (1, 1),
                oo_1_3: (11, 1),
                expected_health:
                    // tokens
                    -100.0 * 1.2 - 10.0 * 5.0 * 1.5
                    // oo_1_2 (-> token1)
                    + (1.0 + 3.0) * 1.2
                    // oo_1_3 (-> token3)
                    + (11.0 / 12.0 + 1.0) * 10.0 * 0.5,
                extra: Some(|account: &mut MangoAccountValue| {
                    let s2 = account.serum3_orders_mut(2).unwrap();
                    s2.lowest_placed_ask = 3.0;
                    let s3 = account.serum3_orders_mut(3).unwrap();
                    s3.highest_placed_bid_inv = 1.0 / 12.0;
                }),
                ..Default::default()
            },
            TestHealth1Case {
                // 15, reserved oo funds with max bid/min ask not crossing oracle
                token1: -100,
                token2: -10,
                token3: 0,
                oo_1_2: (1, 1),
                oo_1_3: (11, 1),
                expected_health:
                    // tokens
                    -100.0 * 1.2 - 10.0 * 5.0 * 1.5
                    // oo_1_2 (-> token1)
                    + (1.0 + 5.0) * 1.2
                    // oo_1_3 (-> token3)
                    + (11.0 / 10.0 + 1.0) * 10.0 * 0.5,
                extra: Some(|account: &mut MangoAccountValue| {
                    let s2 = account.serum3_orders_mut(2).unwrap();
                    s2.lowest_placed_ask = 6.0;
                    let s3 = account.serum3_orders_mut(3).unwrap();
                    s3.highest_placed_bid_inv = 1.0 / 9.0;
                }),
                ..Default::default()
            },
            TestHealth1Case {
                // 16, base case for 17
                token1: 100,
                token2: 100,
                token3: 100,
                oo_1_2: (0, 100),
                oo_1_3: (0, 100),
                expected_health:
                    // tokens
                    100.0 * 0.8 + 100.0 * 5.0 * 0.5 + 100.0 * 10.0 * 0.5
                    // oo_1_2 (-> token2)
                    + 100.0 * 5.0 * 0.5
                    // oo_1_3 (-> token1)
                    + 100.0 * 10.0 * 0.5,
                ..Default::default()
            },
            TestHealth1Case {
                // 17, potential_serum_tokens counts for deposit weight scaling
                token1: 100,
                token2: 100,
                token3: 100,
                oo_1_2: (0, 100),
                oo_1_3: (0, 100),
                bank_settings: [
                    BankSettings {
                        ..BankSettings::default()
                    },
                    BankSettings {
                        deposits: 100,
                        deposit_weight_scale_start_quote: 100 * 5,
                        potential_serum_tokens: 100,
                        ..BankSettings::default()
                    },
                    BankSettings {
                        deposits: 600,
                        deposit_weight_scale_start_quote: 500 * 10,
                        potential_serum_tokens: 100,
                        ..BankSettings::default()
                    },
                ],
                expected_health:
                    // tokens
                    100.0 * 0.8 + 100.0 * 5.0 * 0.5 * (100.0 / 200.0) + 100.0 * 10.0 * 0.5 * (500.0 / 700.0)
                    // oo_1_2 (-> token2)
                    + 100.0 * 5.0 * 0.5 * (100.0 / 200.0)
                    // oo_1_3 (-> token1)
                    + 100.0 * 10.0 * 0.5 * (500.0 / 700.0),
                ..Default::default()
            },
        ];

        for (i, testcase) in testcases.iter().enumerate() {
            println!("checking testcase {}", i);
            test_health1_runner(testcase);
        }
    }
}
