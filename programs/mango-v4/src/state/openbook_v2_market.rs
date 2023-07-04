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
    pub reduce_only: u8,
    pub force_close: u8,
    pub padding1: [u8; 2],
    pub name: [u8; 16],
    pub openbook_v2_program: Pubkey,
    pub openbook_v2_market_external: Pubkey,

    pub market_index: OpenbookV2MarketIndex,

    pub bump: u8,

    pub padding2: [u8; 5],

    pub registration_time: u64,

    pub reserved: [u8; 512],
}
const_assert_eq!(
    size_of::<OpenbookV2Market>(),
    32 + 2 + 2 + 1 + 3 + 16 + 2 * 32 + 2 + 1 + 5 + 8 + 512
);
const_assert_eq!(size_of::<OpenbookV2Market>(), 648);
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
