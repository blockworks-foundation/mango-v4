use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::i80f48::ClampToInt;
use crate::logs::{
    emit_stack, LoanOriginationFeeInstruction, TokenBalanceLog, TokenConditionalSwapCancelLog,
    TokenConditionalSwapTriggerLogV3, WithdrawLoanLog,
};
use crate::state::*;

/// If init health is reduced below this number, the tcs is considered done.
///
/// This avoids a situation where the same tcs can be triggered again and again
/// for small amounts every time the init health increases by small amounts.
const TCS_TRIGGER_INIT_HEALTH_THRESHOLD: u64 = 1_000_000;

#[allow(clippy::too_many_arguments)]
pub fn token_conditional_swap_trigger(
    ctx: Context<TokenConditionalSwapTrigger>,
    token_conditional_swap_index: usize,
    token_conditional_swap_id: u64,
    max_buy_token_to_liqee: u64,
    max_sell_token_to_liqor: u64,
    min_buy_token: u64,
    min_taker_price: f64,
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

    let tcs = liqee.token_conditional_swap_by_index(token_conditional_swap_index)?;
    require!(tcs.is_configured(), MangoError::TokenConditionalSwapNotSet);
    require_eq!(
        tcs.id,
        token_conditional_swap_id,
        MangoError::TokenConditionalSwapIndexIdMismatch
    );
    let buy_token_index = tcs.buy_token_index;
    let sell_token_index = tcs.sell_token_index;
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    let tcs_is_expired = tcs.is_expired(now_ts);

    // Possibly wipe the tcs and exit, if it's already expired
    if tcs_is_expired {
        require!(min_buy_token == 0, MangoError::TokenConditionalSwapExpired);

        let (buy_bank, _buy_token_price, sell_bank_and_oracle_opt) =
            account_retriever.banks_mut_and_oracles(buy_token_index, sell_token_index)?;
        let (sell_bank, _sell_token_price) = sell_bank_and_oracle_opt.unwrap();

        let tcs = liqee.token_conditional_swap_mut_by_index(token_conditional_swap_index)?;
        *tcs = TokenConditionalSwap::default();

        // Release the hold on token positions and potentially close them
        liqee.token_decrement_dust_deactivate(buy_bank, now_ts, liqee_key)?;
        liqee.token_decrement_dust_deactivate(sell_bank, now_ts, liqee_key)?;

        msg!("TokenConditionalSwap is expired, removing");
        emit_stack(TokenConditionalSwapCancelLog {
            mango_group: ctx.accounts.group.key(),
            mango_account: ctx.accounts.liqee.key(),
            id: token_conditional_swap_id,
        });

        return Ok(());
    }

    // As a precaution, ensure that the liqee (and its health cache) will have an entry for both tokens:
    // we will want to adjust their values later. This is already guaranteed by the in_use_count
    // changes when the tcs was created.
    liqee.ensure_token_position(buy_token_index)?;
    liqee.ensure_token_position(sell_token_index)?;
    let mut liqee_health_cache = new_health_cache(&liqee.borrow(), &account_retriever, now_ts)
        .context("create liqee health cache")?;

    let (buy_bank, buy_token_price, sell_bank_and_oracle_opt) =
        account_retriever.banks_mut_and_oracles(buy_token_index, sell_token_index)?;
    let (sell_bank, sell_token_price) = sell_bank_and_oracle_opt.unwrap();

    let (liqee_buy_change, liqee_sell_change) = action(
        &mut liqor.borrow_mut(),
        liqor_key,
        &mut liqee.borrow_mut(),
        liqee_key,
        &mut liqee_health_cache,
        token_conditional_swap_index,
        buy_bank,
        buy_token_price,
        max_buy_token_to_liqee,
        sell_bank,
        sell_token_price,
        max_sell_token_to_liqor,
        now_ts,
        min_taker_price,
    )?;

    require_gte!(
        liqee_buy_change,
        min_buy_token,
        MangoError::TokenConditionalSwapMinBuyTokenNotReached
    );

    // Check liqor health, liqee health is checked inside (has to be, since tcs closure depends on it)
    let liqor_health = compute_health(
        &liqor.borrow(),
        HealthType::Init,
        &account_retriever,
        now_ts,
    )
    .context("compute liqor health")?;
    require!(liqor_health >= 0, MangoError::HealthMustBePositive);

    Ok(())
}

/// Figure out the trade amounts based on:
/// - the max requested
/// - remainder on the tcs
/// - allow_deposits / allow_borrows flags
/// - bank reduce only state
///
/// Returns (buy_amount, sell_amount)
fn trade_amount(
    tcs: &TokenConditionalSwap,
    sell_per_buy_price: I80F48,
    max_buy_token_to_liqee: u64,
    max_sell_token_to_liqor: u64,
    liqee_buy_balance: I80F48,
    liqee_sell_balance: I80F48,
    liqor_buy_balance: I80F48,
    liqor_sell_balance: I80F48,
    buy_bank: &Bank,
    sell_bank: &Bank,
) -> (u64, u64) {
    let max_buy = max_buy_token_to_liqee
        .min(tcs.max_buy_for_position(liqee_buy_balance, buy_bank))
        .min(if buy_bank.are_borrows_reduce_only() {
            // floor() so we never go below 0
            liqor_buy_balance.floor().clamp_to_u64()
        } else {
            u64::MAX
        });
    let max_sell = max_sell_token_to_liqor
        .min(tcs.max_sell_for_position(liqee_sell_balance, sell_bank))
        .min(if sell_bank.are_deposits_reduce_only() {
            // ceil() because we're ok reaching 0..1 deposited native tokens
            (-liqor_sell_balance).ceil().clamp_to_u64()
        } else {
            u64::MAX
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
    liqee_health_cache: &mut HealthCache,
    token_conditional_swap_index: usize,
    buy_bank: &mut Bank,
    buy_token_price: I80F48,
    max_buy_token_to_liqee: u64,
    sell_bank: &mut Bank,
    sell_token_price: I80F48,
    max_sell_token_to_liqor: u64,
    now_ts: u64,
    min_taker_price: f64,
) -> Result<(I80F48, I80F48)> {
    let liqee_pre_init_health = liqee.check_health_pre(&liqee_health_cache)?;

    // amount of sell token native per buy token native
    let sell_token_price_f64 = sell_token_price.to_num::<f64>();
    let price = buy_token_price.to_num::<f64>() / sell_token_price_f64;

    let tcs = {
        let tcs = liqee.token_conditional_swap_by_index(token_conditional_swap_index)?;
        require!(tcs.is_configured(), MangoError::TokenConditionalSwapNotSet);
        require!(
            !tcs.is_expired(now_ts),
            MangoError::TokenConditionalSwapExpired
        );
        require_eq!(buy_bank.token_index, tcs.buy_token_index);
        require_eq!(sell_bank.token_index, tcs.sell_token_index);

        tcs.check_triggerable(price, now_ts)?;

        // We need to borrow liqee token positions mutably and can't hold the tcs borrow at the
        // same time. Copying the whole struct is convenience.
        tcs.clone()
    };

    let premium_price = tcs.premium_price(price, now_ts);
    let maker_price = tcs.maker_price(premium_price);
    let maker_price_i80f48 = I80F48::from_num(maker_price);

    let taker_price = tcs.taker_price(premium_price);
    require_gte!(
        taker_price,
        min_taker_price,
        MangoError::TokenConditionalSwapTakerPriceTooLow
    );

    let pre_liqee_buy_token = liqee.token_position(tcs.buy_token_index)?.native(&buy_bank);
    let pre_liqee_sell_token = liqee
        .token_position(tcs.sell_token_index)?
        .native(&sell_bank);
    let pre_liqor_buy_token = liqor
        .ensure_token_position(tcs.buy_token_index)?
        .0
        .native(&buy_bank);
    let pre_liqor_sell_token = liqor
        .ensure_token_position(tcs.sell_token_index)?
        .0
        .native(&sell_bank);

    // derive trade amount based on limits in the tcs and by the liqor
    // the sell_token_amount_with_maker_fee is the amount to deduct from the liqee, it's adjusted upwards
    // for the maker fee (since this is included in the maker_price)
    let (buy_token_amount, sell_token_amount_with_maker_fee) = trade_amount(
        &tcs,
        maker_price_i80f48,
        max_buy_token_to_liqee,
        max_sell_token_to_liqor,
        pre_liqee_buy_token,
        pre_liqee_sell_token,
        pre_liqor_buy_token,
        pre_liqor_sell_token,
        buy_bank,
        sell_bank,
    );
    // NOTE: It's possible that buy_token_amount == sell_token_amount == 0!
    // Proceed with it anyway because we already mutated the account anyway and might want
    // to drop the token stop loss entry later.

    let sell_token_amount =
        (I80F48::from(buy_token_amount) * I80F48::from_num(premium_price)).floor();
    let sell_token_amount_u64 = sell_token_amount.to_num::<u64>();
    let maker_fee = sell_token_amount_with_maker_fee - sell_token_amount_u64;
    let taker_fee = tcs.taker_fee(sell_token_amount);

    let sell_token_amount_from_liqee = sell_token_amount_with_maker_fee;
    let sell_token_amount_to_liqor = sell_token_amount_u64 - taker_fee;

    // do the token transfer between liqee and liqor
    let buy_token_amount_i80f48 = I80F48::from(buy_token_amount);

    let (liqee_buy_token, liqee_buy_raw_index) = liqee.token_position_mut(tcs.buy_token_index)?;
    let (liqor_buy_token, liqor_buy_raw_index) = liqor.token_position_mut(tcs.buy_token_index)?;
    let buy_transfer = buy_bank.checked_transfer_with_fee(
        liqor_buy_token,
        buy_token_amount_i80f48,
        liqee_buy_token,
        buy_token_amount_i80f48,
        now_ts,
        buy_token_price,
    )?;
    let liqor_buy_active = buy_transfer.source_is_active;

    let post_liqee_buy_token = liqee_buy_token.native(&buy_bank);
    let post_liqor_buy_token = liqor_buy_token.native(&buy_bank);
    let liqee_buy_indexed_position = liqee_buy_token.indexed_position;
    let liqor_buy_indexed_position = liqor_buy_token.indexed_position;

    let (liqee_sell_token, liqee_sell_raw_index) =
        liqee.token_position_mut(tcs.sell_token_index)?;
    let (liqor_sell_token, liqor_sell_raw_index) =
        liqor.token_position_mut(tcs.sell_token_index)?;
    let sell_transfer = sell_bank.checked_transfer_with_fee(
        liqee_sell_token,
        I80F48::from(sell_token_amount_from_liqee),
        liqor_sell_token,
        I80F48::from(sell_token_amount_to_liqor),
        now_ts,
        sell_token_price,
    )?;
    let liqor_sell_active = sell_transfer.target_is_active;

    sell_bank.collected_fees_native += I80F48::from(maker_fee + taker_fee);

    let post_liqee_sell_token = liqee_sell_token.native(&sell_bank);
    let post_liqor_sell_token = liqor_sell_token.native(&sell_bank);
    let liqee_sell_indexed_position = liqee_sell_token.indexed_position;
    let liqor_sell_indexed_position = liqor_sell_token.indexed_position;

    // With a scanning account retriever, it's safe to deactivate inactive token positions immediately.
    // Liqee positions can only be deactivated if the tcs is closed (see below).
    if !liqor_buy_active {
        liqor.deactivate_token_position_and_log(liqor_buy_raw_index, liqor_key);
    }
    if !liqor_sell_active {
        liqor.deactivate_token_position_and_log(liqor_sell_raw_index, liqor_key)
    }

    // Log info

    // liqee buy token
    emit_stack(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqee_key,
        token_index: tcs.buy_token_index,
        indexed_position: liqee_buy_indexed_position.to_bits(),
        deposit_index: buy_bank.deposit_index.to_bits(),
        borrow_index: buy_bank.borrow_index.to_bits(),
    });
    // liqee sell token
    emit_stack(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqee_key,
        token_index: tcs.sell_token_index,
        indexed_position: liqee_sell_indexed_position.to_bits(),
        deposit_index: sell_bank.deposit_index.to_bits(),
        borrow_index: sell_bank.borrow_index.to_bits(),
    });
    // liqor buy token
    emit_stack(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqor_key,
        token_index: tcs.buy_token_index,
        indexed_position: liqor_buy_indexed_position.to_bits(),
        deposit_index: buy_bank.deposit_index.to_bits(),
        borrow_index: buy_bank.borrow_index.to_bits(),
    });
    // liqor sell token
    emit_stack(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqor_key,
        token_index: tcs.sell_token_index,
        indexed_position: liqor_sell_indexed_position.to_bits(),
        deposit_index: sell_bank.deposit_index.to_bits(),
        borrow_index: sell_bank.borrow_index.to_bits(),
    });

    if buy_transfer.has_loan() {
        emit_stack(WithdrawLoanLog {
            mango_group: liqee.fixed.group,
            mango_account: liqor_key,
            token_index: tcs.buy_token_index,
            loan_amount: buy_transfer.loan_amount.to_bits(),
            loan_origination_fee: buy_transfer.loan_origination_fee.to_bits(),
            instruction: LoanOriginationFeeInstruction::TokenConditionalSwapTrigger,
            price: Some(buy_token_price.to_bits()),
        });
    }
    if sell_transfer.has_loan() {
        emit_stack(WithdrawLoanLog {
            mango_group: liqee.fixed.group,
            mango_account: liqee_key,
            token_index: tcs.sell_token_index,
            loan_amount: sell_transfer.loan_amount.to_bits(),
            loan_origination_fee: sell_transfer.loan_origination_fee.to_bits(),
            instruction: LoanOriginationFeeInstruction::TokenConditionalSwapTrigger,
            price: Some(sell_token_price.to_bits()),
        });
    }

    // Check liqee health after the transaction
    // using sell_token_amount_i80f48 here would not account for loan origination fees!
    let liqee_buy_change = buy_token_amount_i80f48;
    let liqee_sell_change = post_liqee_sell_token - pre_liqee_sell_token;
    liqee_health_cache.adjust_token_balance(&buy_bank, liqee_buy_change)?;
    liqee_health_cache.adjust_token_balance(&sell_bank, liqee_sell_change)?;

    let liqee_post_init_health =
        liqee.check_health_post(&liqee_health_cache, liqee_pre_init_health)?;

    // update tcs information on the account
    let closed = {
        // record amount
        let tcs = liqee.token_conditional_swap_mut_by_index(token_conditional_swap_index)?;
        tcs.bought += buy_token_amount;
        tcs.sold += sell_token_amount_from_liqee;
        assert!(tcs.bought <= tcs.max_buy);
        assert!(tcs.sold <= tcs.max_sell);

        if !tcs.passed_start(now_ts) {
            tcs.start_timestamp = now_ts;
        }

        // Maybe remove token stop loss entry
        //
        // This drops the tcs if no more swapping is possible at the current price:
        // - if bought/sold reached the max
        // - if the "don't create deposits/borrows" constraint is reached
        // - if the price is such that swapping 1 native token would already exceed the buy/sell limit
        // - if the liqee health is so low that we believe the triggerer attempted to
        //   trigger as much as was possible given the liqee's account health

        let (future_buy, future_sell) = trade_amount(
            tcs,
            maker_price_i80f48,
            u64::MAX,
            u64::MAX,
            post_liqee_buy_token,
            post_liqee_sell_token,
            I80F48::MAX, // other liqors might not have reduce-only related restrictions
            I80F48::MIN,
            buy_bank,
            sell_bank,
        );

        // It's impossible to fulfill most requests exactly: You cannot buy 1 native SOL for 1 native USDC
        // because 1 native-USDC = 50 native-SOL.
        // Compute the smallest possible trade amount and close the tcs if it's close enough to it.
        let max_trade_reached;
        if maker_price > 1.0 {
            // 1 native buy token converts to >1 native sell tokens

            // Example: sell SOL, buy USDC, maker_price = 50 natSOL/natUSDC; if future_sell < 50, we can't
            // possibly buy another native USDC for it.
            max_trade_reached = future_sell < 2 * (maker_price as u64);
        } else {
            let buy_per_sell_price = 1.0 / maker_price;

            // Example: sell USDC, buy SOL, maker_price = 0.02 natUSDC/natSOL; if future_buy < 50, selling
            // even a single native USDC would overshoot it
            max_trade_reached = future_buy < 2 * (buy_per_sell_price as u64);
        }

        // If the health went down and is low enough, close the trigger. Otherwise it'd trigger repeatedly
        // as oracle prices fluctuate.
        let liqee_health_is_low = liqee_post_init_health < liqee_pre_init_health
            && liqee_post_init_health < TCS_TRIGGER_INIT_HEALTH_THRESHOLD;

        if future_buy == 0 || future_sell == 0 || liqee_health_is_low || max_trade_reached {
            *tcs = TokenConditionalSwap::default();
            true
        } else {
            false
        }
    };

    if closed {
        // Free up token position locks, maybe dusting and deactivating them
        liqee.token_decrement_dust_deactivate(buy_bank, now_ts, liqee_key)?;
        liqee.token_decrement_dust_deactivate(sell_bank, now_ts, liqee_key)?;
    }

    emit_stack(TokenConditionalSwapTriggerLogV3 {
        mango_group: liqee.fixed.group,
        liqee: liqee_key,
        liqor: liqor_key,
        token_conditional_swap_id: tcs.id,
        buy_token_index: tcs.buy_token_index,
        sell_token_index: tcs.sell_token_index,
        buy_amount: buy_token_amount,
        sell_amount: sell_token_amount_from_liqee,
        maker_fee,
        taker_fee,
        buy_token_price: buy_token_price.to_bits(),
        sell_token_price: sell_token_price.to_bits(),
        closed,
        display_price_style: tcs.display_price_style,
        intention: tcs.intention,
        tcs_type: tcs.tcs_type,
        start_timestamp: tcs.start_timestamp,
    });

    // Return the change in liqee token account balances
    Ok((liqee_buy_change, liqee_sell_change))
}

#[cfg(test)]
mod tests {
    use bytemuck::Zeroable;

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
                (1, 100, 100, 100, 1.0),
                (0.0, 0.0, true, true),
                (0.0, 0.0, 0, 0),
                (1, 1),
            ),
            (
                "limit 2",
                (100, 1, 100, 100, 1.0),
                (0.0, 0.0, true, true),
                (0.0, 0.0, 0, 0),
                (1, 1),
            ),
            (
                "limit 3",
                (100, 100, 1, 100, 1.0),
                (0.0, 0.0, true, true),
                (0.0, 0.0, 0, 0),
                (1, 1),
            ),
            (
                "limit 4",
                (100, 100, 100, 1, 1.0),
                (0.0, 0.0, true, true),
                (0.0, 0.0, 0, 0),
                (1, 1),
            ),
            (
                "limit 5",
                (100, 100, 100, 100, 1.0),
                (-0.3, 0.0, false, true),
                (0.0, 0.0, 0, 0),
                (1, 1),
            ),
            (
                "limit 6",
                (100, 100, 100, 100, 1.0),
                (0.0, 1.8, true, false),
                (0.0, 0.0, 0, 0),
                (1, 1),
            ),
            (
                "full 1",
                (100, 100, 100, 100, 1.0),
                (-100.0, 100.0, false, false),
                (0.0, 0.0, 0, 0),
                (100, 100),
            ),
            (
                "full 2",
                (100, 100, 100, 100, 1.0),
                (0.0, 0.0, true, true),
                (0.0, 0.0, 0, 0),
                (100, 100),
            ),
            (
                "reduce only buy 1",
                (100, 100, 100, 100, 1.0),
                (-10.0, 0.0, true, true),
                (20.0, 0.0, 1, 0),
                (10, 10),
            ),
            (
                "reduce only buy 2",
                (100, 100, 100, 100, 1.0),
                (-20.0, 0.0, true, true),
                (10.0, 0.0, 1, 0),
                (10, 10),
            ),
            (
                "reduce only buy 3",
                (100, 100, 100, 100, 1.0),
                (-10.0, 0.0, true, true),
                (20.0, 0.0, 2, 0),
                (20, 20),
            ),
            (
                "reduce only sell 1",
                (100, 100, 100, 100, 1.0),
                (0.0, 10.0, true, true),
                (0.0, -20.0, 0, 1),
                (10, 10),
            ),
            (
                "reduce only sell 2",
                (100, 100, 100, 100, 1.0),
                (0.0, 20.0, true, true),
                (0.0, -10.0, 0, 1),
                (10, 10),
            ),
            (
                "reduce only sell 3",
                (100, 100, 100, 100, 1.0),
                (0.0, 20.0, true, true),
                (0.0, -10.0, 0, 2),
                (20, 20),
            ),
            (
                "price 1",
                (100, 100, 100, 100, 1.23456),
                (0.0, 0.0, true, true),
                (0.0, 0.0, 0, 0),
                (81, 99),
            ),
            (
                "price 2",
                (100, 100, 100, 100, 0.76543),
                (0.0, 0.0, true, true),
                (0.0, 0.0, 0, 0),
                (100, 76),
            ),
        ];

        for (
            name,
            (tcs_buy, tcs_sell, liqor_buy, liqor_sell, price),
            (liqee_buy_balance, liqee_sell_balance, liqee_allow_deposit, liqee_allow_borrow),
            (liqor_buy_balance, liqor_sell_balance, buy_reduce_only, sell_reduce_only),
            (buy_amount, sell_amount),
        ) in cases
        {
            println!("{name}");

            let tcs = TokenConditionalSwap {
                max_buy: 42 + tcs_buy,
                max_sell: 100 + tcs_sell,
                bought: 42,
                sold: 100,
                allow_creating_borrows: u8::from(liqee_allow_borrow),
                allow_creating_deposits: u8::from(liqee_allow_deposit),
                ..Default::default()
            };

            let buy_bank = Bank {
                reduce_only: buy_reduce_only,
                ..Bank::zeroed()
            };

            let sell_bank = Bank {
                reduce_only: sell_reduce_only,
                ..Bank::zeroed()
            };

            let (actual_buy, actual_sell) = trade_amount(
                &tcs,
                I80F48::from_num(price),
                liqor_buy,
                liqor_sell,
                I80F48::from_num(liqee_buy_balance),
                I80F48::from_num(liqee_sell_balance),
                I80F48::from_num(liqor_buy_balance),
                I80F48::from_num(liqor_sell_balance),
                &buy_bank,
                &sell_bank,
            );
            println!("actual: {actual_buy} {actual_sell}, expected: {buy_amount}, {sell_amount}");
            assert_eq!(actual_buy, buy_amount);
            assert_eq!(actual_sell, (actual_buy as f64 * price).floor() as u64); // invariant
            assert_eq!(actual_sell, sell_amount);
        }
    }

    #[derive(Clone)]
    struct TestSetup {
        group: Pubkey,
        asset_bank: TestAccount<Bank>,
        liab_bank: TestAccount<Bank>,
        asset_oracle: TestAccount<StubOracle>,
        liab_oracle: TestAccount<StubOracle>,
        liqee: MangoAccountValue,
        liqor: MangoAccountValue,
    }

    impl TestSetup {
        fn new() -> Self {
            let group = Pubkey::new_unique();
            let (asset_bank, asset_oracle) = mock_bank_and_oracle(group, 0, 1.0, 0.0, 0.0);
            let (liab_bank, liab_oracle) = mock_bank_and_oracle(group, 1, 1.0, 0.0, 0.0);

            let mut liqee_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            liqee_buffer.extend_from_slice(&[0u8; 512]);
            let mut liqee = MangoAccountValue::from_bytes(&liqee_buffer).unwrap();
            {
                liqee.resize_dynamic_content(3, 5, 4, 6, 1).unwrap();
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
                group,
                asset_bank,
                liab_bank,
                asset_oracle,
                liab_oracle,
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

        fn trigger(
            &mut self,
            buy_price: f64,
            buy_max: u64,
            sell_price: f64,
            sell_max: u64,
        ) -> Result<(I80F48, I80F48)> {
            let mut setup = self.clone();

            let ais = vec![
                setup.asset_bank.as_account_info(),
                setup.liab_bank.as_account_info(),
                setup.asset_oracle.as_account_info(),
                setup.liab_oracle.as_account_info(),
            ];
            let retriever =
                ScanningAccountRetriever::new_with_staleness(&ais, &setup.group, None).unwrap();
            let mut liqee_health_cache =
                crate::health::new_health_cache(&setup.liqee.borrow(), &retriever, 0).unwrap();

            action(
                &mut self.liqor.borrow_mut(),
                Pubkey::default(),
                &mut self.liqee.borrow_mut(),
                Pubkey::default(),
                &mut liqee_health_cache,
                0,
                self.liab_bank.data(),
                I80F48::from_num(buy_price),
                buy_max,
                self.asset_bank.data(),
                I80F48::from_num(sell_price),
                sell_max,
                0,
                0.0,
            )
        }
    }

    #[test]
    fn test_token_conditional_swap_trigger() {
        let mut setup = TestSetup::new();

        let asset_pos = 100_000_000;

        setup
            .asset_bank
            .data()
            .deposit(
                &mut setup.liqee.token_position_mut(0).unwrap().0,
                I80F48::from(asset_pos),
                0,
            )
            .unwrap();

        let tcs = TokenConditionalSwap {
            max_buy: 100,
            max_sell: 100,
            price_lower_limit: 1.0,
            price_upper_limit: 3.0,
            price_premium_rate: 0.11,
            buy_token_index: 1,
            sell_token_index: 0,
            is_configured: 1,
            allow_creating_borrows: 1,
            allow_creating_deposits: 1,
            ..Default::default()
        };
        *setup.liqee.free_token_conditional_swap_mut().unwrap() = tcs.clone();
        assert_eq!(setup.liqee.active_token_conditional_swaps().count(), 1);

        assert!(setup.trigger(0.99, 40, 1.0, 100,).is_err());
        assert!(setup.trigger(1.0, 40, 0.33, 100,).is_err());

        let (buy_change, sell_change) = setup.trigger(2.0, 40, 1.0, 100).unwrap();
        assert_eq!(buy_change.round(), 40);
        assert_eq!(sell_change.round(), -88);

        assert_eq!(setup.liqee.active_token_conditional_swaps().count(), 1);
        let tcs = setup
            .liqee
            .token_conditional_swap_by_index(0)
            .unwrap()
            .clone();
        assert_eq!(tcs.bought, 40);
        assert_eq!(tcs.sold, 88);

        assert_eq!(setup.liqee_liab_pos().round(), 40);
        assert_eq!(setup.liqee_asset_pos().round(), asset_pos - 88);
        assert_eq!(setup.liqor_liab_pos().round(), -40);
        assert_eq!(setup.liqor_asset_pos().round(), 88);

        let (buy_change, sell_change) = setup.trigger(2.0, 40, 1.0, 100).unwrap();
        assert_eq!(buy_change.round(), 5);
        assert_eq!(sell_change.round(), -11);

        assert_eq!(setup.liqee.active_token_conditional_swaps().count(), 0);

        assert_eq!(setup.liqee_liab_pos().round(), 45);
        assert_eq!(setup.liqee_asset_pos().round(), asset_pos - 99);
        assert_eq!(setup.liqor_liab_pos().round(), -45);
        assert_eq!(setup.liqor_asset_pos().round(), 99);
    }

    #[test]
    fn test_token_conditional_swap_low_health_close() {
        let mut setup = TestSetup::new();

        setup
            .asset_bank
            .data()
            .deposit(
                &mut setup.liqee.token_position_mut(0).unwrap().0,
                I80F48::from(100_000_000),
                0,
            )
            .unwrap();

        let tcs = TokenConditionalSwap {
            max_buy: 10_000_000_000,
            max_sell: 10_000_000_000,
            price_lower_limit: 1.0,
            price_upper_limit: 3.0,
            price_premium_rate: 0.0,
            buy_token_index: 1,
            sell_token_index: 0,
            is_configured: 1,
            allow_creating_borrows: 1,
            allow_creating_deposits: 1,
            ..Default::default()
        };
        *setup.liqee.free_token_conditional_swap_mut().unwrap() = tcs.clone();

        let (buy_change, sell_change) = setup.trigger(2.0, 50_000_000, 1.0, 50_000_000).unwrap();
        assert_eq!(buy_change.round(), 25_000_000);
        assert_eq!(sell_change.round(), -50_000_000);

        // Not closed yet, health still good
        assert_eq!(setup.liqee.active_token_conditional_swaps().count(), 1);

        let (buy_change, sell_change) = setup.trigger(2.0, 150_000_000, 1.0, 150_000_000).unwrap();
        assert_eq!(buy_change.round(), 75_000_000);
        assert_eq!(sell_change.round(), -150_000_000);

        // Health is 0
        assert_eq!(setup.liqee.active_token_conditional_swaps().count(), 0);

        assert_eq!(setup.liqee_liab_pos().round(), 100_000_000);
        assert_eq!(setup.liqee_asset_pos().round(), -100_000_000);
    }

    #[test]
    fn test_token_conditional_swap_trigger_fees() {
        let mut setup = TestSetup::new();

        let asset_pos = 100_000_000;
        setup
            .asset_bank
            .data()
            .deposit(
                &mut setup.liqee.token_position_mut(0).unwrap().0,
                I80F48::from(asset_pos),
                0,
            )
            .unwrap();

        let tcs = TokenConditionalSwap {
            max_buy: 1000,
            max_sell: 1000,
            price_lower_limit: 1.0,
            price_upper_limit: 3.0,
            price_premium_rate: 0.02,
            maker_fee_rate: 0.03,
            taker_fee_rate: 0.05,
            buy_token_index: 1,
            sell_token_index: 0,
            is_configured: 1,
            allow_creating_borrows: 1,
            allow_creating_deposits: 1,
            ..Default::default()
        };
        *setup.liqee.free_token_conditional_swap_mut().unwrap() = tcs.clone();
        assert_eq!(setup.liqee.active_token_conditional_swaps().count(), 1);

        let (buy_change, sell_change) = setup.trigger(1.0, 1000, 1.0, 1000).unwrap();
        assert_eq!(buy_change.round(), 952);
        assert_eq!(sell_change.round(), -1000);

        assert_eq!(setup.liqee_liab_pos().round(), 952);
        assert_eq!(setup.liqee_asset_pos().round(), asset_pos - 1000);
        assert_eq!(setup.liqor_liab_pos().round(), -952);
        assert_eq!(setup.liqor_asset_pos().round(), 923); // floor(952*1.02*0.95)

        assert_eq!(setup.asset_bank.data().collected_fees_native, 77);
    }
}
