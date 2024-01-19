use std::mem::size_of;

use anchor_lang::prelude::*;
use derivative::Derivative;
use fixed::types::I80F48;

use oracle::oracle_log_context;
use static_assertions::const_assert_eq;

use crate::accounts_zerocopy::KeyedAccountReader;
use crate::error::{Contextable, MangoError};
use crate::logs::{emit_stack, PerpUpdateFundingLogV2};
use crate::state::orderbook::Side;
use crate::state::{oracle, TokenIndex};
use crate::util;

use super::{
    orderbook, OracleAccountInfos, OracleConfig, OracleState, Orderbook, StablePriceModel,
    DAY_I80F48,
};

pub type PerpMarketIndex = u16;

#[account(zero_copy)]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct PerpMarket {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,

    /// Token index that settlements happen in.
    ///
    /// Currently required to be 0, USDC. In the future settlement
    /// may be allowed to happen in other tokens.
    pub settle_token_index: TokenIndex,

    /// Index of this perp market. Other data, like the MangoAccount's PerpPosition
    /// reference this market via this index. Unique for this group's perp markets.
    pub perp_market_index: PerpMarketIndex,

    /// Field used to contain the trusted_market flag and is now unused.
    pub blocked1: u8,

    /// Is this market covered by the group insurance fund?
    pub group_insurance_fund: u8,

    /// PDA bump
    pub bump: u8,

    /// Number of decimals used for the base token.
    ///
    /// Used to convert the oracle's price into a native/native price.
    pub base_decimals: u8,

    /// Name. Trailing zero bytes are ignored.
    #[derivative(Debug(format_with = "util::format_zero_terminated_utf8_bytes"))]
    pub name: [u8; 16],

    /// Address of the BookSide account for bids
    pub bids: Pubkey,
    /// Address of the BookSide account for asks
    pub asks: Pubkey,
    /// Address of the EventQueue account
    pub event_queue: Pubkey,

    /// Oracle account address
    pub oracle: Pubkey,
    /// Oracle configuration
    pub oracle_config: OracleConfig,
    /// Maintains a stable price based on the oracle price that is less volatile.
    pub stable_price_model: StablePriceModel,

    /// Number of quote native in a quote lot. Must be a power of 10.
    ///
    /// Primarily useful for increasing the tick size on the market: A lot price
    /// of 1 becomes a native price of quote_lot_size/base_lot_size becomes a
    /// ui price of quote_lot_size*base_decimals/base_lot_size/quote_decimals.
    pub quote_lot_size: i64,

    /// Number of base native in a base lot. Must be a power of 10.
    ///
    /// Example: If base decimals for the underlying asset is 6, base lot size
    /// is 100 and and base position lots is 10_000 then base position native is
    /// 1_000_000 and base position ui is 1.
    pub base_lot_size: i64,

    /// These weights apply to the base position. The quote position has
    /// no explicit weight (but may be covered by the overall pnl asset weight).
    pub maint_base_asset_weight: I80F48,
    pub init_base_asset_weight: I80F48,
    pub maint_base_liab_weight: I80F48,
    pub init_base_liab_weight: I80F48,

    /// Number of base lots currently active in the market. Always >= 0.
    ///
    /// Since this counts positive base lots and negative base lots, the more relevant
    /// number of open base lot pairs is half this value.
    pub open_interest: i64,

    /// Total number of orders seen
    pub seq_num: u64,

    /// Timestamp in seconds that the market was registered at.
    pub registration_time: u64,

    // Funding
    /// Minimal funding rate per day, must be <= 0.
    pub min_funding: I80F48,
    /// Maximal funding rate per day, must be >= 0.
    pub max_funding: I80F48,
    /// For funding, get the impact price this many base lots deep into the book.
    pub impact_quantity: i64,

    /// Current long funding value. Increasing it means that every long base lot
    /// needs to pay that amount of quote native in funding.
    ///
    /// PerpPosition uses and tracks it settle funding. Updated by the perp
    /// keeper instruction.
    pub long_funding: I80F48,
    /// See long_funding.
    pub short_funding: I80F48,
    /// timestamp that funding was last updated in
    pub funding_last_updated: u64,

    /// Fees

    /// Fee for base position liquidation
    pub base_liquidation_fee: I80F48,
    /// Fee when matching maker orders. May be negative.
    pub maker_fee: I80F48,
    /// Fee for taker orders, may not be negative.
    pub taker_fee: I80F48,

    /// Fees accrued in native quote currency
    /// these are increased when new fees are paid and decreased when perp_settle_fees is called
    pub fees_accrued: I80F48,
    /// Fees settled in native quote currency
    /// these are increased when perp_settle_fees is called, and never decreased
    pub fees_settled: I80F48,

    /// Fee (in quote native) to charge for ioc orders
    pub fee_penalty: f32,

    // Settling incentives
    /// In native units of settlement token, given to each settle call above the
    /// settle_fee_amount_threshold if settling at least 1% of perp base pos value.
    pub settle_fee_flat: f32,
    /// Pnl settlement amount needed to be eligible for the flat fee.
    pub settle_fee_amount_threshold: f32,
    /// Fraction of pnl to pay out as fee if +pnl account has low health.
    /// (limited to 2x settle_fee_flat)
    pub settle_fee_fraction_low_health: f32,

    // Pnl settling limits
    /// Controls the strictness of the settle limit.
    /// Set to a negative value to disable the limit.
    ///
    /// This factor applies to the settle limit in two ways
    /// - for the unrealized pnl settle limit, the factor is multiplied with the stable perp base value
    ///   (i.e. limit_factor * base_native * stable_price)
    /// - when increasing the realized pnl settle limit (stored per PerpPosition), the factor is
    ///   multiplied with the stable value of the perp pnl being realized
    ///   (i.e. limit_factor * reduced_native * stable_price)
    ///
    /// See also PerpPosition::settle_pnl_limit_realized_trade
    pub settle_pnl_limit_factor: f32,

    #[derivative(Debug = "ignore")]
    pub padding3: [u8; 4],

    /// Window size in seconds for the perp settlement limit
    pub settle_pnl_limit_window_size_ts: u64,

    /// If true, users may no longer increase their market exposure. Only actions
    /// that reduce their position are still allowed.
    pub reduce_only: u8,
    pub force_close: u8,

    #[derivative(Debug = "ignore")]
    pub padding4: [u8; 6],

    /// Weights for full perp market health, if positive
    pub maint_overall_asset_weight: I80F48,
    pub init_overall_asset_weight: I80F48,

    pub positive_pnl_liquidation_fee: I80F48,

    // Do separate bookkeping for how many tokens were withdrawn
    // This ensures that fees_settled is strictly increasing for stats gathering purposes
    pub fees_withdrawn: u64,

    /// Additional to liquidation_fee, but goes to the group owner instead of the liqor
    pub platform_liquidation_fee: I80F48,

    /// Platform fees that were accrued during liquidation (in native tokens)
    ///
    /// These fees are also added to fees_accrued, this is just for bookkeeping the total
    /// liquidation fees that happened. So never decreases (different to fees_accrued).
    pub accrued_liquidation_fees: I80F48,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 1848],
}

const_assert_eq!(
    size_of::<PerpMarket>(),
    32 + 2
        + 2
        + 1
        + 1
        + 16
        + 32
        + 32
        + 32
        + 32
        + 96
        + 288
        + 8
        + 8
        + 16 * 4
        + 8
        + 8
        + 1
        + 1
        + 8
        + 16 * 2
        + 8
        + 16 * 2
        + 8
        + 16 * 5
        + 4
        + 4 * 3
        + 8
        + 8
        + 1
        + 7
        + 3 * 16
        + 8
        + 2 * 16
        + 1848
);
const_assert_eq!(size_of::<PerpMarket>(), 2808);
const_assert_eq!(size_of::<PerpMarket>() % 8, 0);

impl PerpMarket {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn is_reduce_only(&self) -> bool {
        self.reduce_only == 1
    }

    pub fn is_force_close(&self) -> bool {
        self.force_close == 1
    }

    pub fn elligible_for_group_insurance_fund(&self) -> bool {
        self.group_insurance_fund == 1
    }

    pub fn set_elligible_for_group_insurance_fund(&mut self, v: bool) {
        self.group_insurance_fund = u8::from(v);
    }

    pub fn settle_pnl_limit_factor(&self) -> I80F48 {
        I80F48::from_num(self.settle_pnl_limit_factor)
    }

    pub fn gen_order_id(&mut self, side: Side, price_data: u64) -> u128 {
        self.seq_num += 1;
        orderbook::new_node_key(side, price_data, self.seq_num)
    }

    pub fn oracle_price<T: KeyedAccountReader>(
        &self,
        oracle_acc_infos: &OracleAccountInfos<T>,
        staleness_slot: Option<u64>,
    ) -> Result<I80F48> {
        Ok(self.oracle_state(oracle_acc_infos, staleness_slot)?.price)
    }

    pub fn oracle_state<T: KeyedAccountReader>(
        &self,
        oracle_acc_infos: &OracleAccountInfos<T>,
        staleness_slot: Option<u64>,
    ) -> Result<OracleState> {
        require_keys_eq!(self.oracle, *oracle_acc_infos.oracle.key());
        let state = oracle::oracle_state_unchecked(oracle_acc_infos, self.base_decimals)?;
        state
            .check_confidence_and_maybe_staleness(&self.oracle_config, staleness_slot)
            .with_context(|| {
                oracle_log_context(self.name(), &state, &self.oracle_config, staleness_slot)
            })?;
        Ok(state)
    }

    pub fn stable_price(&self) -> I80F48 {
        I80F48::from_num(self.stable_price_model.stable_price)
    }

    /// Use current order book price and index price to update the instantaneous funding
    pub fn update_funding_and_stable_price(
        &mut self,
        book: &Orderbook,
        oracle_state: &OracleState,
        now_ts: u64,
    ) -> Result<()> {
        if now_ts <= self.funding_last_updated {
            return Ok(());
        }

        let oracle_price = oracle_state.price;
        let oracle_price_lots = self.native_price_to_lot(oracle_price);

        // Get current book price & compare it to index price
        let bid =
            book.bookside(Side::Bid)
                .impact_price(self.impact_quantity, now_ts, oracle_price_lots);
        let ask =
            book.bookside(Side::Ask)
                .impact_price(self.impact_quantity, now_ts, oracle_price_lots);

        let funding_rate = match (bid, ask) {
            (Some(bid), Some(ask)) => {
                // calculate mid-market rate
                let mid_price = (bid + ask) / 2;
                let book_price = self.lot_to_native_price(mid_price);
                let diff = book_price / oracle_price - I80F48::ONE;
                diff.clamp(self.min_funding, self.max_funding)
            }
            (Some(_bid), None) => self.max_funding,
            (None, Some(_ask)) => self.min_funding,
            (None, None) => I80F48::ZERO,
        };

        // Limit the maximal time interval that funding is applied for. This means we won't use
        // the funding rate computed from a single orderbook snapshot for a very long time period
        // in exceptional circumstances, like a solana downtime or the security council disabling
        // funding updates.
        let max_funding_timestep = 3600; // one hour
        let diff_ts =
            I80F48::from_num((now_ts - self.funding_last_updated as u64).min(max_funding_timestep));

        let time_factor = diff_ts / DAY_I80F48;
        let base_lot_size = I80F48::from_num(self.base_lot_size);

        // The number of native quote that one base lot should pay in funding
        let funding_delta = oracle_price * base_lot_size * funding_rate * time_factor;

        self.long_funding += funding_delta;
        self.short_funding += funding_delta;
        self.funding_last_updated = now_ts;

        self.stable_price_model
            .update(now_ts, oracle_price.to_num());

        emit_stack(PerpUpdateFundingLogV2 {
            mango_group: self.group,
            market_index: self.perp_market_index,
            long_funding: self.long_funding.to_bits(),
            short_funding: self.short_funding.to_bits(),
            price: oracle_price.to_bits(),
            oracle_slot: oracle_state.last_update_slot,
            oracle_confidence: oracle_state.deviation.to_bits(),
            oracle_type: oracle_state.oracle_type,
            stable_price: self.stable_price().to_bits(),
            fees_accrued: self.fees_accrued.to_bits(),
            fees_settled: self.fees_settled.to_bits(),
            open_interest: self.open_interest,
            instantaneous_funding_rate: funding_rate.to_bits(),
        });

        Ok(())
    }

    /// Convert from the price stored on the book to the price used in value calculations
    pub fn lot_to_native_price(&self, price: i64) -> I80F48 {
        I80F48::from_num(price) * I80F48::from_num(self.quote_lot_size)
            / I80F48::from_num(self.base_lot_size)
    }

    pub fn native_price_to_lot(&self, price: I80F48) -> i64 {
        (price * I80F48::from_num(self.base_lot_size) / I80F48::from_num(self.quote_lot_size))
            .to_num()
    }

    /// Is `native_price` an acceptable order for the `side` of this market, given `oracle_price`?
    pub fn inside_price_limit(
        &self,
        side: Side,
        native_price: I80F48,
        oracle_price: I80F48,
    ) -> bool {
        match side {
            Side::Bid => native_price <= (self.maint_base_liab_weight * oracle_price),
            Side::Ask => native_price >= (self.maint_base_asset_weight * oracle_price),
        }
    }

    /// Socialize the loss in this account across all longs and shorts
    ///
    /// `loss` is in settle token native units
    pub fn socialize_loss(&mut self, loss: I80F48) -> Result<I80F48> {
        require_gte!(0, loss);

        // TODO convert into only socializing on one side
        // native settle token per contract open interest
        let socialized_loss = if self.open_interest == 0 {
            // AUDIT: think about the following:
            // This is kind of an unfortunate situation. This means socialized loss occurs on the
            // last person to call settle_pnl on their profits. Any advice on better mechanism
            // would be appreciated. Luckily, this will be an extremely rare situation.
            I80F48::ZERO
        } else {
            loss / I80F48::from(self.open_interest)
        };
        self.long_funding -= socialized_loss;
        self.short_funding += socialized_loss;
        Ok(socialized_loss)
    }

    /// Returns the fee for settling `settlement` when the account with positive unsettled pnl
    /// has the given source pnl/position/health values.
    pub fn compute_settle_fee(
        &self,
        settlement: I80F48,
        source_pnl_value: I80F48,
        source_position_value: I80F48,
        source_liq_end_health: I80F48,
        source_maint_health: I80F48,
    ) -> Result<I80F48> {
        // Only incentivize if pnl is at least 1% of position.
        //
        // This avoids large positions being settled all the time when tiny price
        // movements can bring the settlement amount over the settle_fee_amount_threshold.
        //
        // Always true when the source position is closed.
        let pnl_at_least_one_percent = I80F48::from(100) * source_pnl_value > source_position_value;
        if !pnl_at_least_one_percent {
            return Ok(I80F48::ZERO);
        }

        assert!(source_maint_health >= source_liq_end_health);

        // A percentage fee is paid to the settler when the source account's health is low.
        // That's because the settlement could avoid it getting liquidated: settling will
        // increase its health by actualizing positive perp pnl.
        let low_health_fee = if source_liq_end_health < 0 {
            let fee_fraction = I80F48::from_num(self.settle_fee_fraction_low_health);
            if source_maint_health < 0 {
                settlement * fee_fraction
            } else {
                settlement
                    * fee_fraction
                    * (-source_liq_end_health / (source_maint_health - source_liq_end_health))
            }
        } else {
            I80F48::ZERO
        };

        let flat_fee = I80F48::from_num(self.settle_fee_flat);

        let mut fee = if settlement >= self.settle_fee_amount_threshold {
            // If the settlement is big enough: give the flat fee
            flat_fee
        } else {
            // Else give the low-health fee, but never more than twice flat fee
            low_health_fee.min(flat_fee * I80F48::from(2))
        };

        // Fee can't exceed the settlement (just for safety)
        fee = fee.min(settlement);

        // Safety check to prevent any accidental negative transfer
        require!(fee >= 0, MangoError::SettlementAmountMustBePositive);

        Ok(fee)
    }

    /// Creates default market for tests
    pub fn default_for_tests() -> PerpMarket {
        PerpMarket {
            group: Pubkey::new_unique(),
            settle_token_index: 0,
            perp_market_index: 0,
            blocked1: 0,
            group_insurance_fund: 0,
            bump: 0,
            base_decimals: 0,
            name: Default::default(),
            bids: Pubkey::new_unique(),
            asks: Pubkey::new_unique(),
            event_queue: Pubkey::new_unique(),
            oracle: Pubkey::new_unique(),
            oracle_config: OracleConfig {
                conf_filter: I80F48::ZERO,
                max_staleness_slots: -1,
                reserved: [0; 72],
            },
            stable_price_model: StablePriceModel::default(),
            quote_lot_size: 1,
            base_lot_size: 1,
            maint_base_asset_weight: I80F48::from(1),
            init_base_asset_weight: I80F48::from(1),
            maint_base_liab_weight: I80F48::from(1),
            init_base_liab_weight: I80F48::from(1),
            open_interest: 0,
            seq_num: 0,
            registration_time: 0,
            min_funding: I80F48::ZERO,
            max_funding: I80F48::ZERO,
            impact_quantity: 0,
            long_funding: I80F48::ZERO,
            short_funding: I80F48::ZERO,
            funding_last_updated: 0,
            base_liquidation_fee: I80F48::ZERO,
            maker_fee: I80F48::ZERO,
            taker_fee: I80F48::ZERO,
            fees_accrued: I80F48::ZERO,
            fees_settled: I80F48::ZERO,
            fee_penalty: 0.0,
            settle_fee_flat: 0.0,
            settle_fee_amount_threshold: 0.0,
            settle_fee_fraction_low_health: 0.0,
            settle_pnl_limit_factor: 0.2,
            padding3: Default::default(),
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            reduce_only: 0,
            force_close: 0,
            padding4: Default::default(),
            maint_overall_asset_weight: I80F48::ONE,
            init_overall_asset_weight: I80F48::ONE,
            positive_pnl_liquidation_fee: I80F48::ZERO,
            fees_withdrawn: 0,
            platform_liquidation_fee: I80F48::ZERO,
            accrued_liquidation_fees: I80F48::ZERO,
            reserved: [0; 1848],
        }
    }
}
