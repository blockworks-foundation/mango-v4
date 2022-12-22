use anchor_lang::prelude::*;
use checked_math as cm;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::*;
use crate::state::*;

use crate::accounts_ix::*;
use crate::logs::{emit_perp_balances, PerpLiqBaseOrPositivePnlLog, TokenBalanceLog};

/// This instruction deals with increasing health by:
/// - reducing the liqee's base position
/// - taking over the liqee's positive pnl
///
/// It's a combined instruction because reducing the base position is not necessarily
/// a health-increasing action when perp overall asset weight = 0. There, the pnl
/// takeover can allow further base position to be reduced.
///
/// Taking over negative pnl - or positive pnl when the unweighted perp health contributin
/// is negative - never increases liqee health. That's why it's relegated to the
/// separate liq_negative_pnl_or_bankruptcy instruction instead.
pub fn perp_liq_base_or_positive_pnl(
    ctx: Context<PerpLiqBaseOrPositivePnl>,
    mut max_base_transfer: i64,
    max_pnl_transfer: u64,
) -> Result<()> {
    // Ensure max_base_transfer can be negated
    max_base_transfer = max_base_transfer.max(i64::MIN + 1);

    let group_pk = &ctx.accounts.group.key();

    require_keys_neq!(ctx.accounts.liqor.key(), ctx.accounts.liqee.key());
    let mut liqor = ctx.accounts.liqor.load_full_mut()?;
    // account constraint #1
    require!(
        liqor
            .fixed
            .is_owner_or_delegate(ctx.accounts.liqor_owner.key()),
        MangoError::SomeError
    );
    require_msg_typed!(
        !liqor.fixed.being_liquidated(),
        MangoError::BeingLiquidated,
        "liqor account"
    );

    let mut liqee = ctx.accounts.liqee.load_full_mut()?;

    // Initial liqee health check
    let mut liqee_health_cache = {
        let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)
            .context("create account retriever")?;
        new_health_cache(&liqee.borrow(), &account_retriever)
            .context("create liqee health cache")?
    };
    let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);
    liqee_health_cache.require_after_phase1_liquidation()?;

    if !liqee.check_liquidatable(&liqee_health_cache)? {
        return Ok(());
    }

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let perp_market_index = perp_market.perp_market_index;
    let settle_token_index = perp_market.settle_token_index;

    let mut settle_bank = ctx.accounts.settle_bank.load_mut()?;
    // account constraint #2
    require!(
        settle_bank.token_index == settle_token_index,
        MangoError::InvalidBank
    );

    // Get oracle price for market. Price is validated inside
    let oracle_price = perp_market.oracle_price(
        &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
        None, // checked in health
    )?;

    // Fetch perp positions for accounts, creating for the liqor if needed
    let liqee_perp_position = liqee.perp_position_mut(perp_market_index)?;
    require!(
        !liqee_perp_position.has_open_taker_fills(),
        MangoError::HasOpenPerpTakerFills
    );

    let liqor_perp_position = liqor
        .ensure_perp_position(perp_market_index, perp_market.settle_token_index)?
        .0;

    // Settle funding, update limit
    liqee_perp_position.settle_funding(&perp_market);
    liqor_perp_position.settle_funding(&perp_market);
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    liqee_perp_position.update_settle_limit(&perp_market, now_ts);

    //
    // Perform the liquidation
    //
    let (base_transfer, quote_transfer, pnl_transfer, pnl_settle_limit_transfer) =
        liquidation_action(
            &mut perp_market,
            &mut settle_bank,
            &mut liqor.borrow_mut(),
            &mut liqee.borrow_mut(),
            &mut liqee_health_cache,
            liqee_liq_end_health,
            now_ts,
            max_base_transfer,
            max_pnl_transfer,
        )?;

    //
    // Wrap up
    //

    let liqee_perp_position = liqee.perp_position_mut(perp_market_index)?;
    let liqor_perp_position = liqor.perp_position_mut(perp_market_index)?;

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.liqor.key(),
        liqor_perp_position,
        &perp_market,
    );

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.liqee.key(),
        liqee_perp_position,
        &perp_market,
    );

    if pnl_transfer != 0 {
        let liqee_token_position = liqee.token_position(settle_token_index)?;
        let liqor_token_position = liqor.token_position(settle_token_index)?;

        emit!(TokenBalanceLog {
            mango_group: ctx.accounts.group.key(),
            mango_account: ctx.accounts.liqee.key(),
            token_index: settle_token_index,
            indexed_position: liqee_token_position.indexed_position.to_bits(),
            deposit_index: settle_bank.deposit_index.to_bits(),
            borrow_index: settle_bank.borrow_index.to_bits(),
        });

        emit!(TokenBalanceLog {
            mango_group: ctx.accounts.group.key(),
            mango_account: ctx.accounts.liqor.key(),
            token_index: settle_token_index,
            indexed_position: liqor_token_position.indexed_position.to_bits(),
            deposit_index: settle_bank.deposit_index.to_bits(),
            borrow_index: settle_bank.borrow_index.to_bits(),
        });
    }

    if base_transfer != 0 || pnl_transfer != 0 {
        emit!(PerpLiqBaseOrPositivePnlLog {
            mango_group: ctx.accounts.group.key(),
            perp_market_index: perp_market.perp_market_index,
            liqor: ctx.accounts.liqor.key(),
            liqee: ctx.accounts.liqee.key(),
            base_transfer,
            quote_transfer: quote_transfer.to_bits(),
            pnl_transfer: pnl_transfer.to_bits(),
            pnl_settle_limit_transfer: pnl_settle_limit_transfer.to_bits(),
            price: oracle_price.to_bits(),
        });
    }

    // Check liqee health again
    let liqee_liq_end_health_after = liqee_health_cache.health(HealthType::LiquidationEnd);
    liqee
        .fixed
        .maybe_recover_from_being_liquidated(liqee_liq_end_health_after);
    require_gte!(liqee_liq_end_health_after, liqee_liq_end_health);
    msg!(
        "liqee liq end health: {} -> {}",
        liqee_liq_end_health,
        liqee_liq_end_health_after
    );

    drop(settle_bank);
    drop(perp_market);

    // Check liqor's health
    if !liqor.fixed.is_in_health_region() {
        let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)
            .context("create account retriever end")?;
        let liqor_health = compute_health(&liqor.borrow(), HealthType::Init, &account_retriever)
            .context("compute liqor health")?;
        require!(liqor_health >= 0, MangoError::HealthMustBePositive);
    }

    Ok(())
}

pub(crate) fn liquidation_action(
    perp_market: &mut PerpMarket,
    settle_bank: &mut Bank,
    liqor: &mut MangoAccountRefMut,
    liqee: &mut MangoAccountRefMut,
    liqee_health_cache: &mut HealthCache,
    liqee_liq_end_health: I80F48,
    now_ts: u64,
    max_base_transfer: i64,
    max_pnl_transfer: u64,
) -> Result<(i64, I80F48, I80F48, I80F48)> {
    let perp_market_index = perp_market.perp_market_index;
    let settle_token_index = perp_market.settle_token_index;

    let liqee_perp_position = liqee.perp_position_mut(perp_market_index)?;
    let liqor_perp_position = liqor.perp_position_mut(perp_market_index)?;

    let perp_info = liqee_health_cache.perp_info(perp_market_index)?;
    let oracle_price = perp_info.prices.oracle;
    let base_lot_size = I80F48::from(perp_market.base_lot_size);
    let oracle_price_per_lot = cm!(base_lot_size * oracle_price);

    let liqee_positive_settle_limit = liqee_perp_position.settle_limit(&perp_market).1;

    // The max settleable amount does not need to be constrained by the liqor's perp settle health,
    // because taking over perp quote decreases liqor health: every unit of quote taken costs
    // (1-positive_pnl_liq_fee) USDC and only gains init_overall_asset_weight in perp health.
    let max_pnl_transfer = I80F48::from(max_pnl_transfer);

    // Take over the liqee's base in exchange for quote
    let liqee_base_lots = liqee_perp_position.base_position_lots();
    // Each lot the base position gets closer to 0, the unweighted perp health contribution
    // increases by this amount.
    let unweighted_health_per_lot;
    // -1 (liqee base lots decrease) or +1 (liqee base lots increase)
    let direction: i64;
    // Either 1+fee or 1-fee, depending on direction.
    let fee_factor;
    if liqee_base_lots > 0 {
        require_msg!(
            max_base_transfer >= 0,
            "max_base_transfer can't be negative when liqee's base_position is positive"
        );

        // the unweighted perp health contribution gets reduced by `base * price * perp_init_asset_weight`
        // and increased by `base * price * (1 - liq_fee) * quote_init_asset_weight`
        let quote_init_asset_weight = I80F48::ONE;
        direction = -1;
        fee_factor = cm!(I80F48::ONE - perp_market.base_liquidation_fee);
        let asset_price = perp_info.prices.asset(HealthType::LiquidationEnd);
        unweighted_health_per_lot = cm!(-asset_price
            * base_lot_size
            * perp_market.init_base_asset_weight
            + oracle_price_per_lot * quote_init_asset_weight * fee_factor);
    } else {
        // liqee_base_lots <= 0
        require_msg!(
            max_base_transfer <= 0,
            "max_base_transfer can't be positive when liqee's base_position is negative"
        );

        // health gets increased by `base * price * perp_init_liab_weight`
        // and reduced by `base * price * (1 + liq_fee) * quote_init_liab_weight`
        let quote_init_liab_weight = I80F48::ONE;
        direction = 1;
        fee_factor = cm!(I80F48::ONE + perp_market.base_liquidation_fee);
        let liab_price = perp_info.prices.liab(HealthType::LiquidationEnd);
        unweighted_health_per_lot = cm!(liab_price
            * base_lot_size
            * perp_market.init_base_liab_weight
            - oracle_price_per_lot * quote_init_liab_weight * fee_factor);
    };
    assert!(unweighted_health_per_lot > 0);

    // Amount of settle token received for each token that is settled
    let spot_gain_per_settled = cm!(I80F48::ONE - perp_market.positive_pnl_liquidation_fee);

    let init_overall_asset_weight = perp_market.init_overall_asset_weight;

    // The overall health contribution from perp including spot health increases from settling pnl.
    // This is needed in order to reduce the base position the right amount when taking into
    // account the settlement that will happen afterwards.
    let expected_perp_health = |unweighted: I80F48| {
        if unweighted < 0 {
            unweighted
        } else if unweighted < max_pnl_transfer {
            cm!(unweighted * spot_gain_per_settled)
        } else {
            let unsettled = cm!(unweighted - max_pnl_transfer);
            cm!(max_pnl_transfer * spot_gain_per_settled + unsettled * init_overall_asset_weight)
        }
    };

    //
    // Several steps of perp base position reduction will follow, and they'll update
    // these variables
    //
    let mut base_reduction = 0;
    let mut current_unweighted_perp_health =
        perp_info.unweighted_health_contribution(HealthType::LiquidationEnd);
    let initial_weighted_perp_health = perp_info
        .weigh_health_contribution(current_unweighted_perp_health, HealthType::LiquidationEnd);
    let mut current_expected_perp_health = expected_perp_health(current_unweighted_perp_health);
    let mut current_expected_health =
        cm!(liqee_liq_end_health + current_expected_perp_health - initial_weighted_perp_health);

    let mut reduce_base = |step: &str,
                           health_amount: I80F48,
                           health_per_lot: I80F48,
                           current_unweighted_perp_health: &mut I80F48| {
        // How much are we willing to increase the unweighted perp health?
        let health_limit = health_amount
            .min(-current_expected_health)
            .max(I80F48::ZERO);
        // How many lots to transfer?
        let base_lots = cm!(health_limit / health_per_lot)
            .checked_ceil() // overshoot to aim for init_health >= 0
            .unwrap()
            .checked_to_num::<i64>()
            .unwrap()
            .min(liqee_base_lots.abs() - base_reduction)
            .min(max_base_transfer.abs() - base_reduction)
            .max(0);
        let unweighted_change = cm!(I80F48::from(base_lots) * unweighted_health_per_lot);
        let current_unweighted = *current_unweighted_perp_health;
        let new_unweighted_perp = cm!(current_unweighted + unweighted_change);
        let new_expected_perp = expected_perp_health(new_unweighted_perp);
        let new_expected_health =
            cm!(current_expected_health + (new_expected_perp - current_expected_perp_health));
        msg!(
            "{}: {} lots, health {} -> {}, unweighted perp {} -> {}",
            step,
            base_lots,
            current_expected_health,
            new_expected_health,
            current_unweighted,
            new_unweighted_perp
        );

        base_reduction += base_lots;
        current_expected_health = new_expected_health;
        *current_unweighted_perp_health = new_unweighted_perp;
        current_expected_perp_health = new_expected_perp;
    };

    //
    // Step 1: While the perp unsettled health is negative, any perp base position reduction
    // directly increases it for the full amount.
    //
    if current_unweighted_perp_health < 0 {
        reduce_base(
            "negative",
            -current_unweighted_perp_health,
            unweighted_health_per_lot,
            &mut current_unweighted_perp_health,
        );
    }

    //
    // Step 2: If perp unsettled health is positive but below max_settle, perp base position reductions
    // benefit account health slightly less because of the settlement liquidation fee.
    //
    if current_unweighted_perp_health >= 0 && current_unweighted_perp_health < max_pnl_transfer {
        let settled_health_per_lot = cm!(unweighted_health_per_lot * spot_gain_per_settled);
        reduce_base(
            "settleable",
            cm!(max_pnl_transfer - current_unweighted_perp_health),
            settled_health_per_lot,
            &mut current_unweighted_perp_health,
        );
    }

    //
    // Step 3: Above that, perp base positions only benefit account health if the pnl asset weight is positive
    //
    if current_unweighted_perp_health >= max_pnl_transfer && init_overall_asset_weight > 0 {
        let weighted_health_per_lot = cm!(unweighted_health_per_lot * init_overall_asset_weight);
        reduce_base(
            "positive",
            I80F48::MAX,
            weighted_health_per_lot,
            &mut current_unweighted_perp_health,
        );
    }

    //
    // Execute the base reduction. This is essentially a forced trade and updates the
    // liqee and liqors entry and break even prices.
    //
    let base_transfer = cm!(direction * base_reduction);
    let quote_transfer = cm!(-I80F48::from(base_transfer) * oracle_price_per_lot * fee_factor);
    if base_transfer != 0 {
        msg!(
            "transfering: {} base lots and {} quote",
            base_transfer,
            quote_transfer
        );
        liqee_perp_position.record_trade(perp_market, base_transfer, quote_transfer);
        liqor_perp_position.record_trade(perp_market, -base_transfer, -quote_transfer);
    }

    //
    // Step 4: Let the liqor take over positive pnl until the account health is positive,
    // but only while the unweighted perp health is positive (otherwise it would decrease liqee health!)
    //
    let final_weighted_perp_health = perp_info
        .weigh_health_contribution(current_unweighted_perp_health, HealthType::LiquidationEnd);
    let current_actual_health =
        cm!(liqee_liq_end_health - initial_weighted_perp_health + final_weighted_perp_health);
    let pnl_transfer_possible =
        current_actual_health < 0 && current_unweighted_perp_health > 0 && max_pnl_transfer > 0;
    let (pnl_transfer, limit_transfer) = if pnl_transfer_possible {
        let health_per_transfer = cm!(spot_gain_per_settled - init_overall_asset_weight);
        let transfer_for_zero = cm!(-current_actual_health / health_per_transfer)
            .checked_ceil()
            .unwrap();
        let liqee_pnl = liqee_perp_position.unsettled_pnl(&perp_market, oracle_price)?;

        // Allow taking over *more* than the liqee_positive_settle_limit. In exchange, the liqor
        // also can't settle fully immediately and just takes over a fractional chunk of the limit.
        //
        // If this takeover were limited by the settle limit, then we couldn't always bring the liqee
        // base position to zero and would need to deal with that in bankruptcy. Also, the settle
        // limit changes with the base position price, so it'd be hard to say when this liquidation
        // step is done.
        let pnl_transfer = liqee_pnl
            .min(max_pnl_transfer)
            .min(transfer_for_zero)
            .min(current_unweighted_perp_health)
            .max(I80F48::ZERO);
        let limit_transfer = {
            // take care, liqee_limit may be i64::MAX
            let liqee_limit: i128 = liqee_positive_settle_limit.into();
            let settle = pnl_transfer.checked_floor().unwrap().to_num::<i128>();
            let total = liqee_pnl.checked_ceil().unwrap().to_num::<i128>();
            let liqor_limit: i64 = cm!(liqee_limit * settle / total).try_into().unwrap();
            I80F48::from(liqor_limit).min(pnl_transfer).max(I80F48::ONE)
        };

        // The liqor pays less than the full amount to receive the positive pnl
        let token_transfer = cm!(pnl_transfer * spot_gain_per_settled);

        if pnl_transfer > 0 {
            liqor_perp_position.record_liquidation_pnl_takeover(pnl_transfer, limit_transfer);
            liqee_perp_position.record_settle(pnl_transfer);

            // Update the accounts' perp_spot_transfer statistics.
            let transfer_i64 = token_transfer
                .round_to_zero()
                .checked_to_num::<i64>()
                .unwrap();
            cm!(liqor_perp_position.perp_spot_transfers -= transfer_i64);
            cm!(liqee_perp_position.perp_spot_transfers += transfer_i64);
            cm!(liqor.fixed.perp_spot_transfers -= transfer_i64);
            cm!(liqee.fixed.perp_spot_transfers += transfer_i64);

            // Transfer token balance
            let liqor_token_position = liqor.token_position_mut(settle_token_index)?.0;
            let liqee_token_position = liqee.token_position_mut(settle_token_index)?.0;
            settle_bank.deposit(liqee_token_position, token_transfer, now_ts)?;
            settle_bank.withdraw_without_fee(
                liqor_token_position,
                token_transfer,
                now_ts,
                oracle_price,
            )?;
            liqee_health_cache.adjust_token_balance(&settle_bank, token_transfer)?;
        }
        msg!(
            "pnl {} was transferred to liqor for quote {} with settle limit {}",
            pnl_transfer,
            token_transfer,
            limit_transfer
        );

        (pnl_transfer, limit_transfer)
    } else {
        (I80F48::ZERO, I80F48::ZERO)
    };

    let liqee_perp_position = liqee.perp_position_mut(perp_market_index)?;
    liqee_health_cache.recompute_perp_info(liqee_perp_position, &perp_market)?;

    Ok((base_transfer, quote_transfer, pnl_transfer, limit_transfer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{self, test::*};

    #[derive(Clone)]
    struct TestSetup {
        group: Pubkey,
        settle_bank: TestAccount<Bank>,
        settle_oracle: TestAccount<StubOracle>,
        perp_market: TestAccount<PerpMarket>,
        perp_oracle: TestAccount<StubOracle>,
        liqee: MangoAccountValue,
        liqor: MangoAccountValue,
    }

    impl TestSetup {
        fn new() -> Self {
            let group = Pubkey::new_unique();
            let (settle_bank, settle_oracle) = mock_bank_and_oracle(group, 0, 1.0, 0.0, 0.0);
            let (_bank2, perp_oracle) = mock_bank_and_oracle(group, 4, 1.0, 0.5, 0.3);
            let mut perp_market =
                mock_perp_market(group, perp_oracle.pubkey, 1.0, 9, (0.2, 0.1), (0.05, 0.02));
            perp_market.data().base_lot_size = 1;

            let liqee_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            let mut liqee = MangoAccountValue::from_bytes(&liqee_buffer).unwrap();
            {
                liqee.ensure_token_position(0).unwrap();
                liqee.ensure_perp_position(9, 0).unwrap();
            }

            let liqor_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            let mut liqor = MangoAccountValue::from_bytes(&liqor_buffer).unwrap();
            {
                liqor.ensure_token_position(0).unwrap();
                liqor.ensure_perp_position(9, 0).unwrap();
            }

            Self {
                group,
                settle_bank,
                settle_oracle,
                perp_market,
                perp_oracle,
                liqee,
                liqor,
            }
        }

        fn liqee_health_cache(&self) -> HealthCache {
            let mut setup = self.clone();

            let ais = vec![
                setup.settle_bank.as_account_info(),
                setup.settle_oracle.as_account_info(),
                setup.perp_market.as_account_info(),
                setup.perp_oracle.as_account_info(),
            ];
            let retriever =
                ScanningAccountRetriever::new_with_staleness(&ais, &setup.group, None).unwrap();

            health::new_health_cache(&setup.liqee.borrow(), &retriever).unwrap()
        }

        fn run(&self, max_base: i64, max_pnl: u64) -> Result<Self> {
            let mut setup = self.clone();

            let mut liqee_health_cache = setup.liqee_health_cache();
            let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);

            liquidation_action(
                setup.perp_market.data(),
                setup.settle_bank.data(),
                &mut setup.liqor.borrow_mut(),
                &mut setup.liqee.borrow_mut(),
                &mut liqee_health_cache,
                liqee_liq_end_health,
                0,
                max_base,
                max_pnl,
            )?;

            Ok(setup)
        }
    }

    fn token_p(account: &mut MangoAccountValue) -> &mut TokenPosition {
        account.token_position_mut(0).unwrap().0
    }
    fn perp_p(account: &mut MangoAccountValue) -> &mut PerpPosition {
        account.perp_position_mut(9).unwrap()
    }

    macro_rules! assert_eq_f {
        ($value:expr, $expected:expr, $max_error:expr) => {
            let value = $value;
            let expected = $expected;
            let ok = (value.to_num::<f64>() - expected).abs() < $max_error;
            assert!(ok, "value: {value}, expected: {expected}");
        };
    }

    #[test]
    fn test_liq_base_or_positive_pnl() {
        let test_cases = vec![
            (
                "nothing",
                (0.9, 0.9),
                (0.0, 0, 0.0),
                (0.0, 0, 0.0),
                (0, 100),
            ),
            //
            // liquidate base position when perp health is negative
            //
            (
                "neg base liq 1: limited",
                (0.5, 0.5),
                (5.0, -10, 0.0),
                (5.0, -9, -1.0),
                (-1, 100),
            ),
            (
                "neg base liq 2: base to zero",
                (0.5, 0.5),
                (5.0, -10, 0.0),
                (5.0, 0, -10.0),
                (-20, 100),
            ),
            (
                "neg base liq 3: health positive",
                (0.5, 0.5),
                (5.0, -4, 0.0),
                (5.0, -2, -2.0),
                (-20, 100),
            ),
            (
                "pos base liq 1: limited",
                (0.5, 0.5),
                (5.0, 20, -20.0),
                (5.0, 19, -19.0),
                (1, 100),
            ),
            (
                "pos base liq 2: base to zero",
                (0.5, 0.5),
                (0.0, 20, -30.0),
                (0.0, 0, -10.0),
                (100, 100),
            ),
            (
                "pos base liq 3: health positive",
                (0.5, 0.5),
                (5.0, 20, -20.0),
                (5.0, 10, -10.0),
                (100, 100),
            ),
            //
            // liquidate base position when perp health is positive and overall asset weight is positive
            //
            (
                "base liq, pos perp health 1: until health positive",
                (0.5, 1.0),
                (-20.0, 20, 5.0),
                (-20.0, 10, 15.0),
                (100, 100),
            ),
            (
                "base liq, pos perp health 2-1: settle until health positive",
                (0.5, 0.5),
                (-19.0, 20, 10.0),
                (-1.0, 20, -8.0),
                (100, 100),
            ),
            (
                "base liq, pos perp health 2-2: base+settle until health positive",
                (0.5, 0.5),
                (-25.0, 20, 10.0),
                (0.0, 10, -5.0),
                (100, 100),
            ),
            (
                "base liq, pos perp health 2-3: base+settle until pnl limit",
                (0.5, 0.5),
                (-23.0, 20, 10.0),
                (-2.0, 10, -1.0),
                (100, 21),
            ),
            (
                "base liq, pos perp health 2-4: base+settle until base limit",
                (0.5, 0.5),
                (-25.0, 20, 10.0),
                (-4.0, 18, -9.0),
                (2, 100),
            ),
            (
                "base liq, pos perp health 2-5: base+settle until both limits",
                (0.5, 0.5),
                (-25.0, 20, 10.0),
                (-4.0, 16, -7.0),
                (4, 21),
            ),
            (
                "base liq, pos perp health 4: liq some base, then settle some",
                (0.5, 0.5),
                (-20.0, 20, 10.0),
                (-15.0, 10, 15.0),
                (10, 5),
            ),
            (
                "base liq, pos perp health 5: base to zero even without settlement",
                (0.5, 0.5),
                (-20.0, 20, 10.0),
                (-20.0, 0, 30.0),
                (100, 0),
            ),
            //
            // liquidate base position when perp health is positive but overall asset weight is zero
            //
            (
                "base liq, pos perp health 6: don't touch base without settlement",
                (0.5, 0.0),
                (-20.0, 20, 10.0),
                (-20.0, 20, 10.0),
                (10, 0),
            ),
            (
                "base liq, pos perp health 7: settlement without base",
                (0.5, 0.0),
                (-20.0, 20, 10.0),
                (-15.0, 20, 5.0),
                (10, 5),
            ),
            (
                "base liq, pos perp health 8: settlement enables base",
                (0.5, 0.0),
                (-30.0, 20, 10.0),
                (-7.5, 15, -7.5),
                (5, 30),
            ),
            (
                "base liq, pos perp health 9: until health positive",
                (0.5, 0.0),
                (-25.0, 20, 10.0),
                (0.0, 10, -5.0),
                (200, 200),
            ),
        ];

        for (
            name,
            (base_weight, overall_weight),
            (init_liqee_spot, init_liqee_base, init_liqee_quote),
            (exp_liqee_spot, exp_liqee_base, exp_liqee_quote),
            (max_base, max_pnl),
        ) in test_cases
        {
            println!("test: {name}");
            let mut setup = TestSetup::new();
            {
                let pm = setup.perp_market.data();
                pm.init_base_asset_weight = I80F48::from_num(base_weight);
                pm.init_base_liab_weight = I80F48::from_num(2.0 - base_weight);
                pm.init_overall_asset_weight = I80F48::from_num(overall_weight);
            }
            {
                perp_p(&mut setup.liqee).record_trade(
                    setup.perp_market.data(),
                    init_liqee_base,
                    I80F48::from_num(init_liqee_quote),
                );

                let settle_bank = setup.settle_bank.data();
                settle_bank
                    .change_without_fee(
                        token_p(&mut setup.liqee),
                        I80F48::from_num(init_liqee_spot),
                        0,
                        I80F48::from(1),
                    )
                    .unwrap();
                settle_bank
                    .change_without_fee(
                        token_p(&mut setup.liqor),
                        I80F48::from_num(1000.0),
                        0,
                        I80F48::from(1),
                    )
                    .unwrap();
            }

            let mut result = setup.run(max_base, max_pnl).unwrap();

            let liqee_perp = perp_p(&mut result.liqee);
            assert_eq!(liqee_perp.base_position_lots(), exp_liqee_base);
            assert_eq_f!(liqee_perp.quote_position_native(), exp_liqee_quote, 0.01);
            let liqor_perp = perp_p(&mut result.liqor);
            assert_eq!(
                liqor_perp.base_position_lots(),
                -(exp_liqee_base - init_liqee_base)
            );
            assert_eq_f!(
                liqor_perp.quote_position_native(),
                -(exp_liqee_quote - init_liqee_quote),
                0.01
            );
            let settle_bank = result.settle_bank.data();
            assert_eq_f!(
                token_p(&mut result.liqee).native(settle_bank),
                exp_liqee_spot,
                0.01
            );
            assert_eq_f!(
                token_p(&mut result.liqor).native(settle_bank),
                1000.0 - (exp_liqee_spot - init_liqee_spot),
                0.01
            );
        }
    }

    // Checks that the stable price does _not_ affect the liquidation target amount
    #[test]
    fn test_liq_base_or_positive_pnl_stable_price() {
        let mut setup = TestSetup::new();
        {
            let pm = setup.perp_market.data();
            pm.stable_price_model.stable_price = 0.5;
            pm.init_base_asset_weight = I80F48::from_num(0.6);
            pm.maint_base_asset_weight = I80F48::from_num(0.8);
        }
        {
            perp_p(&mut setup.liqee).record_trade(
                setup.perp_market.data(),
                30,
                I80F48::from_num(-30),
            );

            let settle_bank = setup.settle_bank.data();
            settle_bank
                .change_without_fee(
                    token_p(&mut setup.liqee),
                    I80F48::from_num(5.0),
                    0,
                    I80F48::from(1),
                )
                .unwrap();
            settle_bank
                .change_without_fee(
                    token_p(&mut setup.liqor),
                    I80F48::from_num(1000.0),
                    0,
                    I80F48::from(1),
                )
                .unwrap();
        }

        let hc = setup.liqee_health_cache();
        assert_eq_f!(
            hc.health(HealthType::Init),
            5.0 + (-30.0 + 30.0 * 0.5 * 0.6), // init + stable
            0.1
        );
        assert_eq_f!(
            hc.health(HealthType::LiquidationEnd),
            5.0 + (-30.0 + 30.0 * 0.6), // init + oracle
            0.1
        );
        assert_eq_f!(
            hc.health(HealthType::Maint),
            5.0 + (-30.0 + 30.0 * 0.8), // maint + oracle
            0.1
        );

        let mut result = setup.run(100, 0).unwrap();

        let liqee_perp = perp_p(&mut result.liqee);
        assert_eq!(liqee_perp.base_position_lots(), 12);

        let hc = result.liqee_health_cache();
        assert_eq_f!(
            hc.health(HealthType::LiquidationEnd),
            5.0 + (-12.0 + 12.0 * 0.6),
            0.1
        );
    }
}
