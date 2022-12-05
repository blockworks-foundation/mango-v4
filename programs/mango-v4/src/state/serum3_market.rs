use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::state::*;

pub type Serum3MarketIndex = u16;

#[account(zero_copy)]
#[derive(Debug)]
pub struct Serum3Market {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,
    // ABI: Clients rely on this being at offset 40
    pub base_token_index: TokenIndex,
    // ABI: Clients rely on this being at offset 42
    pub quote_token_index: TokenIndex,
    pub padding1: [u8; 4],
    pub name: [u8; 16],
    pub serum_program: Pubkey,
    pub serum_market_external: Pubkey,

    pub market_index: Serum3MarketIndex,

    pub bump: u8,

    pub padding2: [u8; 5],

    pub registration_time: u64,

    pub reserved: [u8; 128],
}
const_assert_eq!(
    size_of::<Serum3Market>(),
    32 + 2 + 2 + 4 + 16 + 2 * 32 + 2 + 1 + 5 + 8 + 128
);
const_assert_eq!(size_of::<Serum3Market>(), 264);
const_assert_eq!(size_of::<Serum3Market>() % 8, 0);

impl Serum3Market {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }
}

#[account(zero_copy)]
#[derive(Debug)]
pub struct Serum3MarketIndexReservation {
    pub group: Pubkey,
    pub market_index: Serum3MarketIndex,
    pub reserved: [u8; 38],
}
const_assert_eq!(size_of::<Serum3MarketIndexReservation>(), 32 + 2 + 38);
const_assert_eq!(size_of::<Serum3MarketIndexReservation>(), 72);
const_assert_eq!(size_of::<Serum3MarketIndexReservation>() % 8, 0);

#[macro_export]
macro_rules! serum_market_seeds {
    ( $acc:expr ) => {
        &[
            b"Serum3Market".as_ref(),
            $acc.group.as_ref(),
            $acc.serum_market_external.as_ref(),
            &[$acc.bump],
        ]
    };
}

pub use serum_market_seeds;
