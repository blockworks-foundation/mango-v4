use anchor_lang::prelude::*;
use fixed::types::I80F48;

pub struct SpotMarketInfo {
    pub spot_market: Pubkey,
    pub maint_asset_weight: I80F48,
    pub init_asset_weight: I80F48,
    pub maint_liab_weight: I80F48,
    pub init_liab_weight: I80F48,
    pub liquidation_fee: I80F48,
}
