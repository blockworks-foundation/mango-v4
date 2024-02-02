use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::associated_token;
use anchor_spl::token;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::logs::{
    emit_stack, LoanOriginationFeeInstruction, TokenBalanceLog, WithdrawLoanLog, WithdrawLog,
};

pub fn token_charge_collateral_fees(ctx: Context<TokenChargeCollateralFees>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    if group.collateral_fee_interval == 0 {
        return Ok(());
    }

    let mut account = ctx.accounts.account.load_full_mut()?;
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    // TODO: should we charge collateral fees for accounts that are being liquidated?
    require!(!account.being_liquidated(), MangoError::BeingLiquidated);

    // Is the next fee-charging due?
    let last_charge_ts = account.fixed.last_collateral_fee_charge;
    if now_ts < last_charge_ts + group.collateral_fee_interval {
        return Ok(());
    }
    account.fixed.last_collateral_fee_charge = now_ts;

    // Charge the user at most for 2x the interval. So if no one calls this for a long time
    // there won't be a huge charge based only on the end state.
    let charge_seconds = (now_ts - last_charge_ts).min(2 * group.collateral_fee_interval);

    let inv_seconds_per_day = I80F48::from_num(1.157407407407e-5); // 1 / (24 * 60 * 60)
    let collateral_fee_scaling = I80F48::from(charge_seconds) * inv_seconds_per_day;

    // TODO: Get health cache to compute total maint liabs and total maint assets
    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
    let health_cache = new_health_cache(&account.borrow(), &retriever, now_ts)?;

    let token_position_count = account.active_token_positions().count();
    for bank_ai in &ctx.remaining_accounts[0..token_position_count] {
        let mut bank = bank_ai.load_mut::<Bank>()?;
        if bank.collateral_fee_per_day <= 0.0 {
            continue;
        }

        let (token_position, raw_token_index) = account.token_position_mut(bank.token_index)?;
        let token_balance = token_position.native(&bank);
        if token_balance <= 0 {
            continue;
        }

        // TODO: Get the right amounts
        let used_collateral = token_balance; // depends on liab size, this asset size and total asset size
        let fee = used_collateral
            * I80F48::from_num(bank.collateral_fee_per_day)
            * collateral_fee_scaling;
        assert!(fee <= token_balance);

        let is_active = bank.withdraw_without_fee(token_position, fee, now_ts)?;
        if !is_active {
            account.deactivate_token_position_and_log(raw_token_index, ctx.accounts.account.key());
        }

        bank.collected_fees_native += fee;
        bank.collected_collateral_fees += fee;

        // TODO: emit a log
    }

    Ok(())
}
