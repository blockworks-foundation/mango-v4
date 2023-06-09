use anchor_lang::prelude::*;

use derivative::Derivative;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::state::*;

#[derive(
    Eq,
    PartialEq,
    Copy,
    Clone,
    TryFromPrimitive,
    IntoPrimitive,
    Debug,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[repr(u8)]
pub enum TokenStopLossPriceThresholdType {
    PriceOverThreshold,
    PriceUnderThreshold,
}

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Derivative, bytemuck::Pod)]
#[derivative(Debug)]
pub struct TokenStopLoss {
    /// maximum amount of native tokens to buy or sell
    pub max_buy: u64,
    pub max_sell: u64,

    /// how many native tokens were already bought/sold
    pub bought: u64,
    pub sold: u64,

    /// the threshold at which to allow execution
    pub price_threshold: f32,

    /// the premium to pay over oracle price
    pub price_premium_bps: u32,

    /// indexes of tokens for the swap
    pub buy_token_index: TokenIndex,
    pub sell_token_index: TokenIndex,

    /// holds a TokenStopLossPriceThresholdType, so whether the threshold is > or <
    pub price_threshold_type: u8,

    pub is_active: u8,

    /// may token purchases create deposits? (often users just want to get out of a borrow)
    pub allow_creating_deposits: u8,
    /// may token selling create borrows? (often users just want to get out of a long)
    pub allow_creating_borrows: u8,

    // TODO: these should probably have some kind of id, so it's easy to refer to them
    //       and clearer when one got cancelled and replaced
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 136],
}

const_assert_eq!(
    size_of::<TokenStopLoss>(),
    8 * 4 + 4 + 4 + 2 * 2 + 1 * 4 + 136
);
const_assert_eq!(size_of::<TokenStopLoss>(), 184);
const_assert_eq!(size_of::<TokenStopLoss>() % 8, 0);

impl Default for TokenStopLoss {
    fn default() -> Self {
        Self {
            max_buy: 0,
            max_sell: 0,
            bought: 0,
            sold: 0,
            price_threshold: 0.0,
            price_premium_bps: 0,
            buy_token_index: TokenIndex::MAX,
            sell_token_index: TokenIndex::MAX,
            price_threshold_type: TokenStopLossPriceThresholdType::PriceOverThreshold.into(),
            is_active: 0,
            allow_creating_borrows: 0,
            allow_creating_deposits: 0,
            reserved: [0; 136],
        }
    }
}

impl TokenStopLoss {
    pub fn is_active(&self) -> bool {
        self.is_active == 1
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = u8::from(active);
    }

    pub fn allow_creating_deposits(&self) -> bool {
        self.allow_creating_deposits == 1
    }

    pub fn allow_creating_borrows(&self) -> bool {
        self.allow_creating_borrows == 1
    }

    pub fn price_threshold_type(&self) -> TokenStopLossPriceThresholdType {
        TokenStopLossPriceThresholdType::try_from(self.price_threshold_type).unwrap()
    }

    pub fn remaining_buy(&self) -> u64 {
        self.max_buy - self.bought
    }

    pub fn remaining_sell(&self) -> u64 {
        self.max_sell - self.sold
    }
}
