use crate::{
    accounts_ix::FlashLoanType,
    state::{OracleType, PerpMarket, PerpPosition},
};
use anchor_lang::prelude::*;
use borsh::BorshSerialize;

#[inline(never)] // ensure fresh stack frame
pub fn emit_stack<T: anchor_lang::Event>(e: T) {
    use std::io::{Cursor, Write};

    // stack buffer, stack frames are 4kb
    let mut buffer = [0u8; 3000];

    let mut cursor = Cursor::new(&mut buffer[..]);
    cursor.write_all(&T::DISCRIMINATOR).unwrap();
    e.serialize(&mut cursor)
        .expect("event must fit into stack buffer");

    let pos = cursor.position() as usize;
    anchor_lang::solana_program::log::sol_log_data(&[&buffer[..pos]]);
}

pub fn emit_perp_balances(
    mango_group: Pubkey,
    mango_account: Pubkey,
    pp: &PerpPosition,
    pm: &PerpMarket,
) {
    emit_stack(PerpBalanceLog {
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

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct FlashLoanTokenDetailV2 {
    pub token_index: u16,

    /// The amount by which the user's token position changed at the end
    ///
    /// So if the user repaid the approved_amount in full, it'd be 0.
    ///
    /// Does NOT include the loan_origination_fee or deposit_fee, so the true
    /// change is `change_amount - loan_origination_fee - deposit_fee`.
    pub change_amount: i128,

    /// The amount that was a loan (<= approved_amount, depends on user's deposits)
    pub loan: i128,

    /// The fee paid on the loan, not included in `loan` or `change_amount`
    pub loan_origination_fee: i128,

    pub deposit_index: i128,
    pub borrow_index: i128,
    pub price: i128,

    /// Deposit fee paid for positive change_amount.
    ///
    /// Not factored into change_amount.
    pub deposit_fee: i128,

    /// The amount that was transfered out to the user
    pub approved_amount: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct FlashLoanTokenDetailV3 {
    pub token_index: u16,

    /// The amount by which the user's token position changed at the end
    ///
    /// So if the user repaid the approved_amount in full, it'd be 0.
    ///
    /// Does NOT include the loan_origination_fee or deposit_fee, so the true
    /// change is `change_amount - loan_origination_fee - deposit_fee`.
    pub change_amount: i128,

    /// The amount that was a loan (<= approved_amount, depends on user's deposits)
    pub loan: i128,

    /// The fee paid on the loan, not included in `loan` or `change_amount`
    pub loan_origination_fee: i128,

    pub deposit_index: i128,
    pub borrow_index: i128,
    pub price: i128,

    /// Swap fee paid on the in token of a swap.
    ///
    /// Not factored into change_amount.
    pub swap_fee: i128,

    /// The amount that was transfered out to the user
    pub approved_amount: u64,
}

#[event]
pub struct FlashLoanLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_loan_details: Vec<FlashLoanTokenDetail>,
    pub flash_loan_type: FlashLoanType,
}

#[event]
pub struct FlashLoanLogV2 {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_loan_details: Vec<FlashLoanTokenDetailV2>,
    pub flash_loan_type: FlashLoanType,
}

#[event]
pub struct FlashLoanLogV3 {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_loan_details: Vec<FlashLoanTokenDetailV3>,
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
pub struct FillLogV3 {
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
    pub quantity: i64,         // number of base lots
    pub maker_closed_pnl: f64, // settle-token-native units
    pub taker_closed_pnl: f64, // settle-token-native units
}

#[event]
pub struct PerpUpdateFundingLog {
    pub mango_group: Pubkey,
    pub market_index: u16,
    pub long_funding: i128,
    pub short_funding: i128,
    pub price: i128,
    pub oracle_slot: u64,
    pub stable_price: i128,
    pub fees_accrued: i128,
    pub fees_settled: i128,
    pub open_interest: i64,
    pub instantaneous_funding_rate: i128,
}

#[event]
pub struct PerpUpdateFundingLogV2 {
    pub mango_group: Pubkey,
    pub market_index: u16,
    pub long_funding: i128,
    pub short_funding: i128,
    pub price: i128,
    pub oracle_slot: u64,
    pub oracle_confidence: i128,
    pub oracle_type: OracleType,
    pub stable_price: i128,
    pub fees_accrued: i128,
    pub fees_settled: i128,
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
pub struct UpdateRateLogV2 {
    pub mango_group: Pubkey,
    pub token_index: u16,
    // contrary to v1 these do not have curve_scaling factored in!
    pub rate0: i128,    // I80F48
    pub util0: i128,    // I80F48
    pub rate1: i128,    // I80F48
    pub util1: i128,    // I80F48
    pub max_rate: i128, // I80F48
    pub curve_scaling: f64,
    pub target_utilization: f32,
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
pub struct TokenLiqWithTokenLogV2 {
    pub mango_group: Pubkey,
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub asset_token_index: u16,
    pub liab_token_index: u16,
    pub asset_transfer_from_liqee: i128, // I80F48
    pub asset_transfer_to_liqor: i128,   // I80F48
    pub asset_liquidation_fee: i128,     // I80F48
    pub liab_transfer: i128,             // I80F48
    pub asset_price: i128,               // I80F48
    pub liab_price: i128,                // I80F48
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
    TokenConditionalSwapTrigger,
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
pub struct WithdrawLoanLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_index: u16,
    pub loan_amount: i128,
    pub loan_origination_fee: i128,
    pub instruction: LoanOriginationFeeInstruction,
    pub price: Option<i128>, // Ideally would log price everywhere but in serum3_settle_funds oracle is not a passed in account
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
pub struct TokenMetaDataLogV2 {
    pub mango_group: Pubkey,
    pub mint: Pubkey,
    pub token_index: u16,
    pub mint_decimals: u8,
    pub oracle: Pubkey,
    pub fallback_oracle: Pubkey,
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
pub struct PerpLiqBaseOrPositivePnlLogV2 {
    pub mango_group: Pubkey,
    pub perp_market_index: u16,
    pub liqor: Pubkey,
    pub liqee: Pubkey,
    pub base_transfer_liqee: i64,
    pub quote_transfer_liqee: i128,
    pub quote_transfer_liqor: i128,
    pub quote_platform_fee: i128,
    pub pnl_transfer: i128,
    pub pnl_settle_limit_transfer: i128,
    pub price: i128,
}

#[event]
pub struct PerpLiqBaseOrPositivePnlLogV3 {
    pub mango_group: Pubkey,
    pub perp_market_index: u16,
    pub liqor: Pubkey,
    pub liqee: Pubkey,
    pub base_transfer_liqee: i64,
    pub quote_transfer_liqee: i128,
    pub quote_transfer_liqor: i128,
    pub quote_platform_fee: i128,
    pub pnl_transfer: i128,
    pub pnl_settle_limit_transfer_recurring: i64,
    pub pnl_settle_limit_transfer_oneshot: i64,
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

#[event]
pub struct FilledPerpOrderLog {
    pub mango_group: Pubkey,
    pub perp_market_index: u16,
    pub seq_num: u64,
}

#[event]
pub struct PerpTakerTradeLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub perp_market_index: u16,
    pub taker_side: u8,
    pub total_base_lots_taken: i64, // includes decremented base lots
    pub total_base_lots_decremented: i64, // from DecrementTake self-trades
    pub total_quote_lots_taken: i64, // exclusive fees paid, includes decremented quote lots
    pub total_quote_lots_decremented: i64, // from DecrementTake self-trades
    pub taker_fees_paid: i128,      // in native quote units
    pub fee_penalty: i128,          // in native quote units
}

#[event]
pub struct PerpForceClosePositionLog {
    pub mango_group: Pubkey,
    pub perp_market_index: u16,
    pub account_a: Pubkey,
    pub account_b: Pubkey,
    pub base_transfer: i64,
    pub quote_transfer: i128,
    pub price: i128,
}

#[event]
pub struct TokenForceCloseBorrowsWithTokenLog {
    pub mango_group: Pubkey,
    pub liqor: Pubkey,
    pub liqee: Pubkey,
    pub asset_token_index: u16,
    pub liab_token_index: u16,
    pub asset_transfer: i128,
    pub liab_transfer: i128,
    pub asset_price: i128,
    pub liab_price: i128,
    pub fee_factor: i128,
}

#[event]
pub struct TokenForceCloseBorrowsWithTokenLogV2 {
    pub mango_group: Pubkey,
    pub liqor: Pubkey,
    pub liqee: Pubkey,
    pub asset_token_index: u16,
    pub liab_token_index: u16,
    pub asset_transfer_from_liqee: i128, // I80F48
    pub asset_transfer_to_liqor: i128,   // I80F48
    pub asset_liquidation_fee: i128,     // I80F48
    pub liab_transfer: i128,             // I80F48
    pub asset_price: i128,               // I80F48
    pub liab_price: i128,                // I80F48
    /// including liqor and platform liquidation fees
    pub fee_factor: i128, // I80F48
}

#[event]
pub struct TokenConditionalSwapCreateLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub id: u64,
    pub max_buy: u64,
    pub max_sell: u64,
    pub expiry_timestamp: u64,
    pub price_lower_limit: f64,
    pub price_upper_limit: f64,
    pub price_premium_rate: f64,
    pub taker_fee_rate: f32,
    pub maker_fee_rate: f32,
    pub buy_token_index: u16,
    pub sell_token_index: u16,
    pub allow_creating_deposits: bool,
    pub allow_creating_borrows: bool,
}

#[event]
pub struct TokenConditionalSwapCreateLogV2 {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub id: u64,
    pub max_buy: u64,
    pub max_sell: u64,
    pub expiry_timestamp: u64,
    pub price_lower_limit: f64,
    pub price_upper_limit: f64,
    pub price_premium_rate: f64,
    pub taker_fee_rate: f32,
    pub maker_fee_rate: f32,
    pub buy_token_index: u16,
    pub sell_token_index: u16,
    pub allow_creating_deposits: bool,
    pub allow_creating_borrows: bool,
    pub display_price_style: u8,
    pub intention: u8,
}

#[event]
pub struct TokenConditionalSwapCreateLogV3 {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub id: u64,
    pub max_buy: u64,
    pub max_sell: u64,
    pub expiry_timestamp: u64,
    pub price_lower_limit: f64,
    pub price_upper_limit: f64,
    pub price_premium_rate: f64,
    pub taker_fee_rate: f32,
    pub maker_fee_rate: f32,
    pub buy_token_index: u16,
    pub sell_token_index: u16,
    pub allow_creating_deposits: bool,
    pub allow_creating_borrows: bool,
    pub display_price_style: u8,
    pub intention: u8,
    pub tcs_type: u8,
    pub start_timestamp: u64,
    pub duration_seconds: u64,
}

#[event]
pub struct TokenConditionalSwapTriggerLog {
    pub mango_group: Pubkey,
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub token_conditional_swap_id: u64,
    pub buy_token_index: u16,
    pub sell_token_index: u16,
    pub buy_amount: u64,        // amount the liqee got
    pub sell_amount: u64,       // amount the liqee paid (including fees)
    pub maker_fee: u64,         // in native units of sell token (included in sell amount)
    pub taker_fee: u64, // in native units of sell token (deducted from the sell amount the liqor received)
    pub buy_token_price: i128, // I80F48
    pub sell_token_price: i128, // I80F48
    pub closed: bool,
}

#[event]
pub struct TokenConditionalSwapTriggerLogV2 {
    pub mango_group: Pubkey,
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub token_conditional_swap_id: u64,
    pub buy_token_index: u16,
    pub sell_token_index: u16,
    pub buy_amount: u64,        // amount the liqee got
    pub sell_amount: u64,       // amount the liqee paid (including fees)
    pub maker_fee: u64,         // in native units of sell token (included in sell amount)
    pub taker_fee: u64, // in native units of sell token (deducted from the sell amount the liqor received)
    pub buy_token_price: i128, // I80F48
    pub sell_token_price: i128, // I80F48
    pub closed: bool,
    pub display_price_style: u8,
    pub intention: u8,
}

#[event]
pub struct TokenConditionalSwapTriggerLogV3 {
    pub mango_group: Pubkey,
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub token_conditional_swap_id: u64,
    pub buy_token_index: u16,
    pub sell_token_index: u16,
    pub buy_amount: u64,        // amount the liqee got
    pub sell_amount: u64,       // amount the liqee paid (including fees)
    pub maker_fee: u64,         // in native units of sell token (included in sell amount)
    pub taker_fee: u64, // in native units of sell token (deducted from the sell amount the liqor received)
    pub buy_token_price: i128, // I80F48
    pub sell_token_price: i128, // I80F48
    pub closed: bool,
    pub display_price_style: u8,
    pub intention: u8,
    pub tcs_type: u8,
    pub start_timestamp: u64,
}

#[event]
pub struct TokenConditionalSwapCancelLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub id: u64,
}

#[event]
pub struct TokenConditionalSwapStartLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub caller: Pubkey,
    pub token_conditional_swap_id: u64,
    pub incentive_token_index: u16,
    pub incentive_amount: u64,
}

#[event]
pub struct TokenCollateralFeeLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_index: u16,
    pub asset_usage_fraction: i128,
    pub fee: i128,
}

#[event]
pub struct ForceWithdrawLog {
    pub mango_group: Pubkey,
    pub mango_account: Pubkey,
    pub token_index: u16,
    pub quantity: u64,
    pub price: i128, // I80F48
    pub to_token_account: Pubkey,
}
