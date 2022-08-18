use crate::{
    instructions::FlashLoanType,
    state::{PerpMarket, PerpPositions},
};
use anchor_lang::prelude::*;
use borsh::BorshSerialize;

/// Warning: This function needs 512+ bytes free on the stack
pub fn emit_perp_balances(
    mango_group: Pubkey,
    mango_account: Pubkey,
    market_index: u64,
    price: i64,
    pp: &PerpPositions,
    pm: &PerpMarket,
) {
    emit!(PerpBalanceLog {
        mango_group,
        mango_account,
        market_index,
        base_position: pp.base_position_lots,
        quote_position: pp.quote_position_native.to_bits(),
        long_settled_funding: pp.long_settled_funding.to_bits(),
        short_settled_funding: pp.short_settled_funding.to_bits(),
        price,
        long_funding: pm.long_funding.to_bits(),
        short_funding: pm.short_funding.to_bits(),
    });
}

#[event]
pub struct PerpBalanceLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub market_index: u64, // IDL doesn't support usize
    pub base_position: i64,
    pub quote_position: i128,        // I80F48
    pub long_settled_funding: i128,  // I80F48
    pub short_settled_funding: i128, // I80F48
    pub price: i64,
    pub long_funding: i128,  // I80F48
    pub short_funding: i128, // I80F48
}

#[event]
pub struct TokenBalanceLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_index: u16,       // IDL doesn't support usize
    pub indexed_position: i128, // on client convert i128 to I80F48 easily by passing in the BN to I80F48 ctor
    pub deposit_index: i128,    // I80F48
    pub borrow_index: i128,     // I80F48
    pub price: i128,            // I80F48
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct FlashLoanTokenDetail {
    pub token_index: u16,
    pub change_amount: i128,
    pub loan: i128,
    pub loan_origination_fee: i128,
    pub deposit_index: i128,
    pub borrow_index: i128,
    pub price: i128,
}

#[event]
pub struct FlashLoanLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_loan_details: Vec<FlashLoanTokenDetail>,
    pub flash_loan_type: FlashLoanType,
}

#[event]
pub struct WithdrawLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub signer: Pubkey,
    pub token_index: u16,
    pub quantity: u64,
    pub price: i128, // I80F48
}

#[event]
pub struct DepositLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub signer: Pubkey,
    pub token_index: u16,
    pub quantity: u64,
    pub price: i128, // I80F48
}

#[event]
pub struct FillLog {
    pub mango_group: Pubkey,
    pub market_index: u16,
    pub taker_side: u8, // side from the taker's POV
    pub maker_slot: u8,
    pub market_fees_applied: bool,
    pub maker_out: bool, // true if maker order quantity == 0
    pub timestamp: u64,
    pub seq_num: u64, // note: usize same as u64

    pub maker: Pubkey,
    pub maker_order_id: i128,
    pub maker_client_order_id: u64,
    pub maker_fee: i128,

    // Timestamp of when the maker order was placed; copied over from the LeafNode
    pub maker_timestamp: u64,

    pub taker: Pubkey,
    pub taker_order_id: i128,
    pub taker_client_order_id: u64,
    pub taker_fee: i128,

    pub price: i64,
    pub quantity: i64, // number of base lots
}

#[event]
pub struct UpdateFundingLog {
    pub mango_group: Pubkey,
    pub market_index: u16,
    pub long_funding: i128,  // I80F48
    pub short_funding: i128, // I80F48
    pub price: i128,         // I80F48
}

#[event]
pub struct UpdateIndexLog {
    pub mango_group: Pubkey,
    pub token_index: u16,
    pub deposit_index: i128,   // I80F48
    pub borrow_index: i128,    // I80F48
    pub avg_utilization: i128, // I80F48
    pub price: i128,           // I80F48
    pub collected_fees: i128,  // I80F48
    pub loan_fee_rate: i128,   // I80F48
}

#[event]
pub struct UpdateRateLog {
    pub mango_group: Pubkey,
    pub token_index: u16,
    pub rate0: i128,    // I80F48
    pub rate1: i128,    // I80F48
    pub max_rate: i128, // I80F48
}

#[event]
pub struct LiquidateTokenAndTokenLog {
    pub mango_group: Pubkey,
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub asset_token_index: u16,
    pub liab_token_index: u16,
    pub asset_transfer: i128, // I80F48
    pub liab_transfer: i128,  // I80F48
    pub asset_price: i128,    // I80F48
    pub liab_price: i128,     // I80F48
                              // pub bankruptcy: bool,
}

#[event]
pub struct OpenOrdersBalanceLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub market_index: u16,
    pub base_total: u64,
    pub base_free: u64,
    /// this field does not include the referrer_rebates; need to add that in to get true total
    pub quote_total: u64,
    pub quote_free: u64,
    pub referrer_rebates_accrued: u64,
    pub price: i128, // I80F48
}

#[event]
pub struct WithdrawLoanOriginationFeeLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_index: u16,
    pub loan_origination_fee: i128, // I80F48
}
