use anchor_lang::prelude::*;

use derivative::Derivative;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::state::*;

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Derivative, bytemuck::Pod)]
#[derivative(Debug)]
pub struct TokenConditionalSwap {
    pub id: u64,

    /// maximum amount of native tokens to buy or sell
    pub max_buy: u64,
    pub max_sell: u64,

    /// how many native tokens were already bought/sold
    pub bought: u64,
    pub sold: u64,

    /// the threshold at which to allow execution
    pub price_threshold: f32,

    /// the maximum price at which execution is allowed
    pub price_limit: f32,

    /// the premium to pay over oracle price
    pub price_premium_bps: u32,

    /// indexes of tokens for the swap
    pub buy_token_index: TokenIndex,
    pub sell_token_index: TokenIndex,

    pub is_active: u8,

    /// may token purchases create deposits? (often users just want to get out of a borrow)
    pub allow_creating_deposits: u8,
    /// may token selling create borrows? (often users just want to get out of a long)
    pub allow_creating_borrows: u8,

    // TODO: Add some kind of expiry timestamp
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 125],
}

const_assert_eq!(
    size_of::<TokenConditionalSwap>(),
    8 * 5 + 2 * 4 + 4 + 2 * 2 + 1 * 3 + 125
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
            price_threshold: 0.0,
            price_limit: 0.0,
            price_premium_bps: 0,
            buy_token_index: TokenIndex::MAX,
            sell_token_index: TokenIndex::MAX,
            is_active: 0,
            allow_creating_borrows: 0,
            allow_creating_deposits: 0,
            reserved: [0; 125],
        }
    }
}

impl TokenConditionalSwap {
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

    pub fn remaining_buy(&self) -> u64 {
        self.max_buy - self.bought
    }

    pub fn remaining_sell(&self) -> u64 {
        self.max_sell - self.sold
    }

    pub fn execution_price(&self, base_price: f32) -> f32 {
        base_price * (1.0 + (self.price_premium_bps as f32) * 0.0001)
    }
}
