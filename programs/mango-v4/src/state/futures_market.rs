use crate::error::MangoError;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;

use anchor_lang::prelude::*;

#[account(zero_copy)]
pub struct FuturesMarket {
    pub mango_group: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_queue: Pubkey,

    pub mark_price: Pubkey,

    // TODO: Maybe doesn't need to be stored here, can just be stored on ids.json or MangoGroup
    pub quote_lot_size: i64, // number of quote native that reresents min tick
    pub base_lot_size: i64,  // represents number of base native quantity; greater than 0

    pub open_interest: i64, // This is i64 to keep consistent with the units of contracts, but should always be > 0

    pub seq_num: u64,
    pub fees_accrued: I80F48, // native quote currency
}

/// Stores a moving average of the basis to be used to calculate the mark price
/// This is kept updated by the Keeper
#[account(zero_copy)]
pub struct FuturesMarkPrice {
    pub futures_market: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,

    /// The ema update parameter. Typically 2 / (n + 1) where n is number of periods
    pub alpha: I80F48,

    /// Exponential moving average of the basis;
    /// UNIT: number
    pub basis_ema: I80F48,

    /// timestamp of last update
    pub last_update: u64,

    pub update_min_interval: u64,

    /// The mark price as of the last update.
    /// index * (1 + ema(basis))
    /// UNIT: native quote units per one native base
    pub price: I80F48,
}

impl FuturesMarkPrice {
    /// Update the basis ema and store
    /// TODO: `bid` and `ask` are temp until the order book is built.
    pub fn update_mark_price(
        &mut self,
        index: I80F48,
        bid: I80F48,
        ask: I80F48,
        now_ts: u64,
    ) -> Result<()> {
        require!(
            now_ts >= self.last_update + self.update_min_interval,
            MangoError::UpdateTooSoon
        )?;

        // TODO make alpha a function of time between last update
        let mid = (bid + ask) / I80F48!(2);
        let basis = mid / index - I80F48::ONE;
        self.basis_ema = self.alpha * basis + (I80F48::ONE - self.alpha) * self.basis_ema;
        self.price = index * (I80F48::ONE + self.basis_ema);
        self.last_update = now_ts;
        Ok(())
    }

    /// Return the current mark price
    pub fn get(&self) -> I80F48 {
        if self.price.is_zero() {
            panic!();
        }
        self.price
    }
}
