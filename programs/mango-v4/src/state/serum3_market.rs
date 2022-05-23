use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::state::*;

pub type Serum3MarketIndex = u16;

#[account(zero_copy)]
#[derive(Debug)]
pub struct Serum3Market {
    pub name: [u8; 16],
    pub group: Pubkey,
    pub serum_program: Pubkey,
    pub serum_market_external: Pubkey,

    pub market_index: Serum3MarketIndex,
    pub base_token_index: TokenIndex,
    pub quote_token_index: TokenIndex,

    pub bump: u8,
    pub reserved: [u8; 1],
}
const_assert_eq!(size_of::<Serum3Market>(), 16 + 32 * 3 + 3 * 2 + 1 + 1);
const_assert_eq!(size_of::<Serum3Market>() % 8, 0);

impl Serum3Market {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }
}

#[macro_export]
macro_rules! serum_market_seeds {
    ( $acc:expr ) => {
        &[
            $acc.group.as_ref(),
            b"Serum3Market".as_ref(),
            $acc.serum_market_external.as_ref(),
            &[$acc.bump],
        ]
    };
}

pub use serum_market_seeds;
