use fixed::types::I80F48;

// todo: I remember hearing something around we wanting to use the oracle account directly in program?
// but then if the confidence is bad, how would we refer to last known confident value?
pub struct PriceCache {
    pub price: I80F48, // unit is interpreted as how many quote native tokens for 1 base native token
    pub last_update: u64,
}

// todo: can we use rootbank directly?
pub struct RootBankCache {
    pub deposit_index: I80F48,
    pub borrow_index: I80F48,
    pub last_update: u64,
}

// todo: can we just use the perpmarket directly?
pub struct PerpMarketCache {
    pub long_funding: I80F48,
    pub short_funding: I80F48,
    pub last_update: u64,
}

pub struct MangoCache {
    pub meta_data: MetaData,

    pub price_cache: [PriceCache; MAX_PAIRS],
    pub root_bank_cache: [RootBankCache; MAX_TOKENS],
    pub perp_market_cache: [PerpMarketCache; MAX_PAIRS],
}
