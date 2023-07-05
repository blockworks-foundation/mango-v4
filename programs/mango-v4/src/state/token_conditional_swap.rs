use anchor_lang::prelude::*;

use derivative::Derivative;
use fixed::types::I80F48;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::state::*;

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Derivative)]
#[derivative(Debug)]
pub struct TokenConditionalSwap {
    pub id: u64,

    /// maximum amount of native tokens to buy or sell
    pub max_buy: u64,
    pub max_sell: u64,

    /// how many native tokens were already bought/sold
    pub bought: u64,
    pub sold: u64,

    /// timestamp until which the conditional swap is valid
    pub expiry_timestamp: u64,

    /// The price must exceed this threshold to allow execution.
    ///
    /// This threshold is compared to the "sell_token per buy_token" oracle price
    /// (which can be computed by dividing the buy token oracle price by the
    /// sell token oracle price). If that price is >= lower_limit and <= upper_limit
    /// the tcs may be executable.
    ///
    /// Example: Stop loss to get out of a SOL long: The user bought SOL at 20 USDC/SOL
    /// and wants to stop loss at 18 USDC/SOL. They'd set buy_token=USDC, sell_token=SOL
    /// so the reference price is in SOL/USDC units. Set price_lower_limit=toNative(1/18)
    /// and price_upper_limit=toNative(1/10). Also set allow_borrows=false.
    ///
    /// Example: Want to buy SOL with USDC if the price falls below 22 USDC/SOL.
    /// buy_token=SOL, sell_token=USDC, reference price is in USDC/SOL units. Set
    /// price_upper_limit=toNative(22), price_lower_limit=0.
    pub price_lower_limit: f32,

    /// Parallel to price_lower_limit, but an upper limit.
    pub price_upper_limit: f32,

    /// The premium to pay over oracle price to incentivize execution.
    pub price_premium_fraction: f32,

    /// The taker receives only premium_price * (1 - taker_fee_fraction)
    pub taker_fee_fraction: f32,

    /// The maker has to pay premium_price * (1 + maker_fee_fraction)
    pub maker_fee_fraction: f32,

    /// indexes of tokens for the swap
    pub buy_token_index: TokenIndex,
    pub sell_token_index: TokenIndex,

    pub has_data: u8,

    /// may token purchases create deposits? (often users just want to get out of a borrow)
    pub allow_creating_deposits: u8,
    /// may token selling create borrows? (often users just want to get out of a long)
    pub allow_creating_borrows: u8,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 109],
}

const_assert_eq!(
    size_of::<TokenConditionalSwap>(),
    8 * 6 + 4 * 5 + 2 * 2 + 1 * 3 + 109
);
const_assert_eq!(size_of::<TokenConditionalSwap>(), 184);
const_assert_eq!(size_of::<TokenConditionalSwap>() % 8, 0);

impl Default for TokenConditionalSwap {
    fn default() -> Self {
        Self {
            id: 0,
            max_buy: 0,
            max_sell: 0,
            bought: 0,
            sold: 0,
            expiry_timestamp: u64::MAX,
            price_lower_limit: 0.0,
            price_upper_limit: 0.0,
            price_premium_fraction: 0.0,
            taker_fee_fraction: 0.0,
            maker_fee_fraction: 0.0,
            buy_token_index: TokenIndex::MAX,
            sell_token_index: TokenIndex::MAX,
            has_data: 0,
            allow_creating_borrows: 0,
            allow_creating_deposits: 0,
            reserved: [0; 109],
        }
    }
}

impl TokenConditionalSwap {
    /// Whether the entry is in use
    ///
    /// Note that it's possible for an entry to be in use but be expired
    pub fn has_data(&self) -> bool {
        self.has_data == 1
    }

    pub fn set_has_data(&mut self, has_data: bool) {
        self.has_data = u8::from(has_data);
    }

    pub fn is_expired(&self, now_ts: u64) -> bool {
        now_ts >= self.expiry_timestamp
    }

    pub fn allow_creating_deposits(&self) -> bool {
        self.allow_creating_deposits == 1
    }

    pub fn allow_creating_borrows(&self) -> bool {
        self.allow_creating_borrows == 1
    }

    pub fn remaining_buy(&self) -> u64 {
        self.max_buy - self.bought
    }

    pub fn remaining_sell(&self) -> u64 {
        self.max_sell - self.sold
    }

    /// Base price adjusted for the premium
    ///
    /// Base price is the amount of sell_token to pay for one buy_token.
    pub fn premium_price(&self, base_price: f32) -> f32 {
        base_price * (1.0 + self.price_premium_fraction)
    }

    /// Premium price adjusted for the maker fee
    pub fn maker_price(&self, premium_price: f32) -> f32 {
        premium_price * (1.0 + self.maker_fee_fraction)
    }

    /// Premium price adjusted for the taker fee
    pub fn taker_price(&self, premium_price: f32) -> f32 {
        premium_price * (1.0 - self.taker_fee_fraction)
    }

    pub fn maker_fee(&self, base_sell_amount: I80F48) -> u64 {
        (base_sell_amount * I80F48::from_num(self.maker_fee_fraction))
            .floor()
            .to_num()
    }

    pub fn taker_fee(&self, base_sell_amount: I80F48) -> u64 {
        (base_sell_amount * I80F48::from_num(self.taker_fee_fraction))
            .floor()
            .to_num()
    }

    pub fn price_in_range(&self, price: f32) -> bool {
        price >= self.price_lower_limit && price <= self.price_upper_limit
    }
}
