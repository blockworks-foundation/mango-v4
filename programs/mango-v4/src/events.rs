use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::state::{PerpMarketIndex, TokenIndex};

#[event]
pub struct MangoAccountData {
    pub init_health: I80F48,
    pub maint_health: I80F48,
    pub equity: Equity,
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct Equity {
    pub tokens: Vec<TokenEquity>,
    pub perps: Vec<PerpEquity>,
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct TokenEquity {
    pub token_index: TokenIndex,
    pub value: I80F48, // in native quote
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct PerpEquity {
    pub perp_market_index: PerpMarketIndex,
    value: I80F48, // in native quote
}
