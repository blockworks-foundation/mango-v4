use anchor_lang::prelude::*;

use derivative::Derivative;
use fixed::types::I80F48;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::error::MangoError;
use crate::i80f48::ClampToInt;
use crate::state::*;

/// Incentive to pay to callers who start an auction, in $1e-6
pub const TCS_START_INCENTIVE: u64 = 1_000; // $0.001 around 10x tx fee right now

#[derive(
    Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, AnchorDeserialize, AnchorSerialize,
)]
#[repr(u8)]
pub enum TokenConditionalSwapDisplayPriceStyle {
    SellTokenPerBuyToken,
    BuyTokenPerSellToken,
}

#[derive(
    Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, AnchorDeserialize, AnchorSerialize,
)]
#[repr(u8)]
pub enum TokenConditionalSwapIntention {
    Unknown,
    /// Reducing a position when the price gets worse
    StopLoss,
    /// Reducing a position when the price gets better
    TakeProfit,
}

#[derive(
    Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, AnchorDeserialize, AnchorSerialize,
)]
#[repr(u8)]
pub enum TokenConditionalSwapType {
    FixedPremium,
    PremiumAuction,
    LinearAuction,
}

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Derivative, PartialEq)]
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

    /// The lower or starting price:
    /// - For FixedPremium or PremiumAuctions, it's the lower end of the price range:
    ///   the tcs can only be triggered if the oracle price exceeds this value.
    /// - For LinearAuctions it's the starting price that's offered at start_timestamp.
    ///
    /// The price is always in "sell_token per buy_token" units, which can be computed
    /// by dividing the buy token price by the sell token price.
    ///
    /// For FixedPremium or PremiumAuctions:
    ///
    /// The price must exceed this threshold to allow execution.
    ///
    /// This threshold is compared to the "sell_token per buy_token" oracle price.
    /// If that price is >= lower_limit and <= upper_limit the tcs may be executable.
    ///
    /// Example: Stop loss to get out of a SOL long: The user bought SOL at 20 USDC/SOL
    /// and wants to stop loss at 18 USDC/SOL. They'd set buy_token=USDC, sell_token=SOL
    /// so the reference price is in SOL/USDC units. Set price_lower_limit=toNative(1/18)
    /// and price_upper_limit=toNative(1/10). Also set allow_borrows=false.
    ///
    /// Example: Want to buy SOL with USDC if the price falls below 22 USDC/SOL.
    /// buy_token=SOL, sell_token=USDC, reference price is in USDC/SOL units. Set
    /// price_upper_limit=toNative(22), price_lower_limit=0.
    pub price_lower_limit: f64,

    /// Parallel to price_lower_limit, but an upper limit / auction end price.
    pub price_upper_limit: f64,

    /// The premium to pay over oracle price to incentivize execution.
    pub price_premium_rate: f64,

    /// The taker receives only premium_price * (1 - taker_fee_rate)
    pub taker_fee_rate: f32,

    /// The maker has to pay premium_price * (1 + maker_fee_rate)
    pub maker_fee_rate: f32,

    /// indexes of tokens for the swap
    pub buy_token_index: TokenIndex,
    pub sell_token_index: TokenIndex,

    /// If this struct is in use. (tcs are stored in a static-length array)
    pub is_configured: u8,

    /// may token purchases create deposits? (often users just want to get out of a borrow)
    pub allow_creating_deposits: u8,
    /// may token selling create borrows? (often users just want to get out of a long)
    pub allow_creating_borrows: u8,

    /// The stored prices are always "sell token per buy token", but if the user
    /// used "buy token per sell token" when creating the tcs order, we should continue
    /// to show them prices in that way.
    ///
    /// Stores a TokenConditionalSwapDisplayPriceStyle enum value
    pub display_price_style: u8,

    /// The intention the user had when placing this order, display-only
    ///
    /// Stores a TokenConditionalSwapIntention enum value
    pub intention: u8,

    /// Stores a TokenConditionalSwapType enum value
    pub tcs_type: u8,

    pub padding: [u8; 6],

    /// In seconds since epoch. 0 means not-started.
    ///
    /// FixedPremium: Time of first trigger call. No other effect.
    /// PremiumAuction: Time of start or first trigger call. Can continue to trigger once started.
    /// LinearAuction: Set during creation, auction starts with price_lower_limit at this timestamp.
    pub start_timestamp: u64,

    /// Duration of the auction mechanism
    ///
    /// FixedPremium: ignored
    /// PremiumAuction: time after start that the premium needs to scale to price_premium_rate
    /// LinearAuction: time after start to go from price_lower_limit to price_upper_limit
    pub duration_seconds: u64,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 88],
}

const_assert_eq!(
    size_of::<TokenConditionalSwap>(),
    8 * 6 + 8 * 3 + 2 * 4 + 2 * 2 + 1 * 6 + 6 + 2 * 8 + 88
);
const_assert_eq!(size_of::<TokenConditionalSwap>(), 200);
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
            price_premium_rate: 0.0,
            taker_fee_rate: 0.0,
            maker_fee_rate: 0.0,
            buy_token_index: TokenIndex::MAX,
            sell_token_index: TokenIndex::MAX,
            is_configured: 0,
            allow_creating_borrows: 0,
            allow_creating_deposits: 0,
            display_price_style: TokenConditionalSwapDisplayPriceStyle::SellTokenPerBuyToken.into(),
            intention: TokenConditionalSwapIntention::Unknown.into(),
            tcs_type: TokenConditionalSwapType::FixedPremium.into(),
            padding: Default::default(),
            start_timestamp: 0,
            duration_seconds: 0,
            reserved: [0; 88],
        }
    }
}

impl TokenConditionalSwap {
    /// Whether the entry is in use
    ///
    /// Note that it's possible for an entry to be configured but expired.
    /// Or to be configured but not started yet.
    pub fn is_configured(&self) -> bool {
        self.is_configured == 1
    }

    pub fn set_is_configured(&mut self, is_configured: bool) {
        self.is_configured = u8::from(is_configured);
    }

    pub fn tcs_type(&self) -> TokenConditionalSwapType {
        self.tcs_type.try_into().unwrap()
    }

    pub fn is_expired(&self, now_ts: u64) -> bool {
        now_ts >= self.expiry_timestamp
    }

    pub fn passed_start(&self, now_ts: u64) -> bool {
        self.start_timestamp > 0 && now_ts >= self.start_timestamp
    }

    /// Does this tcs type support an explicit tcs_start instruction call?
    pub fn is_startable_type(&self) -> bool {
        self.tcs_type() == TokenConditionalSwapType::PremiumAuction
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

    fn start_timestamp_or_now(&self, now_ts: u64) -> u64 {
        if self.start_timestamp > 0 {
            self.start_timestamp
        } else {
            now_ts
        }
    }

    /// Base price adjusted for the premium
    ///
    /// Base price is the amount of sell_token to pay for one buy_token.
    pub fn premium_price(&self, base_price: f64, now_ts: u64) -> f64 {
        match self.tcs_type() {
            TokenConditionalSwapType::FixedPremium => base_price * (1.0 + self.price_premium_rate),
            TokenConditionalSwapType::PremiumAuction => {
                // Start dynamically when triggerable
                let start = self.start_timestamp_or_now(now_ts);
                assert!(start <= now_ts);

                let duration = self.duration_seconds as f64;
                let current = (now_ts - start) as f64;
                let progress = (current / duration).min(1.0);
                base_price * (1.0 + progress * self.price_premium_rate)
            }
            TokenConditionalSwapType::LinearAuction => {
                // Start time is fixed
                assert!(self.passed_start(now_ts));

                let duration = self.duration_seconds;
                let current = now_ts - self.start_timestamp;
                if current < duration {
                    let progress = (current as f64) / (duration as f64);
                    self.price_lower_limit
                        + progress * (self.price_upper_limit - self.price_lower_limit)
                } else {
                    // explicitly handle the end to avoid rounding issues
                    self.price_upper_limit
                }
            }
        }
    }

    /// Premium price adjusted for the maker fee
    pub fn maker_price(&self, premium_price: f64) -> f64 {
        premium_price * (1.0 + self.maker_fee_rate as f64)
    }

    /// Premium price adjusted for the taker fee
    pub fn taker_price(&self, premium_price: f64) -> f64 {
        premium_price * (1.0 - self.taker_fee_rate as f64)
    }

    pub fn maker_fee(&self, base_sell_amount: I80F48) -> u64 {
        (base_sell_amount * I80F48::from_num(self.maker_fee_rate))
            .floor()
            .to_num()
    }

    pub fn taker_fee(&self, base_sell_amount: I80F48) -> u64 {
        (base_sell_amount * I80F48::from_num(self.taker_fee_rate))
            .floor()
            .to_num()
    }

    fn price_in_range(&self, price: f64) -> bool {
        price >= self.price_lower_limit && price <= self.price_upper_limit
    }

    /// Do the current conditions and tcs type allow starting?
    pub fn check_startable(&self, price: f64, now_ts: u64) -> Result<()> {
        require!(
            !self.is_expired(now_ts),
            MangoError::TokenConditionalSwapExpired
        );
        require!(
            self.start_timestamp == 0,
            MangoError::TokenConditionalSwapAlreadyStarted
        );
        require!(
            self.is_startable_type(),
            MangoError::TokenConditionalSwapTypeNotStartable
        );
        require!(
            self.price_in_range(price),
            MangoError::TokenConditionalSwapPriceNotInRange
        );
        Ok(())
    }

    pub fn is_startable(&self, price: f64, now_ts: u64) -> bool {
        self.check_startable(price, now_ts).is_ok()
    }

    pub fn check_triggerable(&self, price: f64, now_ts: u64) -> Result<()> {
        require!(
            !self.is_expired(now_ts),
            MangoError::TokenConditionalSwapExpired
        );
        match self.tcs_type() {
            TokenConditionalSwapType::FixedPremium => {
                require!(
                    self.price_in_range(price),
                    MangoError::TokenConditionalSwapPriceNotInRange
                );
            }
            TokenConditionalSwapType::PremiumAuction | TokenConditionalSwapType::LinearAuction => {
                // Triggerable once started, whatever the current oracle price
                require!(
                    self.passed_start(now_ts),
                    MangoError::TokenConditionalSwapNotStarted
                );
            }
        }
        Ok(())
    }

    pub fn is_triggerable(&self, price: f64, now_ts: u64) -> bool {
        self.check_triggerable(price, now_ts).is_ok()
    }

    /// The remaining buy amount, taking the current buy token position and
    /// buy bank's reduce-only status into account.
    ///
    /// Note that the account health might further restrict execution.
    pub fn max_buy_for_position(&self, buy_position: I80F48, buy_bank: &Bank) -> u64 {
        self.remaining_buy().min(
            if self.allow_creating_deposits() && !buy_bank.are_deposits_reduce_only() {
                u64::MAX
            } else {
                // ceil() because we're ok reaching 0..1 deposited native tokens
                (-buy_position).ceil().clamp_to_u64()
            },
        )
    }

    /// The remaining sell amount, taking the current sell token position and
    /// sell bank's reduce-only status into account.
    ///
    /// Note that the account health might further restrict execution.
    pub fn max_sell_for_position(&self, sell_position: I80F48, sell_bank: &Bank) -> u64 {
        self.remaining_sell().min(
            if self.allow_creating_borrows() && !sell_bank.are_borrows_reduce_only() {
                u64::MAX
            } else {
                // floor() so we never go below 0
                sell_position.floor().clamp_to_u64()
            },
        )
    }
}
