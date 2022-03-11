use anchor_lang::prelude::*;

use crate::state::*;

pub type SerumMarketIndex = u16;

#[account(zero_copy)]
pub struct SerumMarket {
    pub group: Pubkey,
    pub serum_program: Pubkey,
    pub serum_market_external: Pubkey,

    pub market_index: SerumMarketIndex,
    pub base_token_index: TokenIndex,
    pub quote_token_index: TokenIndex,

    pub bump: u8,
}
// TODO: static assert the size and alignment

#[macro_export]
macro_rules! serum_market_seeds {
    ( $acc:expr ) => {
        &[
            $acc.group.as_ref(),
            b"serum".as_ref(),
            $acc.serum_market_external.as_ref(),
            &[$acc.bump],
        ]
    };
}

pub use serum_market_seeds;
