use crate::{
    accounts_ix::FlashLoanType,
    state::{PerpMarket, PerpPosition},
};
use anchor_lang::prelude::*;
use borsh::BorshSerialize;

pub fn emit_perp_balances(
    mango_group: Pubkey,
    mango_account: Pubkey,
    pp: &PerpPosition,
    pm: &PerpMarket,
) {
    emit!(PerpBalanceLog {
        mango_group,
        mango_account,
        market_index: pm.perp_market_index,
        base_position: pp.base_position_lots(),
        quote_position: pp.quote_position_native().to_bits(),
        long_settled_funding: pp.long_settled_funding.to_bits(),
        short_settled_funding: pp.short_settled_funding.to_bits(),
        long_funding: pm.long_funding.to_bits(),
        short_funding: pm.short_funding.to_bits(),
    });
}

#[event]
pub struct PerpBalanceLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub market_index: u16,
    pub base_position: i64,
    pub quote_position: i128,        // I80F48
    pub long_settled_funding: i128,  // I80F48
    pub short_settled_funding: i128, // I80F48
    pub long_funding: i128,          // I80F48
    pub short_funding: i128,         // I80F48
}

#[event]
pub struct TokenBalanceLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_index: u16,       // IDL doesn't support usize
    pub indexed_position: i128, // on client convert i128 to I80F48 easily by passing in the BN to I80F48 ctor
    pub deposit_index: i128,    // I80F48
    pub borrow_index: i128,     // I80F48
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
    pub maker_out: bool, // true if maker order quantity == 0
    pub timestamp: u64,
    pub seq_num: u64, // note: usize same as u64

    pub maker: Pubkey,
    pub maker_order_id: u128,
    pub maker_fee: i128,

    // Timestamp of when the maker order was placed; copied over from the LeafNode
    pub maker_timestamp: u64,

    pub taker: Pubkey,
    pub taker_order_id: u128,
    pub taker_client_order_id: u64,
    pub taker_fee: i128,

    pub price: i64,
    pub quantity: i64, // number of base lots
}

#[event]
pub struct FillLogV2 {
    pub mango_group: Pubkey,
    pub market_index: u16,
    pub taker_side: u8, // side from the taker's POV
    pub maker_slot: u8,
    pub maker_out: bool, // true if maker order quantity == 0
    pub timestamp: u64,
    pub seq_num: u64, // note: usize same as u64

    pub maker: Pubkey,
    pub maker_client_order_id: u64,
    pub maker_fee: f32,

    // Timestamp of when the maker order was placed; copied over from the LeafNode
    pub maker_timestamp: u64,

    pub taker: Pubkey,
    pub taker_client_order_id: u64,
    pub taker_fee: f32,

    pub price: i64,
    pub quantity: i64, // number of base lots
}

#[event]
pub struct PerpUpdateFundingLog {
    pub mango_group: Pubkey,
    pub market_index: u16,
    pub long_funding: i128,
    pub short_funding: i128,
    pub price: i128,
    pub stable_price: i128,
    pub fees_accrued: i128,
    pub open_interest: i64,
    pub instantaneous_funding_rate: i128,
}

#[event]
pub struct UpdateIndexLog {
    pub mango_group: Pubkey,
    pub token_index: u16,
    pub deposit_index: i128,   // I80F48
    pub borrow_index: i128,    // I80F48
    pub avg_utilization: i128, // I80F48
    pub price: i128,           // I80F48
    pub stable_price: i128,    // I80F48
    pub collected_fees: i128,  // I80F48
    pub loan_fee_rate: i128,   // I80F48
    pub total_borrows: i128,
    pub total_deposits: i128,
    pub borrow_rate: i128,
    pub deposit_rate: i128,
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
pub struct TokenLiqWithTokenLog {
    pub mango_group: Pubkey,
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub asset_token_index: u16,
    pub liab_token_index: u16,
    pub asset_transfer: i128, // I80F48
    pub liab_transfer: i128,  // I80F48
    pub asset_price: i128,    // I80F48
    pub liab_price: i128,     // I80F48
    pub bankruptcy: bool,
}

#[event]
pub struct Serum3OpenOrdersBalanceLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub base_token_index: u16,
    pub quote_token_index: u16,
    pub base_total: u64,
    pub base_free: u64,
    pub quote_total: u64,
    pub quote_free: u64,
    pub referrer_rebates_accrued: u64,
}

#[event]
pub struct Serum3OpenOrdersBalanceLogV2 {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub market_index: u16,
    pub base_token_index: u16,
    pub quote_token_index: u16,
    pub base_total: u64,
    pub base_free: u64,
    pub quote_total: u64,
    pub quote_free: u64,
    pub referrer_rebates_accrued: u64,
}

#[derive(PartialEq, Copy, Clone, Debug, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum LoanOriginationFeeInstruction {
    Unknown,
    LiqTokenBankruptcy,
    LiqTokenWithToken,
    Serum3LiqForceCancelOrders,
    Serum3PlaceOrder,
    Serum3SettleFunds,
    TokenWithdraw,
}

#[event]
pub struct WithdrawLoanOriginationFeeLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_index: u16,
    pub loan_origination_fee: i128, // I80F48
    pub instruction: LoanOriginationFeeInstruction,
}

#[event]
pub struct TokenLiqBankruptcyLog {
    pub mango_group: Pubkey,
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub liab_token_index: u16,
    pub initial_liab_native: i128,
    pub liab_price: i128,
    pub insurance_token_index: u16,
    pub insurance_transfer: i128,
    pub socialized_loss: i128,
    pub starting_liab_deposit_index: i128,
    pub ending_liab_deposit_index: i128,
}

#[event]
pub struct DeactivateTokenPositionLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_index: u16,
    pub cumulative_deposit_interest: f64,
    pub cumulative_borrow_interest: f64,
}

#[event]
pub struct DeactivatePerpPositionLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub market_index: u16,
    pub cumulative_long_funding: f64,
    pub cumulative_short_funding: f64,
    pub maker_volume: u64,
    pub taker_volume: u64,
    pub perp_spot_transfers: i64,
}

#[event]
pub struct TokenMetaDataLog {
    pub mango_group: Pubkey,
    pub mint: Pubkey,
    pub token_index: u16,
    pub mint_decimals: u8,
    pub oracle: Pubkey,
    pub mint_info: Pubkey,
}

#[event]
pub struct PerpMarketMetaDataLog {
    pub mango_group: Pubkey,
    pub perp_market: Pubkey,
    pub perp_market_index: u16,
    pub base_decimals: u8,
    pub base_lot_size: i64,
    pub quote_lot_size: i64,
    pub oracle: Pubkey,
}

#[event]
pub struct Serum3RegisterMarketLog {
    pub mango_group: Pubkey,
    pub serum_market: Pubkey,
    pub market_index: u16,
    pub base_token_index: u16,
    pub quote_token_index: u16,
    pub serum_program: Pubkey,
    pub serum_program_external: Pubkey,
}

#[event]
pub struct PerpLiqBaseOrPositivePnlLog {
    pub mango_group: Pubkey,
    pub perp_market_index: u16,
    pub liqor: Pubkey,
    pub liqee: Pubkey,
    pub base_transfer: i64,
    pub quote_transfer: i128,
    pub pnl_transfer: i128,
    pub pnl_settle_limit_transfer: i128,
    pub price: i128,
}

#[event]
pub struct PerpLiqBankruptcyLog {
    pub mango_group: Pubkey,
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub perp_market_index: u16,
    pub insurance_transfer: i128,
    pub socialized_loss: i128,
    pub starting_long_funding: i128,
    pub starting_short_funding: i128,
    pub ending_long_funding: i128,
    pub ending_short_funding: i128,
}

#[event]
pub struct PerpLiqNegativePnlOrBankruptcyLog {
    pub mango_group: Pubkey,
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub perp_market_index: u16,
    pub settlement: i128,
}

#[event]
pub struct PerpSettlePnlLog {
    pub mango_group: Pubkey,
    pub mango_account_a: Pubkey,
    pub mango_account_b: Pubkey,
    pub perp_market_index: u16,
    pub settlement: i128,
    pub settler: Pubkey,
    pub fee: i128,
}

#[event]
pub struct PerpSettleFeesLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub perp_market_index: u16,
    pub settlement: i128,
}

#[event]
pub struct AccountBuybackFeesWithMngoLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub buyback_fees: i128,
    pub buyback_mngo: i128,
    pub mngo_buyback_price: i128,
    pub oracle_price: i128,
}
