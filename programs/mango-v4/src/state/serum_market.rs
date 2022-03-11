use anchor_lang::prelude::*;

use crate::state::*;

type SerumMarketIndex = u16;

#[account(zero_copy)]
pub struct SerumMarket {
    pub group: Pubkey,
    pub serum_program: Pubkey,
    pub serum_market_external: Pubkey,

    pub index: SerumMarketIndex,
    pub base_token_index: TokenIndex,
    pub quote_token_index: TokenIndex,
}
// TODO: static assert the size and alignment
