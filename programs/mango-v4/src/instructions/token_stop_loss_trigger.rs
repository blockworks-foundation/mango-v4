use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::logs::{
    LoanOriginationFeeInstruction, TokenBalanceLog, TokenConditionalSwapTriggerLog,
    WithdrawLoanOriginationFeeLog,
};
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_conditional_swap_trigger(
    ctx: Context<TokenConditionalSwapTrigger>,
    token_conditional_swap_index: usize,
    token_conditional_swap_id: u64,
    max_buy_token_to_liqee: u64,
    max_sell_token_to_liqor: u64,
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

    let tcs = liqee.token_conditional_swap_by_index(token_conditional_swap_index)?;
    require!(tcs.is_active(), MangoError::SomeError);
    require_eq!(tcs.id, token_conditional_swap_id);

    let (buy_bank, buy_token_price, sell_bank_and_oracle_opt) =
        account_retriever.banks_mut_and_oracles(tcs.buy_token_index, tcs.sell_token_index)?;
    let (sell_bank, sell_token_price) = sell_bank_and_oracle_opt.unwrap();

    let now_ts: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
    let (liqee_buy_change, liqee_sell_change) = action(
        &mut liqor.borrow_mut(),
        liqor_key,
        &mut liqee.borrow_mut(),
        liqee_key,
        token_conditional_swap_index,
        buy_bank,
        buy_token_price,
        max_buy_token_to_liqee,
        sell_bank,
        sell_token_price,
        max_sell_token_to_liqor,
        now_ts,
    )?;

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
    tcs: &TokenConditionalSwap,
    sell_per_buy_price: I80F48,
    max_buy_token_to_liqee: u64,
    max_sell_token_to_liqor: u64,
    buy_balance: I80F48,
    sell_balance: I80F48,
) -> (u64, u64) {
    let max_buy =
        max_buy_token_to_liqee
            .min(tcs.remaining_buy())
            .min(if tcs.allow_creating_deposits() {
                u64::MAX
            } else {
                // ceil() because we're ok reaching 0..1 deposited native tokens
                (-buy_balance).max(I80F48::ZERO).ceil().to_num::<u64>()
            });
    let max_sell =
        max_sell_token_to_liqor
            .min(tcs.remaining_sell())
            .min(if tcs.allow_creating_borrows() {
                u64::MAX
            } else {
                // floor() so we never go below 0
                sell_balance.max(I80F48::ZERO).floor().to_num::<u64>()
            });
    trade_amount_inner(max_buy, max_sell, sell_per_buy_price)
}

fn trade_amount_inner(max_buy: u64, max_sell: u64, sell_per_buy_price: I80F48) -> (u64, u64) {
    // This logic looks confusing, but also check the test_trade_amount_inner
    let buy_for_sell: u64 = if sell_per_buy_price > I80F48::ONE {
        // Example: max_sell=1 and price=1.9. Result should be buy=1, sell=1
        // since we're ok flooring the sell amount.
        ((I80F48::from(max_sell) + I80F48::ONE - I80F48::DELTA) / sell_per_buy_price)
            .floor()
            .to_num()
    } else {
        // Example: max_buy=7, max_sell=4, price=0.6. Result should be buy=7, sell=4
        // Example: max_buy=1, max_sell=1, price=0.01. Result should be 0, 0
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
    let sell_amount = max_sell.min(sell_for_buy);

    // Invariant: never exchange something for nothing.
    // the proof for buy==0 => sell==0 is trivial, other directly is less clear but should hold
    assert!(!((buy_amount > 0) ^ (sell_amount > 0)));

    (buy_amount, sell_amount)
}

fn action(
    liqor: &mut MangoAccountRefMut,
    liqor_key: Pubkey,
    liqee: &mut MangoAccountRefMut,
    liqee_key: Pubkey,
    token_conditional_swap_index: usize,
    buy_bank: &mut Bank,
    buy_token_price: I80F48,
    max_buy_token_to_liqee: u64,
    sell_bank: &mut Bank,
    sell_token_price: I80F48,
    max_sell_token_to_liqor: u64,
    now_ts: u64,
) -> Result<(I80F48, I80F48)> {
    let tcs = liqee
        .token_conditional_swap_by_index(token_conditional_swap_index)?
        .clone();
    require!(tcs.is_active(), MangoError::SomeError);
    require_eq!(buy_bank.token_index, tcs.buy_token_index);
    require_eq!(sell_bank.token_index, tcs.sell_token_index);

    // amount of sell token native per buy token native
    let price: f32 = (buy_token_price.to_num::<f64>() / sell_token_price.to_num::<f64>()) as f32;
    require_gte!(
        price,
        tcs.price_threshold,
        MangoError::StopLossPriceThresholdNotReached
    );

    let premium_price = tcs.execution_price(price);
    require_gte!(tcs.price_limit, premium_price);
    let premium_price_i80f48 = I80F48::from_num(premium_price);

    let pre_liqee_buy_token = liqee
        .ensure_token_position(tcs.buy_token_index)?
        .0
        .native(&buy_bank);
    let pre_liqee_sell_token = liqee
        .ensure_token_position(tcs.sell_token_index)?
        .0
        .native(&sell_bank);

    // derive trade amount based on limits in the tcs and by the liqor
    let (buy_token_amount, sell_token_amount) = trade_amount(
        &tcs,
        premium_price_i80f48,
        max_buy_token_to_liqee,
        max_sell_token_to_liqor,
        pre_liqee_buy_token,
        pre_liqee_sell_token,
    );
    // NOTE: It's possible that buy_token_amount == sell_token_amount == 0!
    // Proceed with it anyway because we already mutated the account anyway and might want
    // to drop the token stop loss entry later.

    // do the token transfer between liqee and liqor
    let buy_token_amount_i80f48 = I80F48::from(buy_token_amount);
    let sell_token_amount_i80f48 = I80F48::from(sell_token_amount);

    let (liqee_buy_token, liqee_buy_raw_index) = liqee.token_position_mut(tcs.buy_token_index)?;
    let (liqor_buy_token, liqor_buy_raw_index, _) =
        liqor.ensure_token_position(tcs.buy_token_index)?;
    let liqee_buy_active = buy_bank.deposit(liqee_buy_token, buy_token_amount_i80f48, now_ts)?;
    let (liqor_buy_active, liqor_buy_loan_origination) =
        buy_bank.withdraw_with_fee(liqor_buy_token, buy_token_amount_i80f48, now_ts)?;

    let post_liqee_buy_token = liqee_buy_token.native(&buy_bank);
    let post_liqor_buy_token = liqor_buy_token.native(&buy_bank);

    let (liqee_sell_token, liqee_sell_raw_index) =
        liqee.token_position_mut(tcs.sell_token_index)?;
    let (liqor_sell_token, liqor_sell_raw_index, _) =
        liqor.ensure_token_position(tcs.sell_token_index)?;
    let liqor_sell_active =
        sell_bank.deposit(liqor_sell_token, sell_token_amount_i80f48, now_ts)?;
    let (liqee_sell_active, liqee_sell_loan_origination) =
        sell_bank.withdraw_with_fee(liqee_sell_token, sell_token_amount_i80f48, now_ts)?;

    let post_liqee_sell_token = liqee_sell_token.native(&sell_bank);
    let post_liqor_sell_token = liqor_sell_token.native(&sell_bank);

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

    // Log info

    // liqee buy token
    emit!(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqee_key,
        token_index: tcs.buy_token_index,
        indexed_position: post_liqee_buy_token.to_bits(),
        deposit_index: buy_bank.deposit_index.to_bits(),
        borrow_index: buy_bank.borrow_index.to_bits(),
    });
    // liqee sell token
    emit!(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqee_key,
        token_index: tcs.sell_token_index,
        indexed_position: post_liqee_sell_token.to_bits(),
        deposit_index: sell_bank.deposit_index.to_bits(),
        borrow_index: sell_bank.borrow_index.to_bits(),
    });
    // liqor buy token
    emit!(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqor_key,
        token_index: tcs.buy_token_index,
        indexed_position: post_liqor_buy_token.to_bits(),
        deposit_index: buy_bank.deposit_index.to_bits(),
        borrow_index: buy_bank.borrow_index.to_bits(),
    });
    // liqor sell token
    emit!(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqor_key,
        token_index: tcs.sell_token_index,
        indexed_position: post_liqor_sell_token.to_bits(),
        deposit_index: sell_bank.deposit_index.to_bits(),
        borrow_index: sell_bank.borrow_index.to_bits(),
    });

    if liqor_buy_loan_origination.is_positive() {
        emit!(WithdrawLoanOriginationFeeLog {
            mango_group: liqee.fixed.group,
            mango_account: liqor_key,
            token_index: tcs.buy_token_index,
            loan_origination_fee: liqor_buy_loan_origination.to_bits(),
            instruction: LoanOriginationFeeInstruction::TokenConditionalSwapTrigger
        });
    }
    if liqee_sell_loan_origination.is_positive() {
        emit!(WithdrawLoanOriginationFeeLog {
            mango_group: liqee.fixed.group,
            mango_account: liqee_key,
            token_index: tcs.sell_token_index,
            loan_origination_fee: liqee_sell_loan_origination.to_bits(),
            instruction: LoanOriginationFeeInstruction::TokenConditionalSwapTrigger
        });
    }

    // update tcs information on the account
    let closed = {
        // record amount
        let tcs = liqee.token_conditional_swap_mut_by_index(token_conditional_swap_index)?;
        tcs.bought += buy_token_amount;
        tcs.sold += sell_token_amount;
        assert!(tcs.bought <= tcs.max_buy);
        assert!(tcs.sold <= tcs.max_sell);

        // Maybe remove token stop loss entry
        //
        // This drops the tcs if no more swapping is possible at the current price:
        // - if bought/sold reached the max
        // - if the "don't create deposits/borrows" constraint is reached
        // - if the price is such that swapping 1 native token would already exceed the limit
        let (future_buy, future_sell) = trade_amount(
            tcs,
            premium_price_i80f48,
            u64::MAX,
            u64::MAX,
            post_liqee_buy_token,
            post_liqee_sell_token,
        );
        if future_buy == 0 || future_sell == 0 {
            *tcs = TokenConditionalSwap::default();
            true
        } else {
            false
        }
    };

    emit!(TokenConditionalSwapTriggerLog {
        mango_group: liqee.fixed.group,
        liqee: liqee_key,
        liqor: liqor_key,
        token_conditional_swap_id: tcs.id,
        buy_token_index: tcs.buy_token_index,
        sell_token_index: tcs.sell_token_index,
        buy_amount: buy_token_amount,
        sell_amount: sell_token_amount,
        buy_token_price: buy_token_price.to_bits(),
        sell_token_price: sell_token_price.to_bits(),
        closed,
    });

    // Return the change in liqee token account balances
    Ok((
        buy_token_amount_i80f48,
        // using sell_token_amount_i80f48 here would not account for loan origination fees!
        post_liqee_sell_token - pre_liqee_sell_token,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::test::*;

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
            ("sell limit 3", (50, 50, 1.1), (46, 50)),
            ("sell limit 4", (60, 50, 0.9), (56, 50)),
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
                tcs_buy,
                tcs_sell,
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

            let tcs = TokenConditionalSwap {
                max_buy: 42 + tcs_buy,
                max_sell: 100 + tcs_sell,
                bought: 42,
                sold: 100,
                allow_creating_borrows: u8::from(allow_borrow),
                allow_creating_deposits: u8::from(allow_deposit),
                ..Default::default()
            };

            let (actual_buy, actual_sell) = trade_amount(
                &tcs,
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

    #[derive(Clone)]
    struct TestSetup {
        asset_bank: TestAccount<Bank>,
        liab_bank: TestAccount<Bank>,
        liqee: MangoAccountValue,
        liqor: MangoAccountValue,
    }

    impl TestSetup {
        fn new() -> Self {
            let group = Pubkey::new_unique();
            let (asset_bank, asset_oracle) = mock_bank_and_oracle(group, 0, 1.0, 0.0, 0.0);
            let (liab_bank, liab_oracle) = mock_bank_and_oracle(group, 1, 1.0, 0.0, 0.0);

            let mut liqee_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            liqee_buffer.extend_from_slice(&[0u8; 256]);
            let mut liqee = MangoAccountValue::from_bytes(&liqee_buffer).unwrap();
            {
                liqee.expand_dynamic_content(3, 5, 4, 6, 1).unwrap();
                liqee.ensure_token_position(0).unwrap();
                liqee.ensure_token_position(1).unwrap();
            }

            let liqor_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            let mut liqor = MangoAccountValue::from_bytes(&liqor_buffer).unwrap();
            {
                liqor.ensure_token_position(0).unwrap();
                liqor.ensure_token_position(1).unwrap();
            }

            Self {
                asset_bank,
                liab_bank,
                liqee,
                liqor,
            }
        }

        fn liqee_asset_pos(&mut self) -> I80F48 {
            self.liqee
                .token_position(0)
                .unwrap()
                .native(self.asset_bank.data())
        }
        fn liqee_liab_pos(&mut self) -> I80F48 {
            self.liqee
                .token_position(1)
                .unwrap()
                .native(self.liab_bank.data())
        }
        fn liqor_asset_pos(&mut self) -> I80F48 {
            self.liqor
                .token_position(0)
                .unwrap()
                .native(self.asset_bank.data())
        }
        fn liqor_liab_pos(&mut self) -> I80F48 {
            self.liqor
                .token_position(1)
                .unwrap()
                .native(self.liab_bank.data())
        }
    }

    #[test]
    fn test_token_conditional_swap_trigger() {
        let mut setup = TestSetup::new();

        let tcs = TokenConditionalSwap {
            max_buy: 100,
            max_sell: 100,
            price_threshold: 1.0,
            price_limit: 3.0,
            price_premium_bps: 1000,
            buy_token_index: 1,
            sell_token_index: 0,
            is_active: 1,
            allow_creating_borrows: 1,
            allow_creating_deposits: 1,
            ..Default::default()
        };
        *setup.liqee.add_token_conditional_swap().unwrap() = tcs.clone();
        assert_eq!(setup.liqee.active_token_conditional_swap().count(), 1);

        let trigger = |setup: &mut TestSetup, buy_price, buy_max, sell_price, sell_max| {
            action(
                &mut setup.liqor.borrow_mut(),
                Pubkey::default(),
                &mut setup.liqee.borrow_mut(),
                Pubkey::default(),
                0,
                setup.liab_bank.data(),
                I80F48::from_num(buy_price),
                buy_max,
                setup.asset_bank.data(),
                I80F48::from_num(sell_price),
                sell_max,
                0,
            )
        };

        assert!(trigger(&mut setup, 0.99, 40, 1.0, 100,).is_err());
        assert!(trigger(&mut setup, 1.0, 40, 0.33, 100,).is_err());

        let (buy_change, sell_change) = trigger(&mut setup, 2.0, 40, 1.0, 100).unwrap();
        assert_eq!(buy_change.round(), 40);
        assert_eq!(sell_change.round(), -88);

        assert_eq!(setup.liqee.active_token_conditional_swap().count(), 1);
        let tcs = setup
            .liqee
            .token_conditional_swap_by_index(0)
            .unwrap()
            .clone();
        assert_eq!(tcs.bought, 40);
        assert_eq!(tcs.sold, 88);

        assert_eq!(setup.liqee_liab_pos().round(), 40);
        assert_eq!(setup.liqee_asset_pos().round(), -88);
        assert_eq!(setup.liqor_liab_pos().round(), -40);
        assert_eq!(setup.liqor_asset_pos().round(), 88);

        let (buy_change, sell_change) = trigger(&mut setup, 2.0, 40, 1.0, 100).unwrap();
        assert_eq!(buy_change.round(), 5);
        assert_eq!(sell_change.round(), -11);

        assert_eq!(setup.liqee.active_token_conditional_swap().count(), 0);

        assert_eq!(setup.liqee_liab_pos().round(), 45);
        assert_eq!(setup.liqee_asset_pos().round(), -99);
        assert_eq!(setup.liqor_liab_pos().round(), -45);
        assert_eq!(setup.liqor_asset_pos().round(), 99);
    }
}
