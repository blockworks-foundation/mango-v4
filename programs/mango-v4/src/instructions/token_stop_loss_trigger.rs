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

fn trade_amount(
    tsl: &TokenStopLoss,
    sell_per_buy_price: I80F48,
    liqor_max_buy_token_to_give: u64,
    liqor_max_sell_token_to_receive: u64,
    buy_balance: I80F48,
    sell_balance: I80F48,
) -> (u64, u64) {
    let max_buy = liqor_max_buy_token_to_give.min(tsl.remaining_buy()).min(
        if tsl.allow_creating_deposits() {
            u64::MAX
        } else {
            // ceil() because we're ok reaching 0..1 deposited native tokens
            (-buy_balance).max(I80F48::ZERO).ceil().to_num::<u64>()
        },
    );
    let max_sell = liqor_max_sell_token_to_receive
        .min(tsl.remaining_sell())
        .min(if tsl.allow_creating_borrows() {
            u64::MAX
        } else {
            // floor() so we never go below 0
            sell_balance.max(I80F48::ZERO).floor().to_num::<u64>()
        });
    trade_amount_inner(max_buy, max_sell, sell_per_buy_price)
}

fn trade_amount_inner(max_buy: u64, max_sell: u64, sell_per_buy_price: I80F48) -> (u64, u64) {
    let buy_for_sell: u64 = if sell_per_buy_price > I80F48::ONE {
        (I80F48::from(max_sell) / sell_per_buy_price)
            .floor()
            .to_num()
    } else {
        // This logic looks confusing, but please check the test_trade_amount_inner
        ((I80F48::from(max_buy) * sell_per_buy_price)
            .floor()
            .min(I80F48::from(max_sell))
            / sell_per_buy_price)
            .ceil()
            .to_num()
    };
    let buy_amount = max_buy.min(buy_for_sell);
    let sell_for_buy = (I80F48::from(buy_amount) * sell_per_buy_price)
        .floor()
        .to_num::<u64>();

    (buy_amount, max_sell.min(sell_for_buy))
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
    let (buy_token_amount, sell_token_amount) = trade_amount(
        &tsl,
        premium_price_i80f48,
        liqor_max_buy_token_to_give,
        liqor_max_sell_token_to_receive,
        pre_liqee_buy_token,
        pre_liqee_sell_token,
    );

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

    #[test]
    fn test_trade_amount_inner() {
        let cases = vec![
            ("null 1", (0, 0, 1.0), (0, 0)),
            ("null 2", (0, 10, 1.0), (0, 0)),
            ("null 3", (10, 0, 1.0), (0, 0)),
            ("buy limit 1", (10, 30, 2.11), (10, 21)),
            ("buy limit 2", (10, 50, 0.75), (10, 7)),
            ("sell limit 1", (10, 15, 2.1), (7, 14)),
            ("sell limit 2", (10, 5, 0.75), (7, 5)),
            ("less than one 1", (10, 10, 100.0), (0, 0)),
            ("less than one 2", (10, 10, 0.001), (0, 0)),
            ("round 1", (10, 110, 100.0), (1, 100)),
            ("round 2", (199, 10, 0.01), (100, 1)),
        ];

        for (name, (max_buy, max_sell, price), (buy_amount, sell_amount)) in cases {
            println!("{name}");

            let (actual_buy, actual_sell) =
                trade_amount_inner(max_buy, max_sell, I80F48::from_num(price));
            println!("actual: {actual_buy} {actual_sell}, expected: {buy_amount}, {sell_amount}");
            assert_eq!(actual_buy, buy_amount);
            assert_eq!(actual_sell, (actual_buy as f64 * price).floor() as u64); // invariant
            assert_eq!(actual_sell, sell_amount);
        }
    }

    #[test]
    fn test_trade_amount_outer() {
        let cases = vec![
            (
                "limit 1",
                (1, 100, 100, 100, 0.0, 0.0, true, true, 1.0),
                (1, 1),
            ),
            (
                "limit 2",
                (100, 1, 100, 100, 0.0, 0.0, true, true, 1.0),
                (1, 1),
            ),
            (
                "limit 3",
                (100, 100, 1, 100, 0.0, 0.0, true, true, 1.0),
                (1, 1),
            ),
            (
                "limit 4",
                (100, 100, 100, 1, 0.0, 0.0, true, true, 1.0),
                (1, 1),
            ),
            (
                "limit 5",
                (100, 100, 100, 100, -0.3, 0.0, false, true, 1.0),
                (1, 1),
            ),
            (
                "limit 6",
                (100, 100, 100, 100, 0.0, 1.8, true, false, 1.0),
                (1, 1),
            ),
            (
                "full 1",
                (100, 100, 100, 100, -100.0, 100.0, false, false, 1.0),
                (100, 100),
            ),
            (
                "full 2",
                (100, 100, 100, 100, 0.0, 0.0, true, true, 1.0),
                (100, 100),
            ),
            (
                "price 1",
                (100, 100, 100, 100, 0.0, 0.0, true, true, 1.23456),
                (81, 99),
            ),
            (
                "price 2",
                (100, 100, 100, 100, 0.0, 0.0, true, true, 0.76543),
                (100, 76),
            ),
        ];

        for (
            name,
            (
                tsl_buy,
                tsl_sell,
                liqor_buy,
                liqor_sell,
                buy_balance,
                sell_balance,
                allow_deposit,
                allow_borrow,
                price,
            ),
            (buy_amount, sell_amount),
        ) in cases
        {
            println!("{name}");

            let tsl = TokenStopLoss {
                max_buy: 42 + tsl_buy,
                max_sell: 100 + tsl_sell,
                bought: 42,
                sold: 100,
                allow_creating_borrows: u8::from(allow_borrow),
                allow_creating_deposits: u8::from(allow_deposit),
                ..Default::default()
            };

            let (actual_buy, actual_sell) = trade_amount(
                &tsl,
                I80F48::from_num(price),
                liqor_buy,
                liqor_sell,
                I80F48::from_num(buy_balance),
                I80F48::from_num(sell_balance),
            );
            println!("actual: {actual_buy} {actual_sell}, expected: {buy_amount}, {sell_amount}");
            assert_eq!(actual_buy, buy_amount);
            assert_eq!(actual_sell, (actual_buy as f64 * price).floor() as u64); // invariant
            assert_eq!(actual_sell, sell_amount);
        }
    }
}
