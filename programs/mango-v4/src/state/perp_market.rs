use std::mem::size_of;

use anchor_lang::prelude::*;
use fixed::types::I80F48;

use static_assertions::const_assert_eq;

use crate::accounts_zerocopy::KeyedAccountReader;
use crate::logs::PerpUpdateFundingLog;
use crate::state::orderbook::Side;
use crate::state::{oracle, TokenIndex};
use crate::util::checked_math as cm;

use super::{orderbook, OracleConfig, Orderbook, StablePriceModel, DAY_I80F48};

pub type PerpMarketIndex = u16;

#[account(zero_copy)]
#[derive(Debug)]
pub struct PerpMarket {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,

    pub settle_token_index: TokenIndex,

    /// Lookup indices
    pub perp_market_index: PerpMarketIndex,

    /// May this market contribute positive values to health?
    pub trusted_market: u8,

    /// Is this market covered by the group insurance fund?
    pub group_insurance_fund: u8,

    /// PDA bump
    pub bump: u8,

    pub base_decimals: u8,

    pub name: [u8; 16],

    pub orderbook: Pubkey,
    pub event_queue: Pubkey,

    pub oracle: Pubkey,
    pub oracle_config: OracleConfig,
    pub stable_price_model: StablePriceModel,

    /// Number of quote native that reresents min tick
    pub quote_lot_size: i64,
    /// Represents number of base native quantity
    /// e.g. if base decimals for underlying asset are 6, base lot size is 100, and base position is 10000, then
    /// UI position is 1
    pub base_lot_size: i64,

    // These weights apply to the base asset, the quote token is always assumed to be
    // the health-reference token and have 1 for price and weights
    pub maint_asset_weight: I80F48,
    pub init_asset_weight: I80F48,
    pub maint_liab_weight: I80F48,
    pub init_liab_weight: I80F48,

    pub open_interest: i64,

    /// Total number of orders seen
    pub seq_num: u64,

    pub registration_time: u64,

    /// Funding
    pub min_funding: I80F48,
    pub max_funding: I80F48,
    pub impact_quantity: i64,
    pub long_funding: I80F48,
    pub short_funding: I80F48,
    /// timestamp that funding was last updated in
    pub funding_last_updated: u64,

    /// Fees
    pub liquidation_fee: I80F48,
    pub maker_fee: I80F48,
    pub taker_fee: I80F48,
    /// Fees accrued in native quote currency
    pub fees_accrued: I80F48,
    /// Fees settled in native quote currency
    pub fees_settled: I80F48,
    pub fee_penalty: f32,

    // Settling incentives
    /// In native units of settlement token, given to each settle call above the
    /// settle_fee_amount_threshold.
    pub settle_fee_flat: f32,
    /// Pnl settlement amount needed to be eligible for fees.
    pub settle_fee_amount_threshold: f32,
    /// Fraction of pnl to pay out as fee if +pnl account has low health.
    pub settle_fee_fraction_low_health: f32,

    // Pnl settling limits
    /// Fraction of perp base value that can be settled each window.
    /// Set to a negative value to disable the limit.
    pub settle_pnl_limit_factor: f32,
    pub padding3: [u8; 4],
    /// Window size in seconds for the perp settlement limit
    pub settle_pnl_limit_window_size_ts: u64,

    pub reserved: [u8; 1944],
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
        + 1944
);
const_assert_eq!(size_of::<PerpMarket>(), 2776);
const_assert_eq!(size_of::<PerpMarket>() % 8, 0);

impl PerpMarket {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn elligible_for_group_insurance_fund(&self) -> bool {
        self.group_insurance_fund == 1
    }

    pub fn set_elligible_for_group_insurance_fund(&mut self, v: bool) {
        self.group_insurance_fund = if v { 1 } else { 0 };
    }

    pub fn trusted_market(&self) -> bool {
        self.trusted_market == 1
    }

    pub fn settle_pnl_limit_factor(&self) -> I80F48 {
        I80F48::from_num(self.settle_pnl_limit_factor)
    }

    pub fn gen_order_id(&mut self, side: Side, price_data: u64) -> u128 {
        self.seq_num += 1;
        orderbook::new_node_key(side, price_data, self.seq_num)
    }

    pub fn oracle_price(
        &self,
        oracle_acc: &impl KeyedAccountReader,
        staleness_slot: Option<u64>,
    ) -> Result<I80F48> {
        require_keys_eq!(self.oracle, *oracle_acc.key());
        oracle::oracle_price(
            oracle_acc,
            &self.oracle_config,
            self.base_decimals,
            staleness_slot,
        )
    }

    pub fn stable_price(&self) -> I80F48 {
        I80F48::from_num(self.stable_price_model.stable_price)
    }

    /// Use current order book price and index price to update the instantaneous funding
    pub fn update_funding_and_stable_price(
        &mut self,
        book: &Orderbook,
        oracle_price: I80F48,
        now_ts: u64,
    ) -> Result<()> {
        if now_ts <= self.funding_last_updated {
            return Ok(());
        }

        let index_price = oracle_price;
        let oracle_price_lots = self.native_price_to_lot(oracle_price);

        // Get current book price & compare it to index price
        let bid = book.impact_price(Side::Bid, self.impact_quantity, now_ts, oracle_price_lots);
        let ask = book.impact_price(Side::Ask, self.impact_quantity, now_ts, oracle_price_lots);

        let diff_price = match (bid, ask) {
            (Some(bid), Some(ask)) => {
                // calculate mid-market rate
                let mid_price = bid.checked_add(ask).unwrap() / 2;
                let book_price = self.lot_to_native_price(mid_price);
                let diff = cm!(book_price / index_price - I80F48::ONE);
                diff.clamp(self.min_funding, self.max_funding)
            }
            (Some(_bid), None) => self.max_funding,
            (None, Some(_ask)) => self.min_funding,
            (None, None) => I80F48::ZERO,
        };

        let diff_ts = I80F48::from_num(now_ts - self.funding_last_updated as u64);
        let time_factor = cm!(diff_ts / DAY_I80F48);
        let base_lot_size = I80F48::from_num(self.base_lot_size);
        let funding_delta = cm!(index_price * diff_price * base_lot_size * time_factor);

        self.long_funding += funding_delta;
        self.short_funding += funding_delta;
        self.funding_last_updated = now_ts;

        self.stable_price_model
            .update(now_ts, oracle_price.to_num());

        emit!(PerpUpdateFundingLog {
            mango_group: self.group,
            market_index: self.perp_market_index,
            long_funding: self.long_funding.to_bits(),
            short_funding: self.long_funding.to_bits(),
            price: oracle_price.to_bits(),
            stable_price: self.stable_price().to_bits(),
            fees_accrued: self.fees_accrued.to_bits(),
            open_interest: self.open_interest,
            instantaneous_funding_rate: diff_price.to_bits(),
        });

        Ok(())
    }

    /// Convert from the price stored on the book to the price used in value calculations
    pub fn lot_to_native_price(&self, price: i64) -> I80F48 {
        I80F48::from_num(price)
            .checked_mul(I80F48::from_num(self.quote_lot_size))
            .unwrap()
            .checked_div(I80F48::from_num(self.base_lot_size))
            .unwrap()
    }

    pub fn native_price_to_lot(&self, price: I80F48) -> i64 {
        price
            .checked_mul(I80F48::from_num(self.base_lot_size))
            .unwrap()
            .checked_div(I80F48::from_num(self.quote_lot_size))
            .unwrap()
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
            Side::Bid => native_price <= cm!(self.maint_liab_weight * oracle_price),
            Side::Ask => native_price >= cm!(self.maint_asset_weight * oracle_price),
        }
    }

    /// Socialize the loss in this account across all longs and shorts
    pub fn socialize_loss(&mut self, loss: I80F48) -> Result<I80F48> {
        require_gte!(0, loss);

        // TODO convert into only socializing on one side
        // native USDC per contract open interest
        let socialized_loss = if self.open_interest == 0 {
            // AUDIT: think about the following:
            // This is kind of an unfortunate situation. This means socialized loss occurs on the
            // last person to call settle_pnl on their profits. Any advice on better mechanism
            // would be appreciated. Luckily, this will be an extremely rare situation.
            I80F48::ZERO
        } else {
            cm!(loss / I80F48::from(self.open_interest))
        };
        self.long_funding -= socialized_loss;
        self.short_funding += socialized_loss;
        Ok(socialized_loss)
    }

    /// Creates default market for tests
    pub fn default_for_tests() -> PerpMarket {
        PerpMarket {
            group: Pubkey::new_unique(),
            settle_token_index: 0,
            perp_market_index: 0,
            trusted_market: 0,
            group_insurance_fund: 0,
            name: Default::default(),
            orderbook: Pubkey::new_unique(),
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
            maint_asset_weight: I80F48::from(1),
            init_asset_weight: I80F48::from(1),
            maint_liab_weight: I80F48::from(1),
            init_liab_weight: I80F48::from(1),
            open_interest: 0,
            seq_num: 0,
            bump: 0,
            base_decimals: 0,
            registration_time: 0,
            min_funding: I80F48::ZERO,
            max_funding: I80F48::ZERO,
            impact_quantity: 0,
            long_funding: I80F48::ZERO,
            short_funding: I80F48::ZERO,
            funding_last_updated: 0,
            liquidation_fee: I80F48::ZERO,
            maker_fee: I80F48::ZERO,
            taker_fee: I80F48::ZERO,
            fees_accrued: I80F48::ZERO,
            fees_settled: I80F48::ZERO,
            fee_penalty: 0.0,
            settle_fee_flat: 0.0,
            settle_fee_amount_threshold: 0.0,
            settle_fee_fraction_low_health: 0.0,
            padding3: Default::default(),
            settle_pnl_limit_factor: 0.2,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            reserved: [0; 1944],
        }
    }
}
