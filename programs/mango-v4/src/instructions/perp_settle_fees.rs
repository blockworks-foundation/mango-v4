use anchor_lang::prelude::*;

use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::{compute_health, new_fixed_order_account_retriever, HealthType};
use crate::state::*;

use crate::accounts_ix::*;
use crate::logs::{emit_perp_balances, emit_stack, PerpSettleFeesLog, TokenBalanceLog};

pub fn perp_settle_fees(ctx: Context<PerpSettleFees>, max_settle_amount: u64) -> Result<()> {
    // max_settle_amount must greater than zero
    require!(
        max_settle_amount > 0,
        MangoError::MaxSettleAmountMustBeGreaterThanZero
    );

    let mut account = ctx.accounts.account.load_full_mut()?;
    let mut settle_bank = ctx.accounts.settle_bank.load_mut()?;
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    // Verify that the bank is the quote currency bank (#2)
    require_eq!(
        settle_bank.token_index,
        perp_market.settle_token_index,
        MangoError::InvalidBank
    );

    // Get oracle prices
    let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
    let oracle_price = perp_market.oracle_price(
        &OracleAccountInfos::from_reader(oracle_ref),
        None, // staleness checked in health
    )?;
    let settle_oracle_ref = &AccountInfoRef::borrow(ctx.accounts.settle_oracle.as_ref())?;
    let settle_token_oracle_price = settle_bank.oracle_price(
        &OracleAccountInfos::from_reader(settle_oracle_ref),
        None, // staleness checked in health
    )?;

    // Fetch perp positions for accounts
    let perp_position = account.perp_position_mut(perp_market.perp_market_index)?;

    // Settle funding before settling any PnL
    perp_position.settle_funding(&perp_market);

    // Calculate PnL
    let pnl = perp_position.unsettled_pnl(&perp_market, oracle_price)?;

    let settleable_pnl = perp_position.apply_pnl_settle_limit(&perp_market, pnl);

    if !settleable_pnl.is_negative() || !perp_market.fees_accrued.is_positive() {
        msg!(
            "Not settling: pnl {}, perp_market.fees_accrued {}, settleable_pnl {}",
            pnl,
            perp_market.fees_accrued,
            settleable_pnl
        );
        return Ok(());
    }

    // Settle for the maximum possible capped to max_settle_amount
    let settlement = settleable_pnl
        .abs()
        .min(perp_market.fees_accrued.abs())
        .min(I80F48::from(max_settle_amount));
    require!(settlement >= 0, MangoError::SettlementAmountMustBePositive);

    perp_position.record_settle(-settlement, &perp_market); // settle the negative pnl on the user perp position
    perp_market.fees_accrued -= settlement;

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.account.key(),
        perp_position,
        &perp_market,
    );

    // Update the account's perp_spot_transfers with the new PnL
    let settlement_i64 = settlement.round().to_num::<i64>();

    // Safety check to prevent any accidental negative transfer
    require!(
        settlement_i64 >= 0,
        MangoError::SettlementAmountMustBePositive
    );

    perp_position.perp_spot_transfers -= settlement_i64;
    account.fixed.perp_spot_transfers -= settlement_i64;

    // Transfer token balances
    let token_position = account
        .token_position_mut(perp_market.settle_token_index)?
        .0;
    settle_bank.withdraw_without_fee(
        token_position,
        settlement,
        Clock::get()?.unix_timestamp.try_into().unwrap(),
    )?;
    // Update the settled balance on the market itself
    perp_market.fees_settled += settlement;

    emit_stack(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        token_index: perp_market.settle_token_index,
        indexed_position: token_position.indexed_position.to_bits(),
        deposit_index: settle_bank.deposit_index.to_bits(),
        borrow_index: settle_bank.borrow_index.to_bits(),
    });

    emit_stack(PerpSettleFeesLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        perp_market_index: perp_market.perp_market_index,
        settlement: settlement.to_bits(),
    });

    // Bank & perp_market are dropped to prevent re-borrow from remaining_accounts
    drop(settle_bank);
    drop(perp_market);

    // Verify that the result of settling did not violate the health of the account that lost money
    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    let health = compute_health(&account.borrow(), HealthType::Init, &retriever, now_ts)?;
    require!(health >= 0, MangoError::HealthMustBePositive);

    msg!("settled fees = {}", settlement);
    Ok(())
}
