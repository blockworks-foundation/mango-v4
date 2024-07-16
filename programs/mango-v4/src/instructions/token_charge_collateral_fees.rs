use crate::accounts_zerocopy::*;
use crate::health::*;
use crate::state::*;
use crate::util::clock_now;
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use std::collections::HashMap;
use std::ops::{Deref, Div};

use crate::accounts_ix::*;
use crate::logs::{emit_stack, TokenBalanceLog, TokenCollateralFeeLog};

pub fn token_charge_collateral_fees(ctx: Context<TokenChargeCollateralFees>) -> Result<()> {
    token_charge_collateral_fees_internal(
        ctx.accounts.account.load_full_mut()?,
        ctx.accounts.group.load()?.deref(),
        &ctx.remaining_accounts,
        ctx.accounts.group.key(),
        ctx.accounts.account.key(),
        clock_now(),
        None,
    )
}

pub fn token_charge_collateral_fees_internal<Header, Fixed, Dynamic>(
    mut account: DynamicAccount<Header, Fixed, Dynamic>,
    group: &Group,
    remaining_accounts: &[AccountInfo],
    group_key: Pubkey,
    account_key: Pubkey,
    now: (u64, u64),
    mut out_fees: Option<&mut HashMap<TokenIndex, (I80F48, I80F48)>>,
) -> Result<()>
where
    Header: DerefOrBorrowMut<MangoAccountDynamicHeader> + DerefOrBorrow<MangoAccountDynamicHeader>,
    Fixed: DerefOrBorrowMut<MangoAccountFixed> + DerefOrBorrow<MangoAccountFixed>,
    Dynamic: DerefOrBorrowMut<[u8]> + DerefOrBorrow<[u8]>,
{
    let mut account = account.borrow_mut();
    let (now_ts, now_slot) = now;

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
            new_fixed_order_account_retriever(remaining_accounts, &account.borrow(), now_slot)?;
        new_health_cache(&account.borrow(), &retriever, now_ts)?
    };

    let (_, liabs) = health_cache.assets_and_liabs();
    // Account with liabs below ~100$ should not be charged any collateral fees
    if liabs < 100_000_000 {
        // msg!("liabs {}, below threshold to charge collateral fees", liabs);
        return Ok(());
    }

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

    // Users only pay for assets that are actively used to cover their liabilities.
    let asset_usage_scaling = (total_liab_health / total_asset_health)
        .max(I80F48::ZERO)
        .min(I80F48::ONE);

    let scaling = asset_usage_scaling * time_scaling;

    let mut total_collateral_fees_in_usd = I80F48::ZERO;

    let token_position_count = account.active_token_positions().count();
    for bank_ai in &remaining_accounts[0..token_position_count] {
        let mut bank = bank_ai.load_mut::<Bank>()?;
        if bank.collateral_fee_per_day <= 0.0 || bank.maint_asset_weight.is_zero() {
            continue;
        }

        let token_index_in_health_cache = health_cache
            .token_infos
            .iter()
            .position(|x| x.token_index == bank.token_index)
            .expect("missing token in health");
        let token_balance = token_balances[token_index_in_health_cache].spot_and_perp;
        if token_balance <= 0 {
            continue;
        }

        let fee = token_balance * scaling * I80F48::from_num(bank.collateral_fee_per_day);
        assert!(fee <= token_balance);

        let token_info = health_cache.token_info(bank.token_index)?;

        total_collateral_fees_in_usd += fee * token_info.prices.oracle;

        bank.collected_collateral_fees += fee;

        emit_stack(TokenCollateralFeeLog {
            mango_group: group_key,
            mango_account: account_key,
            token_index: bank.token_index,
            fee: fee.to_bits(),
            asset_usage_fraction: asset_usage_scaling.to_bits(),
            price: token_info.prices.oracle.to_bits(),
        });
    }

    for bank_ai in &remaining_accounts[0..token_position_count] {
        let mut bank = bank_ai.load_mut::<Bank>()?;

        let token_info = health_cache.token_info(bank.token_index)?;

        let token_index_in_health_cache = health_cache
            .token_infos
            .iter()
            .position(|x| x.token_index == bank.token_index)
            .expect("missing token in health");

        let token_balance = token_balances[token_index_in_health_cache].spot_and_perp;
        let health = token_info.health_contribution(HealthType::Maint, token_balance);

        if health >= 0 {
            continue;
        }

        let (token_position, raw_token_index) = account.token_position_mut(bank.token_index)?;

        let borrow_scaling = (health / total_liab_health).abs();
        let fee = borrow_scaling * total_collateral_fees_in_usd / token_info.prices.oracle;

        if let Some(ref mut output) = out_fees {
            output.insert(
                token_info.token_index,
                (
                    fee,
                    (fee * token_info.prices.oracle).div(I80F48::from_num(1_000_000)),
                ),
            );
        }

        let is_active = bank.withdraw_without_fee(token_position, fee, now_ts)?;
        if !is_active {
            account.deactivate_token_position_and_log(raw_token_index, account_key);
        }

        bank.collected_fees_native += fee;
        let token_position = account.token_position(bank.token_index)?;

        emit_stack(TokenBalanceLog {
            mango_group: group_key,
            mango_account: account_key,
            token_index: bank.token_index,
            indexed_position: token_position.indexed_position.to_bits(),
            deposit_index: bank.deposit_index.to_bits(),
            borrow_index: bank.borrow_index.to_bits(),
        })
    }

    Ok(())
}
