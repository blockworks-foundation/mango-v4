use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::state::*;

pub type OpenbookV2MarketIndex = u16;

#[account(zero_copy)]
#[derive(Debug)]
pub struct OpenbookV2Market {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,
    // ABI: Clients rely on this being at offset 40
    pub base_token_index: TokenIndex,
    // ABI: Clients rely on this being at offset 42
    pub quote_token_index: TokenIndex,
    pub market_index: OpenbookV2MarketIndex,
    pub reduce_only: u8,
    pub force_close: u8,
    pub name: [u8; 16],
    pub openbook_v2_program: Pubkey,
    pub openbook_v2_market_external: Pubkey,

    pub registration_time: u64,

    /// Limit orders must be <= oracle * (1+band) and >= oracle / (1+band)
    ///
    /// Zero value is the default due to migration and disables the limit,
    /// same as f32::MAX.
    pub oracle_price_band: f32,

    pub bump: u8,

    pub reserved: [u8; 1027],
}
const_assert_eq!(
    size_of::<OpenbookV2Market>(),
    32 + 2 * 3 + 1 * 2 + 1 * 16 + 32 * 2 + 8 + 4 + 1 + 1027
);
const_assert_eq!(size_of::<OpenbookV2Market>(), 1160);
const_assert_eq!(size_of::<OpenbookV2Market>() % 8, 0);

impl OpenbookV2Market {
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

    pub fn oracle_price_band(&self) -> f32 {
        if self.oracle_price_band == 0.0 {
            f32::MAX // default disabled
        } else {
            self.oracle_price_band
        }
    }
}

#[account(zero_copy)]
#[derive(Debug)]
pub struct OpenbookV2MarketIndexReservation {
    pub group: Pubkey,
    pub market_index: OpenbookV2MarketIndex,
    pub reserved: [u8; 38],
}
const_assert_eq!(size_of::<OpenbookV2MarketIndexReservation>(), 32 + 2 + 38);
const_assert_eq!(size_of::<OpenbookV2MarketIndexReservation>(), 72);
const_assert_eq!(size_of::<OpenbookV2MarketIndexReservation>() % 8, 0);

#[macro_export]
macro_rules! openbook_v2_market_seeds {
    ( $acc:expr ) => {
        &[
            b"OpenbookV2Market".as_ref(),
            $acc.group.as_ref(),
            $acc.openbook_v2_market_external.as_ref(),
            &[$acc.bump],
        ]
    };
}

pub use openbook_v2_market_seeds;
