use anchor_lang::prelude::*;
use fixed::types::I80F48;
use mango_macro::Pod;

#[derive(Copy, Clone, Pod)]
#[repr(C)]
pub struct PerpMarketInfo {
    pub perp_market: Pubkey, // One of these may be empty
    pub maint_asset_weight: I80F48,
    pub init_asset_weight: I80F48,
    pub maint_liab_weight: I80F48,
    pub init_liab_weight: I80F48,
    pub liquidation_fee: I80F48,
    pub maker_fee: I80F48,
    pub taker_fee: I80F48,
    pub base_lot_size: i64,  // The lot size of the underlying
    pub quote_lot_size: i64, // min tick
}
