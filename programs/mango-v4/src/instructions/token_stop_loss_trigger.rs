use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_stop_loss_trigger(
    ctx: Context<TokenStopLossTrigger>,
    token_stop_loss_index: usize,
    liqor_max_buy_token_to_give: u64,
    liqor_max_sell_token_to_receive: u64,
) -> Result<()> {
    let group_pk = &ctx.accounts.group.key();
    let liqee_key = ctx.accounts.liqee.key();
    let liqor_key = ctx.accounts.liqor.key();
    require_keys_neq!(liqee_key, liqor_key);

    let mut liqor = ctx.accounts.liqor.load_full_mut()?;
    require_msg_typed!(
        !liqor.fixed.being_liquidated(),
        MangoError::BeingLiquidated,
        "liqor account"
    );

    let mut account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)
        .context("create account retriever")?;

    let mut liqee = ctx.accounts.liqee.load_full_mut()?;
    let mut liqee_health_cache = new_health_cache(&liqee.borrow(), &account_retriever)
        .context("create liqee health cache")?;
    let liqee_pre_init_health = liqee.check_health_pre(&liqee_health_cache)?;

    let tsl = liqee
        .token_stop_loss_by_index(token_stop_loss_index)?
        .clone();
    require!(tsl.is_active(), MangoError::SomeError);
    // TODO: this check is purely defensive -- keep?
    if tsl.bought >= tsl.max_buy || tsl.sold >= tsl.max_sell {
        let tsl = liqee.token_stop_loss_mut_by_index(token_stop_loss_index)?;
        *tsl = TokenStopLoss::default();
        return Ok(());
    }

    let (buy_bank, buy_token_price, sell_bank_and_oracle_opt) =
        account_retriever.banks_mut_and_oracles(tsl.buy_token_index, tsl.sell_token_index)?;
    let (sell_bank, sell_token_price) = sell_bank_and_oracle_opt.unwrap();
    let price: f32 = (buy_token_price.to_num::<f64>() / sell_token_price.to_num::<f64>()) as f32;

    let now_ts: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
    let (liqee_buy_change, liqee_sell_change) = action(
        &mut liqor.borrow_mut(),
        liqor_key,
        &mut liqee.borrow_mut(),
        liqee_key,
        &tsl,
        token_stop_loss_index,
        buy_bank,
        liqor_max_buy_token_to_give,
        sell_bank,
        liqor_max_sell_token_to_receive,
        price,
        now_ts,
    )?;

    // TODO: log token positions, loan and origination fee amounts, and the ix

    // Check liqee and liqor health after the transaction
    liqee_health_cache.adjust_token_balance(&buy_bank, liqee_buy_change)?;
    liqee_health_cache.adjust_token_balance(&sell_bank, liqee_sell_change)?;
    liqee.check_health_post(&liqee_health_cache, liqee_pre_init_health)?;

    let liqor_health = compute_health(&liqor.borrow(), HealthType::Init, &account_retriever)
        .context("compute liqor health")?;
    require!(liqor_health >= 0, MangoError::HealthMustBePositive);

    Ok(())
}

fn action(
    liqor: &mut MangoAccountRefMut,
    liqor_key: Pubkey,
    liqee: &mut MangoAccountRefMut,
    liqee_key: Pubkey,
    tsl: &TokenStopLoss,
    token_stop_loss_index: usize,
    buy_bank: &mut Bank,
    liqor_max_buy_token_to_give: u64,
    sell_bank: &mut Bank,
    liqor_max_sell_token_to_receive: u64,
    price: f32,
    now_ts: u64,
) -> Result<(I80F48, I80F48)> {
    // amount of sell token native per buy token native
    match tsl.price_threshold_type() {
        TokenStopLossPriceThresholdType::PriceUnderThreshold => {
            require_gt!(tsl.price_threshold, price);
        }
        TokenStopLossPriceThresholdType::PriceOverThreshold => {
            require_gt!(price, tsl.price_threshold);
        }
    }

    // NOTE: can we just leave computing the max-swap amount to the caller? we just do health checks in the end?
    // that would make this simple and obviously safe

    let pre_liqee_buy_token = liqee
        .ensure_token_position(tsl.buy_token_index)?
        .0
        .native(&buy_bank);
    let pre_liqee_sell_token = liqee
        .ensure_token_position(tsl.sell_token_index)?
        .0
        .native(&sell_bank);

    // derive trade amount based on limits in the tsl and by the liqor
    let premium_price = price * (1.0 + (tsl.price_premium_bps as f32) * 0.0001);
    let premium_price_i80f48 = I80F48::from_num(premium_price);
    // TODO: is it ok for these to be in u64? Otherwise a bunch of fields on the tsl would need to be I80F48 too...
    let buy_token_amount;
    let sell_token_amount;
    {
        let mut initial_buy = (tsl.max_buy - tsl.bought).min(liqor_max_buy_token_to_give);
        if !tsl.allow_creating_deposits() {
            // ceil, because we want to end in the 0..1 native token range, so the position can be closed
            initial_buy = initial_buy.min(
                (-pre_liqee_buy_token)
                    .max(I80F48::ZERO)
                    .ceil()
                    .to_num::<u64>(),
            );
        }
        let sell_for_buy = (I80F48::from(initial_buy) * premium_price_i80f48)
            .ceil() // in doubt, increase the liqee's cost slightly
            .to_num::<u64>();

        let mut initial_sell = (tsl.max_sell - tsl.sold)
            .min(liqor_max_sell_token_to_receive)
            .min(sell_for_buy);
        if !tsl.allow_creating_borrows() {
            initial_sell = initial_sell.min(
                pre_liqee_sell_token
                    .max(I80F48::ZERO)
                    .floor()
                    .to_num::<u64>(),
            );
        }
        let buy_for_sell = (I80F48::from(initial_sell) / premium_price_i80f48)
            .floor() // decreases the amount the liqee would get
            .to_num::<u64>();

        buy_token_amount = initial_buy.min(buy_for_sell);
        sell_token_amount = initial_sell;
    }

    // do the token transfer between liqee and liqor
    let buy_token_amount_i80f48 = I80F48::from(buy_token_amount);
    let sell_token_amount_i80f48 = I80F48::from(sell_token_amount);

    let (liqee_buy_token, liqee_buy_raw_index) = liqee.token_position_mut(tsl.buy_token_index)?;
    let (liqor_buy_token, liqor_buy_raw_index, _) =
        liqor.ensure_token_position(tsl.buy_token_index)?;
    let liqee_buy_active = buy_bank.deposit(liqee_buy_token, buy_token_amount_i80f48, now_ts)?;
    let (liqor_buy_active, liqor_buy_loan_origination) =
        buy_bank.withdraw_with_fee(liqor_buy_token, buy_token_amount_i80f48, now_ts)?;

    let (liqee_sell_token, liqee_sell_raw_index) =
        liqee.token_position_mut(tsl.sell_token_index)?;
    let (liqor_sell_token, liqor_sell_raw_index, _) =
        liqor.ensure_token_position(tsl.sell_token_index)?;
    let liqor_sell_active =
        sell_bank.deposit(liqor_sell_token, sell_token_amount_i80f48, now_ts)?;
    let (liqee_sell_active, liqee_sell_loan_origination) =
        sell_bank.withdraw_with_fee(liqee_sell_token, sell_token_amount_i80f48, now_ts)?;

    let post_liqee_sell_token = liqee_sell_token.native(&sell_bank);

    // With a scanning account retriever, it's safe to deactivate inactive token positions immediately
    if !liqee_buy_active {
        liqee.deactivate_token_position_and_log(liqee_buy_raw_index, liqee_key);
    }
    if !liqee_sell_active {
        liqee.deactivate_token_position_and_log(liqee_sell_raw_index, liqee_key);
    }
    if !liqor_buy_active {
        liqor.deactivate_token_position_and_log(liqor_buy_raw_index, liqor_key);
    }
    if !liqor_sell_active {
        liqor.deactivate_token_position_and_log(liqor_sell_raw_index, liqor_key)
    }

    // update tsl information on the account
    {
        // record amount
        let tsl = liqee.token_stop_loss_mut_by_index(token_stop_loss_index)?;
        tsl.bought += buy_token_amount;
        tsl.sold += sell_token_amount;
        assert!(tsl.bought <= tsl.max_buy);
        assert!(tsl.sold <= tsl.max_sell);

        // maybe remove tsl
        // TODO: a tsl should maybe also end if the don't-create-deposits/borrows limit is hit?!
        if tsl.bought >= tsl.max_buy || tsl.sold >= tsl.max_sell {
            *tsl = TokenStopLoss::default();
        }
    }

    // using sell_token_amount_i80f48 here would not account for loan origination fees!
    Ok((
        buy_token_amount_i80f48,
        post_liqee_sell_token - pre_liqee_sell_token,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{self, test::*};
}
