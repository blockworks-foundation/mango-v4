use crate::accounts_zerocopy::*;
use crate::health::*;
use crate::state::*;
use crate::util::clock_now;
use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::logs::{emit_stack, TokenBalanceLog, TokenCollateralFeeLog};

pub fn token_charge_collateral_fees(ctx: Context<TokenChargeCollateralFees>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let mut account = ctx.accounts.account.load_full_mut()?;
    let (now_ts, now_slot) = clock_now();

    if group.collateral_fee_interval == 0 {
        // By resetting, a new enabling of collateral fees will not immediately create a charge
        account.fixed.last_collateral_fee_charge = 0;
        return Ok(());
    }

    // When collateral fees are enabled the first time, don't immediately charge
    if account.fixed.last_collateral_fee_charge == 0 {
        account.fixed.last_collateral_fee_charge = now_ts;
        return Ok(());
    }

    // Is the next fee-charging due?
    let last_charge_ts = account.fixed.last_collateral_fee_charge;
    if now_ts < last_charge_ts + group.collateral_fee_interval {
        return Ok(());
    }
    account.fixed.last_collateral_fee_charge = now_ts;

    // Charge the user at most for 2x the interval. So if no one calls this for a long time
    // there won't be a huge charge based only on the end state.
    let charge_seconds = (now_ts - last_charge_ts).min(2 * group.collateral_fee_interval);

    // The fees are configured in "interest per day" so we need to get the fraction of days
    // that has passed since the last update for scaling
    let inv_seconds_per_day = I80F48::from_num(1.157407407407e-5); // 1 / (24 * 60 * 60)
    let time_scaling = I80F48::from(charge_seconds) * inv_seconds_per_day;

    let health_cache = {
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow(), now_slot)?;
        new_health_cache(&account.borrow(), &retriever, now_ts)?
    };

    // We want to find the total asset health and total liab health, but don't want
    // to treat borrows that moved into open orders accounts as realized. Hence we
    // pretend all spot orders are closed and settled and add their funds back to
    // the token positions.
    let mut token_balances = health_cache.effective_token_balances(HealthType::Maint);
    for spot_info in health_cache.spot_infos.iter() {
        token_balances[spot_info.base_info_index].spot_and_perp += spot_info.reserved_base;
        token_balances[spot_info.quote_info_index].spot_and_perp += spot_info.reserved_quote;
    }

    let mut total_liab_health = I80F48::ZERO;
    let mut total_asset_health = I80F48::ZERO;
    for (info, balance) in health_cache.token_infos.iter().zip(token_balances.iter()) {
        let health = info.health_contribution(HealthType::Maint, balance.spot_and_perp);
        if health.is_positive() {
            total_asset_health += health;
        } else {
            total_liab_health -= health;
        }
    }

    // If there's no assets or no liabs, we can't charge fees
    if total_asset_health.is_zero() || total_liab_health.is_zero() {
        return Ok(());
    }

    let token_position_count = account.active_token_positions().count();

    // Rather than pay by the pro-rata collateral fees across all assets that
    // are used as collateral, an account should get credit for their most
    // credit-worthy assets first, then others in order. Without this sorting,
    // suppose a user has enough collateral to cover their liabilities using
    // just tokens without a collateral fee. Once they add additional collateral
    // of a type that does have a fee, if all collateral was contributing to
    // this health pro-rata, they would now have a fee as a result of adding
    // collateral without a new liability.
    let mut collateral_fee_per_day_and_bank_ai_index = Vec::with_capacity(token_position_count);
    for index in 0..token_position_count {
        let bank_ai = &ctx.remaining_accounts[index];
        let bank = bank_ai.load::<Bank>()?;
        let collateral_fee_per_day = bank.collateral_fee_per_day;

        collateral_fee_per_day_and_bank_ai_index.push((collateral_fee_per_day, index));
    }
    // Custom sort because f32 doesnt have sort by default.
    collateral_fee_per_day_and_bank_ai_index.sort_by(|a, b| (a.0).partial_cmp(&b.0).unwrap());

    // Remaining amount of liability health that needs to be covered by collateral.
    let mut remaining_liab = total_liab_health;
    for (_collateral_fee, index) in collateral_fee_per_day_and_bank_ai_index.iter() {
        let bank_ai = &ctx.remaining_accounts[*index];
        let mut bank = bank_ai.load_mut::<Bank>()?;
        if bank.collateral_fee_per_day <= 0.0 || bank.maint_asset_weight.is_zero() {
            continue;
        }

        let (token_position, raw_token_index) = account.token_position_mut(bank.token_index)?;
        let token_balance = token_balances[*index].spot_and_perp;
        if token_balance <= 0 {
            continue;
        }

        // Contribution from this bank used as collateral. This is always
        // positive since the check above guarantees token balance is positive.
        let possible_health_contribution =
            health_cache.token_infos[*index].health_contribution(HealthType::Maint, token_balance);

        let asset_usage_scaling = if possible_health_contribution < remaining_liab {
            remaining_liab -= possible_health_contribution;
            I80F48::ONE
        } else {
            let scaling = remaining_liab / possible_health_contribution;
            remaining_liab = I80F48::ZERO;
            scaling
        };

        let fee = token_balance
            * asset_usage_scaling
            * time_scaling
            * I80F48::from_num(bank.collateral_fee_per_day);
        assert!(fee <= token_balance);

        let is_active = bank.withdraw_without_fee(token_position, fee, now_ts)?;
        if !is_active {
            account.deactivate_token_position_and_log(raw_token_index, ctx.accounts.account.key());
        }

        bank.collected_fees_native += fee;
        bank.collected_collateral_fees += fee;

        let token_info = health_cache.token_info(bank.token_index)?;
        let token_position = account.token_position(bank.token_index)?;

        emit_stack(TokenCollateralFeeLog {
            mango_group: ctx.accounts.group.key(),
            mango_account: ctx.accounts.account.key(),
            token_index: bank.token_index,
            fee: fee.to_bits(),
            asset_usage_fraction: asset_usage_scaling.to_bits(),
            price: token_info.prices.oracle.to_bits(),
        });

        emit_stack(TokenBalanceLog {
            mango_group: ctx.accounts.group.key(),
            mango_account: ctx.accounts.account.key(),
            token_index: bank.token_index,
            indexed_position: token_position.indexed_position.to_bits(),
            deposit_index: bank.deposit_index.to_bits(),
            borrow_index: bank.borrow_index.to_bits(),
        });

        // Once all liability health is covered, no more need to charge collateral fees.
        if remaining_liab <= I80F48::ZERO {
            break;
        }
    }

    Ok(())
}
