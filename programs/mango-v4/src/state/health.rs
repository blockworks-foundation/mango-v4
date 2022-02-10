pub struct UserActiveAssets {
    pub spot: [bool; MAX_PAIRS],
    pub perps: [bool; MAX_PAIRS],
}

pub struct HealthCache {
    pub active_assets: UserActiveAssets,

    /// Vec of length MAX_PAIRS containing worst case spot vals; unweighted
    spot: Vec<(I80F48, I80F48)>,
    perp: Vec<(I80F48, I80F48)>,
    quote: I80F48,

    /// This will be zero until update_health is called for the first time
    health: [Option<I80F48>; 2],
}
