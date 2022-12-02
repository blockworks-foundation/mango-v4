#![cfg(feature = "client")]

use anchor_lang::prelude::*;

use fixed::types::I80F48;
use fixed_macro::types::I80F48;

use crate::error::*;
use crate::state::{Bank, MangoAccountValue, PerpMarketIndex};
use crate::util::checked_math as cm;

use super::*;
use crate::state::orderbook::Side as PerpOrderSide;

impl HealthCache {
    pub fn can_call_spot_bankruptcy(&self) -> bool {
        !self.has_liquidatable_assets() && self.has_spot_borrows()
    }

    pub fn is_liquidatable(&self) -> bool {
        if self.being_liquidated {
            self.health(HealthType::Init).is_negative()
        } else {
            self.health(HealthType::Maint).is_negative()
        }
    }

    /// The health ratio is
    /// - 0 if health is 0 - meaning assets = liabs
    /// - 100 if there's 2x as many assets as liabs
    /// - 200 if there's 3x as many assets as liabs
    /// - MAX if liabs = 0
    ///
    /// Maybe talking about the collateralization ratio assets/liabs is more intuitive?
    pub fn health_ratio(&self, health_type: HealthType) -> I80F48 {
        let (assets, liabs) = self.health_assets_and_liabs(health_type);
        let hundred = I80F48::from(100);
        if liabs > 0 {
            // feel free to saturate to MAX for tiny liabs
            cm!(hundred * (assets - liabs)).saturating_div(liabs)
        } else {
            I80F48::MAX
        }
    }

    fn cache_after_swap(
        &self,
        account: &MangoAccountValue,
        source_bank: &Bank,
        target_bank: &Bank,
        amount: I80F48,
        price: I80F48,
    ) -> Self {
        let mut source_position = account
            .token_position(source_bank.token_index)
            .map(|v| v.clone())
            .unwrap_or_default();
        let mut target_position = account
            .token_position(target_bank.token_index)
            .map(|v| v.clone())
            .unwrap_or_default();

        let target_amount = cm!(amount * price);

        let mut source_bank = source_bank.clone();
        source_bank
            .withdraw_with_fee(&mut source_position, amount, 0, I80F48::ZERO)
            .unwrap();
        let mut target_bank = target_bank.clone();
        target_bank
            .deposit(&mut target_position, target_amount, 0)
            .unwrap();

        let mut resulting_cache = self.clone();
        resulting_cache
            .adjust_token_balance(&source_bank, -amount)
            .unwrap();
        resulting_cache
            .adjust_token_balance(&target_bank, target_amount)
            .unwrap();
        resulting_cache
    }

    /// How much source native tokens may be swapped for target tokens while staying
    /// above the min_ratio health ratio.
    ///
    /// `price`: The amount of target native you receive for one source native. So if we
    /// swap BTC -> SOL and they're at ui prices of $20000 and $40, that means price
    /// should be 500000 native_SOL for a native_BTC. Because 1 BTC gives you 500 SOL
    /// so 1e6 native_BTC gives you 500e9 native_SOL.
    ///
    /// NOTE: keep getMaxSourceForTokenSwap in ts/client in sync with changes here
    pub fn max_swap_source_for_health_ratio(
        &self,
        account: &MangoAccountValue,
        source_bank: &Bank,
        target_bank: &Bank,
        price: I80F48,
        min_ratio: I80F48,
    ) -> Result<I80F48> {
        // The health_ratio is nonlinear based on swap amount.
        // For large swap amounts the slope is guaranteed to be negative (unless the price
        // is extremely good), but small amounts can have positive slope (e.g. using
        // source deposits to pay back target borrows).
        //
        // That means:
        // - even if the initial ratio is < min_ratio it can be useful to swap to *increase* health
        // - be careful about finding the min_ratio point: the function isn't convex

        let health_type = HealthType::Init;
        let initial_ratio = self.health_ratio(health_type);
        if initial_ratio < 0 {
            return Ok(I80F48::ZERO);
        }

        let source_index = find_token_info_index(&self.token_infos, source_bank.token_index)?;
        let target_index = find_token_info_index(&self.token_infos, target_bank.token_index)?;
        let source = &self.token_infos[source_index];
        let target = &self.token_infos[target_index];

        // If the price is sufficiently good, then health will just increase from swapping:
        // once we've swapped enough, swapping x reduces health by x * source_liab_weight and
        // increases it by x * target_asset_weight * price_factor.
        // This is just the highest final slope we can get. If the health weights are
        // scaled because the collateral or borrow limits are exceeded, health will decrease
        // more quickly than this number.
        let final_health_slope = -source.init_liab_weight * source.prices.liab(health_type)
            + target.init_asset_weight * target.prices.asset(health_type) * price;
        if final_health_slope >= 0 {
            return Ok(I80F48::MAX);
        }

        let cache_after_swap = |amount: I80F48| {
            self.cache_after_swap(account, source_bank, target_bank, amount, price)
        };
        let health_ratio_after_swap =
            |amount| cache_after_swap(amount).health_ratio(HealthType::Init);

        // There are two key slope changes: Assume source.balance > 0 and target.balance < 0.
        // When these values flip sign, the health slope decreases, but could still be positive.
        // After point1 it's definitely negative (due to final_health_slope check above).
        // The maximum health ratio will be at 0 or at one of these points (ignoring serum3 effects).
        let source_for_zero_target_balance = -target.balance_native / price;
        let point0_amount = source
            .balance_native
            .min(source_for_zero_target_balance)
            .max(I80F48::ZERO);
        let point1_amount = source
            .balance_native
            .max(source_for_zero_target_balance)
            .max(I80F48::ZERO);
        let point0_ratio = health_ratio_after_swap(point0_amount);
        let (point1_ratio, point1_health) = {
            let cache = cache_after_swap(point1_amount);
            (
                cache.health_ratio(HealthType::Init),
                cache.health(HealthType::Init),
            )
        };

        let amount =
            if initial_ratio <= min_ratio && point0_ratio < min_ratio && point1_ratio < min_ratio {
                // If we have to stay below the target ratio, pick the highest one
                if point0_ratio > initial_ratio {
                    if point1_ratio > point0_ratio {
                        point1_amount
                    } else {
                        point0_amount
                    }
                } else if point1_ratio > initial_ratio {
                    point1_amount
                } else {
                    I80F48::ZERO
                }
            } else if point1_ratio >= min_ratio {
                // If point1_ratio is still bigger than min_ratio, the target amount must be >point1_amount
                // search to the right of point1_amount: but how far?
                // At point1, source.balance < 0 and target.balance > 0, so use a simple estimation for
                // zero health: health
                //              - source_liab_weight * source_liab_price * a
                //              + target_asset_weight * target_asset_price * price * a = 0.
                // where a is the source token native amount.
                if point1_health <= 0 {
                    return Ok(I80F48::ZERO);
                }
                let zero_health_amount = point1_amount - point1_health / final_health_slope;
                let zero_health_ratio = health_ratio_after_swap(zero_health_amount);
                binary_search(
                    point1_amount,
                    point1_ratio,
                    zero_health_amount,
                    zero_health_ratio,
                    min_ratio,
                    I80F48::ZERO,
                    health_ratio_after_swap,
                )?
            } else if point0_ratio >= min_ratio {
                // Must be between point0_amount and point1_amount.
                binary_search(
                    point0_amount,
                    point0_ratio,
                    point1_amount,
                    point1_ratio,
                    min_ratio,
                    I80F48::ZERO,
                    health_ratio_after_swap,
                )?
            } else {
                // Must be between 0 and point0_amount
                binary_search(
                    I80F48::ZERO,
                    initial_ratio,
                    point0_amount,
                    point0_ratio,
                    min_ratio,
                    I80F48::ZERO,
                    health_ratio_after_swap,
                )?
            };

        Ok(amount)
    }

    fn perp_info_index(&self, perp_market_index: PerpMarketIndex) -> Result<usize> {
        self.perp_infos
            .iter()
            .position(|pi| pi.perp_market_index == perp_market_index)
            .ok_or_else(|| error_msg!("perp market index {} not found", perp_market_index))
    }

    /// NOTE: keep getMaxSourceForTokenSwap in ts/client in sync with changes here
    pub fn max_perp_for_health_ratio(
        &self,
        perp_market_index: PerpMarketIndex,
        price: I80F48,
        side: PerpOrderSide,
        min_ratio: I80F48,
    ) -> Result<i64> {
        let health_type = HealthType::Init;
        let initial_ratio = self.health_ratio(health_type);
        if initial_ratio < 0 {
            return Ok(0);
        }

        let direction = match side {
            PerpOrderSide::Bid => 1,
            PerpOrderSide::Ask => -1,
        };

        let perp_info_index = self.perp_info_index(perp_market_index)?;
        let perp_info = &self.perp_infos[perp_info_index];
        let prices = &perp_info.prices;
        let base_lot_size = I80F48::from(perp_info.base_lot_size);

        // If the price is sufficiently good then health will just increase from trading
        // TODO: This is not actually correct, since perp health for untrusted markets can't go above 0
        let final_health_slope = if direction == 1 {
            perp_info.init_asset_weight * prices.asset(health_type) - price
        } else {
            price - perp_info.init_liab_weight * prices.liab(health_type)
        };
        if final_health_slope >= 0 {
            return Ok(i64::MAX);
        }

        let cache_after_trade = |base_lots: i64| {
            let mut adjusted_cache = self.clone();
            adjusted_cache.perp_infos[perp_info_index].base_lots += direction * base_lots;
            adjusted_cache.perp_infos[perp_info_index].quote -=
                I80F48::from(direction) * I80F48::from(base_lots) * base_lot_size * price;
            adjusted_cache
        };
        let health_ratio_after_trade =
            |base_lots: i64| cache_after_trade(base_lots).health_ratio(HealthType::Init);
        let health_ratio_after_trade_trunc =
            |base_lots: I80F48| health_ratio_after_trade(base_lots.round_to_zero().to_num());

        let initial_base_lots = perp_info.base_lots;

        // There are two cases:
        // 1. We are increasing abs(base_lots)
        // 2. We are bringing the base position to 0, and then going to case 1.
        let has_case2 =
            initial_base_lots > 0 && direction == -1 || initial_base_lots < 0 && direction == 1;

        let (case1_start, case1_start_ratio) = if has_case2 {
            let case1_start = initial_base_lots.abs();
            let case1_start_ratio = health_ratio_after_trade(case1_start);
            (case1_start, case1_start_ratio)
        } else {
            (0, initial_ratio)
        };
        let case1_start_i80f48 = I80F48::from(case1_start);

        // If we start out below min_ratio and can't go above, pick the best case
        let base_lots = if initial_ratio <= min_ratio && case1_start_ratio < min_ratio {
            if case1_start_ratio >= initial_ratio {
                case1_start_i80f48
            } else {
                I80F48::ZERO
            }
        } else if case1_start_ratio >= min_ratio {
            // Must reach min_ratio to the right of case1_start

            // Need to figure out how many lots to trade to reach zero health (zero_health_amount).
            // We do this by looking at the starting health and the health slope per
            // traded base lot (final_health_slope).
            let start_cache = cache_after_trade(case1_start);
            let start_health = start_cache.health(HealthType::Init);
            if start_health <= 0 {
                return Ok(0);
            }

            // The perp market's contribution to the health above may be capped. But we need to trade
            // enough to fully reduce any positive-pnl buffer. Thus get the uncapped health:
            let perp_info = &start_cache.perp_infos[perp_info_index];
            let start_health_uncapped = start_health
                - perp_info.health_contribution(HealthType::Init)
                + perp_info.uncapped_health_contribution(HealthType::Init);

            // We add 1 here because health is computed for truncated base_lots and we want to guarantee
            // zero_health_ratio <= 0.
            let zero_health_amount = case1_start_i80f48
                - start_health_uncapped / final_health_slope / base_lot_size
                + I80F48::ONE;
            let zero_health_ratio = health_ratio_after_trade_trunc(zero_health_amount);
            assert!(zero_health_ratio <= 0);

            binary_search(
                case1_start_i80f48,
                case1_start_ratio,
                zero_health_amount,
                zero_health_ratio,
                min_ratio,
                I80F48::ONE,
                health_ratio_after_trade_trunc,
            )?
        } else {
            // Between 0 and case1_start
            binary_search(
                I80F48::ZERO,
                initial_ratio,
                case1_start_i80f48,
                case1_start_ratio,
                min_ratio,
                I80F48::ONE,
                health_ratio_after_trade_trunc,
            )?
        };

        Ok(base_lots.round_to_zero().to_num())
    }
}

fn binary_search(
    mut left: I80F48,
    left_value: I80F48,
    mut right: I80F48,
    right_value: I80F48,
    target_value: I80F48,
    min_step: I80F48,
    fun: impl Fn(I80F48) -> I80F48,
) -> Result<I80F48> {
    let max_iterations = 20;
    let target_error = I80F48!(0.1);
    require_msg!(
        (left_value - target_value).signum() * (right_value - target_value).signum() != I80F48::ONE,
        "internal error: left {} and right {} don't contain the target value {}",
        left_value,
        right_value,
        target_value
    );
    for _ in 0..max_iterations {
        if (right - left).abs() < min_step {
            return Ok(left);
        }
        let new = I80F48::from_num(0.5) * (left + right);
        let new_value = fun(new);
        let error = new_value - target_value;
        if error > 0 && error < target_error {
            return Ok(new);
        }

        if (new_value > target_value) ^ (right_value > target_value) {
            left = new;
        } else {
            right = new;
        }
    }
    Err(error_msg!("binary search iterations exhausted"))
}

#[cfg(test)]
mod tests {
    use super::super::test::*;
    use super::*;
    use crate::state::*;
    use serum_dex::state::OpenOrders;

    fn health_eq(a: I80F48, b: f64) -> bool {
        if (a - I80F48::from_num(b)).abs() < 0.001 {
            true
        } else {
            println!("health is {}, but expected {}", a, b);
            false
        }
    }

    #[test]
    fn test_max_swap() {
        let default_token_info = |x| TokenInfo {
            token_index: 0,
            maint_asset_weight: I80F48::from_num(1.0 - x),
            init_asset_weight: I80F48::from_num(1.0 - x),
            maint_liab_weight: I80F48::from_num(1.0 + x),
            init_liab_weight: I80F48::from_num(1.0 + x),
            prices: Prices::new_single_price(I80F48::from_num(2.0)),
            balance_native: I80F48::ZERO,
        };

        let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let account = MangoAccountValue::from_bytes(&buffer).unwrap();

        let group = Pubkey::new_unique();
        let (mut bank0, _) = mock_bank_and_oracle(group, 0, 1.0, 0.1, 0.1);
        let (mut bank1, _) = mock_bank_and_oracle(group, 1, 5.0, 0.2, 0.2);
        let (mut bank2, _) = mock_bank_and_oracle(group, 2, 5.0, 0.3, 0.3);
        let banks = [bank0.data(), bank1.data(), bank2.data()];

        let health_cache = HealthCache {
            token_infos: vec![
                TokenInfo {
                    token_index: 0,
                    prices: Prices::new_single_price(I80F48::from_num(2.0)),
                    ..default_token_info(0.1)
                },
                TokenInfo {
                    token_index: 1,
                    prices: Prices::new_single_price(I80F48::from_num(3.0)),
                    ..default_token_info(0.2)
                },
                TokenInfo {
                    token_index: 2,
                    prices: Prices::new_single_price(I80F48::from_num(4.0)),
                    ..default_token_info(0.3)
                },
            ],
            serum3_infos: vec![],
            perp_infos: vec![],
            being_liquidated: false,
        };

        assert_eq!(health_cache.health(HealthType::Init), I80F48::ZERO);
        assert_eq!(health_cache.health_ratio(HealthType::Init), I80F48::MAX);
        assert_eq!(
            health_cache
                .max_swap_source_for_health_ratio(
                    &account,
                    banks[0],
                    banks[1],
                    I80F48::from_num(2.0 / 3.0),
                    I80F48::from_num(50.0)
                )
                .unwrap(),
            I80F48::ZERO
        );

        let adjust_by_usdc = |c: &mut HealthCache, ti: TokenIndex, usdc: f64| {
            let ti = &mut c.token_infos[ti as usize];
            ti.balance_native += I80F48::from_num(usdc) / ti.prices.oracle;
        };
        let find_max_swap_actual = |c: &HealthCache,
                                    source: TokenIndex,
                                    target: TokenIndex,
                                    ratio: f64,
                                    price_factor: f64| {
            let source_price = &c.token_infos[source as usize].prices;
            let source_bank = &banks[source as usize];
            let target_price = &c.token_infos[target as usize].prices;
            let target_bank = &banks[target as usize];
            let swap_price =
                I80F48::from_num(price_factor) * source_price.oracle / target_price.oracle;
            let source_amount = c
                .max_swap_source_for_health_ratio(
                    &account,
                    source_bank,
                    target_bank,
                    swap_price,
                    I80F48::from_num(ratio),
                )
                .unwrap();
            if source_amount == I80F48::MAX {
                return (f64::MAX, f64::MAX);
            }
            let after_swap = c.cache_after_swap(
                &account,
                source_bank,
                target_bank,
                source_amount,
                swap_price,
            );
            (
                source_amount.to_num::<f64>(),
                after_swap.health_ratio(HealthType::Init).to_num::<f64>(),
            )
        };
        let check_max_swap_result = |c: &HealthCache,
                                     source: TokenIndex,
                                     target: TokenIndex,
                                     ratio: f64,
                                     price_factor: f64| {
            let (source_amount, actual_ratio) =
                find_max_swap_actual(c, source, target, ratio, price_factor);
            println!(
                    "checking {source} to {target} for price_factor: {price_factor}, target ratio {ratio}: actual ratio: {actual_ratio}, amount: {source_amount}",
                );
            assert!((ratio - actual_ratio).abs() < 1.0);
        };

        {
            println!("test 0");
            let mut health_cache = health_cache.clone();
            adjust_by_usdc(&mut health_cache, 1, 100.0);

            for price_factor in [0.1, 0.9, 1.1] {
                for target in 1..100 {
                    let target = target as f64;
                    check_max_swap_result(&health_cache, 0, 1, target, price_factor);
                    check_max_swap_result(&health_cache, 1, 0, target, price_factor);
                    check_max_swap_result(&health_cache, 0, 2, target, price_factor);
                }
            }

            // At this unlikely price it's healthy to swap infinitely
            assert_eq!(
                find_max_swap_actual(&health_cache, 0, 1, 50.0, 1.5).0,
                f64::MAX
            );
        }

        {
            println!("test 1");
            let mut health_cache = health_cache.clone();
            adjust_by_usdc(&mut health_cache, 0, -20.0);
            adjust_by_usdc(&mut health_cache, 1, 100.0);

            for price_factor in [0.1, 0.9, 1.1] {
                for target in 1..100 {
                    let target = target as f64;
                    check_max_swap_result(&health_cache, 0, 1, target, price_factor);
                    check_max_swap_result(&health_cache, 1, 0, target, price_factor);
                    check_max_swap_result(&health_cache, 0, 2, target, price_factor);
                    check_max_swap_result(&health_cache, 2, 0, target, price_factor);
                }
            }
        }

        {
            println!("test 2");
            let mut health_cache = health_cache.clone();
            adjust_by_usdc(&mut health_cache, 0, -50.0);
            adjust_by_usdc(&mut health_cache, 1, 100.0);
            // possible even though the init ratio is <100
            check_max_swap_result(&health_cache, 1, 0, 100.0, 1.0);
        }

        {
            println!("test 3");
            let mut health_cache = health_cache.clone();
            adjust_by_usdc(&mut health_cache, 0, -30.0);
            adjust_by_usdc(&mut health_cache, 1, 100.0);
            adjust_by_usdc(&mut health_cache, 2, -30.0);

            // swapping with a high ratio advises paying back all liabs
            // and then swapping even more because increasing assets in 0 has better asset weight
            let init_ratio = health_cache.health_ratio(HealthType::Init);
            let (amount, actual_ratio) = find_max_swap_actual(&health_cache, 1, 0, 100.0, 1.0);
            println!(
                "init {}, after {}, amount {}",
                init_ratio, actual_ratio, amount
            );
            assert!(actual_ratio / 2.0 > init_ratio);
            assert!((amount - 100.0 / 3.0).abs() < 1.0);
        }

        {
            println!("test 4");
            let mut health_cache = health_cache.clone();
            adjust_by_usdc(&mut health_cache, 0, 100.0);
            adjust_by_usdc(&mut health_cache, 1, -2.0);
            adjust_by_usdc(&mut health_cache, 2, -65.0);

            let init_ratio = health_cache.health_ratio(HealthType::Init);
            assert!(init_ratio > 3 && init_ratio < 4);

            check_max_swap_result(&health_cache, 0, 1, 1.0, 1.0);
            check_max_swap_result(&health_cache, 0, 1, 3.0, 1.0);
            check_max_swap_result(&health_cache, 0, 1, 4.0, 1.0);
        }
    }

    #[test]
    fn test_max_perp() {
        let default_token_info = |x| TokenInfo {
            token_index: 0,
            maint_asset_weight: I80F48::from_num(1.0 - x),
            init_asset_weight: I80F48::from_num(1.0 - x),
            maint_liab_weight: I80F48::from_num(1.0 + x),
            init_liab_weight: I80F48::from_num(1.0 + x),
            prices: Prices::new_single_price(I80F48::from_num(2.0)),
            balance_native: I80F48::ZERO,
        };
        let base_lot_size = 100;
        let default_perp_info = |x| PerpInfo {
            perp_market_index: 0,
            maint_asset_weight: I80F48::from_num(1.0 - x),
            init_asset_weight: I80F48::from_num(1.0 - x),
            maint_liab_weight: I80F48::from_num(1.0 + x),
            init_liab_weight: I80F48::from_num(1.0 + x),
            base_lot_size,
            base_lots: 0,
            bids_base_lots: 0,
            asks_base_lots: 0,
            quote: I80F48::ZERO,
            prices: Prices::new_single_price(I80F48::from_num(2.0)),
            has_open_orders: false,
            trusted_market: false,
        };

        let health_cache = HealthCache {
            token_infos: vec![TokenInfo {
                token_index: 0,
                prices: Prices::new_single_price(I80F48::from_num(1.0)),
                balance_native: I80F48::ZERO,
                ..default_token_info(0.0)
            }],
            serum3_infos: vec![],
            perp_infos: vec![PerpInfo {
                perp_market_index: 0,
                ..default_perp_info(0.3)
            }],
            being_liquidated: false,
        };

        assert_eq!(health_cache.health(HealthType::Init), I80F48::ZERO);
        assert_eq!(health_cache.health_ratio(HealthType::Init), I80F48::MAX);
        assert_eq!(
            health_cache
                .max_perp_for_health_ratio(
                    0,
                    I80F48::from(2),
                    PerpOrderSide::Bid,
                    I80F48::from_num(50.0)
                )
                .unwrap(),
            I80F48::ZERO
        );

        let adjust_token = |c: &mut HealthCache, value: f64| {
            let ti = &mut c.token_infos[0];
            ti.balance_native += I80F48::from_num(value);
        };
        let find_max_trade =
            |c: &HealthCache, side: PerpOrderSide, ratio: f64, price_factor: f64| {
                let prices = &c.perp_infos[0].prices;
                let trade_price = I80F48::from_num(price_factor) * prices.oracle;
                let base_lots = c
                    .max_perp_for_health_ratio(0, trade_price, side, I80F48::from_num(ratio))
                    .unwrap();
                if base_lots == i64::MAX {
                    return (i64::MAX, f64::MAX, f64::MAX);
                }

                let direction = match side {
                    PerpOrderSide::Bid => 1,
                    PerpOrderSide::Ask => -1,
                };

                // compute the health ratio we'd get when executing the trade
                let actual_ratio = {
                    let base_lots = direction * base_lots;
                    let base_native = I80F48::from(base_lots * base_lot_size);
                    let mut c = c.clone();
                    c.perp_infos[0].base_lots += base_lots;
                    c.perp_infos[0].quote -= base_native * trade_price;
                    c.health_ratio(HealthType::Init).to_num::<f64>()
                };
                // the ratio for trading just one base lot extra
                let plus_ratio = {
                    let base_lots = direction * (base_lots + 1);
                    let base_native = I80F48::from(base_lots * base_lot_size);
                    let mut c = c.clone();
                    c.perp_infos[0].base_lots += base_lots;
                    c.perp_infos[0].quote -= base_native * trade_price;
                    c.health_ratio(HealthType::Init).to_num::<f64>()
                };
                (base_lots, actual_ratio, plus_ratio)
            };
        let check_max_trade = |c: &HealthCache,
                               side: PerpOrderSide,
                               ratio: f64,
                               price_factor: f64| {
            let (base_lots, actual_ratio, plus_ratio) =
                find_max_trade(c, side, ratio, price_factor);
            println!(
                    "checking for price_factor: {price_factor}, target ratio {ratio}: actual ratio: {actual_ratio}, plus ratio: {plus_ratio}, base_lots: {base_lots}",
                );
            let max_binary_search_error = 0.1;
            assert!(ratio <= actual_ratio);
            assert!(plus_ratio - max_binary_search_error <= ratio);
        };

        {
            let mut health_cache = health_cache.clone();
            adjust_token(&mut health_cache, 3000.0);

            for existing in [-5, 0, 3] {
                let mut c = health_cache.clone();
                c.perp_infos[0].base_lots += existing;
                c.perp_infos[0].quote -= I80F48::from(existing * base_lot_size * 2);

                for side in [PerpOrderSide::Bid, PerpOrderSide::Ask] {
                    println!("test 0: existing {existing}, side {side:?}");
                    for price_factor in [0.8, 1.0, 1.1] {
                        for ratio in 1..=100 {
                            check_max_trade(&c, side, ratio as f64, price_factor);
                        }
                    }
                }
            }

            // check some extremely bad prices
            check_max_trade(&health_cache, PerpOrderSide::Bid, 50.0, 2.0);
            check_max_trade(&health_cache, PerpOrderSide::Ask, 50.0, 0.1);

            // and extremely good prices
            assert_eq!(
                find_max_trade(&health_cache, PerpOrderSide::Bid, 50.0, 0.1).0,
                i64::MAX
            );
            assert_eq!(
                find_max_trade(&health_cache, PerpOrderSide::Ask, 50.0, 1.5).0,
                i64::MAX
            );
        }
    }

    #[test]
    fn test_health_perp_funding() {
        let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut account = MangoAccountValue::from_bytes(&buffer).unwrap();

        let group = Pubkey::new_unique();

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 1, 1.0, 0.2, 0.1);
        bank1
            .data()
            .change_without_fee(
                account.ensure_token_position(1).unwrap().0,
                I80F48::from(100),
                DUMMY_NOW_TS,
                DUMMY_PRICE,
            )
            .unwrap();

        let mut perp1 = mock_perp_market(group, oracle1.pubkey, 1.0, 9, 0.2, 0.1);
        perp1.data().long_funding = I80F48::from_num(10.1);
        let perpaccount = account.ensure_perp_position(9, 1).unwrap().0;
        perpaccount.record_trade(perp1.data(), 10, I80F48::from(-110));
        perpaccount.long_settled_funding = I80F48::from_num(10.0);

        let oracle1_ai = oracle1.as_account_info();
        let ais = vec![
            bank1.as_account_info(),
            oracle1_ai.clone(),
            perp1.as_account_info(),
            oracle1_ai,
        ];

        let retriever = ScanningAccountRetriever::new_with_staleness(&ais, &group, None).unwrap();

        assert!(health_eq(
            compute_health(&account.borrow(), HealthType::Init, &retriever).unwrap(),
            // token
            0.8 * 100.0
            // perp base
            + 0.8 * 100.0
            // perp quote
            - 110.0
            // perp funding (10 * (10.1 - 10.0))
            - 1.0
        ));
    }

    #[test]
    fn test_scanning_retreiver_mismatched_oracle_for_perps_throws_error() {
        let group = Pubkey::new_unique();

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 1, 1.0, 0.2, 0.1);
        let (mut bank2, mut oracle2) = mock_bank_and_oracle(group, 4, 5.0, 0.5, 0.3);

        let mut oo1 = TestAccount::<OpenOrders>::new_zeroed();

        let mut perp1 = mock_perp_market(group, oracle1.pubkey, 1.0, 9, 0.2, 0.1);
        let mut perp2 = mock_perp_market(group, oracle2.pubkey, 5.0, 8, 0.2, 0.1);

        let oracle1_account_info = oracle1.as_account_info();
        let oracle2_account_info = oracle2.as_account_info();
        let ais = vec![
            bank1.as_account_info(),
            bank2.as_account_info(),
            oracle1_account_info.clone(),
            oracle2_account_info.clone(),
            perp1.as_account_info(),
            perp2.as_account_info(),
            oracle2_account_info, // Oracles wrong way around
            oracle1_account_info,
            oo1.as_account_info(),
        ];

        let retriever = ScanningAccountRetriever::new_with_staleness(&ais, &group, None).unwrap();
        let result = retriever.perp_market_and_oracle_price(&group, 0, 9);
        assert!(result.is_err());
    }

    #[test]
    fn test_health_stable_price_token() {
        let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut account = MangoAccountValue::from_bytes(&buffer).unwrap();
        let buffer2 = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut account2 = MangoAccountValue::from_bytes(&buffer2).unwrap();
        let buffer3 = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut account3 = MangoAccountValue::from_bytes(&buffer3).unwrap();

        let group = Pubkey::new_unique();

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 1, 1.0, 0.2, 0.1);
        bank1.data().stable_price_model.stable_price = 0.5;
        bank1
            .data()
            .change_without_fee(
                account.ensure_token_position(1).unwrap().0,
                I80F48::from(100),
                DUMMY_NOW_TS,
                DUMMY_PRICE,
            )
            .unwrap();
        bank1
            .data()
            .change_without_fee(
                account2.ensure_token_position(1).unwrap().0,
                I80F48::from(-100),
                DUMMY_NOW_TS,
                DUMMY_PRICE,
            )
            .unwrap();

        let mut perp1 = mock_perp_market(group, oracle1.pubkey, 1.0, 9, 0.2, 0.1);
        perp1.data().stable_price_model.stable_price = 0.5;
        let perpaccount = account3.ensure_perp_position(9, 1).unwrap().0;
        perpaccount.record_trade(perp1.data(), 10, I80F48::from(-100));

        let oracle1_ai = oracle1.as_account_info();
        let ais = vec![
            bank1.as_account_info(),
            oracle1_ai.clone(),
            perp1.as_account_info(),
            oracle1_ai,
        ];

        let retriever = ScanningAccountRetriever::new_with_staleness(&ais, &group, None).unwrap();

        assert!(health_eq(
            compute_health(&account.borrow(), HealthType::Init, &retriever).unwrap(),
            0.8 * 0.5 * 100.0
        ));
        assert!(health_eq(
            compute_health(&account.borrow(), HealthType::Maint, &retriever).unwrap(),
            0.9 * 1.0 * 100.0
        ));
        assert!(health_eq(
            compute_health(&account2.borrow(), HealthType::Init, &retriever).unwrap(),
            -1.2 * 1.0 * 100.0
        ));
        assert!(health_eq(
            compute_health(&account2.borrow(), HealthType::Maint, &retriever).unwrap(),
            -1.1 * 1.0 * 100.0
        ));
        assert!(health_eq(
            compute_health(&account3.borrow(), HealthType::Init, &retriever).unwrap(),
            0.8 * 0.5 * 10.0 * 10.0 - 100.0
        ));
        assert!(health_eq(
            compute_health(&account3.borrow(), HealthType::Maint, &retriever).unwrap(),
            0.9 * 1.0 * 10.0 * 10.0 - 100.0
        ));
    }
}
