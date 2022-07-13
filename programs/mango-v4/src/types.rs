use anchor_lang::prelude::*;

/// Nothing in Rust shall use these types. They only exist so that the Anchor IDL
/// knows about them and typescript can deserialize it.

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct TokenIndex {
    val: u16,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct Serum3MarketIndex {
    val: u16,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct PerpMarketIndex {
    val: u16,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct I80F48 {
    val: i128,
}
