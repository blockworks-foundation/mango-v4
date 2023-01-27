use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use checked_math as cm;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::*;
use crate::state::*;

use crate::logs::{emit_perp_balances, PerpLiqBaseAndPositivePnlLog};

#[derive(Accounts)]
pub struct PerpLiqBaseAndPositivePnl<'info> {
    #[account(
        constraint = group.load()?.is_operational() @ MangoError::GroupIsHalted
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group, has_one = oracle)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = group,
        constraint = liqor.load()?.is_operational() @ MangoError::AccountIsFrozen
        // liqor_owner is checked at #1
    )]
    pub liqor: AccountLoader<'info, MangoAccountFixed>,
    pub liqor_owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        constraint = liqee.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub liqee: AccountLoader<'info, MangoAccountFixed>,

    // bank correctness is checked at #2
    #[account(mut, has_one = group)]
    pub settle_bank: AccountLoader<'info, Bank>,

    #[account(
        mut,
        address = settle_bank.load()?.vault
    )]
    pub settle_vault: Account<'info, TokenAccount>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,
}

pub fn perp_liq_base_and_positive_pnl(
    ctx: Context<PerpLiqBaseAndPositivePnl>,
    mut max_base_transfer: i64,
    max_quote_transfer: u64,
) -> Result<()> {
    // Ensure max_base_transfer can be negated
    max_base_transfer = max_base_transfer.max(i64::MIN + 1);

    let group_pk = &ctx.accounts.group.key();

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
    let liqee_init_health = liqee_health_cache.health(HealthType::Init);
    liqee_health_cache.require_after_phase1_liquidation()?;

    if !liqee.check_liquidatable(&liqee_health_cache)? {
        return Ok(());
    }

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let perp_market_index = perp_market.perp_market_index;
    let settle_token_index = perp_market.settle_token_index;
    let base_lot_size = I80F48::from(perp_market.base_lot_size);

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
    let price_per_lot = cm!(base_lot_size * oracle_price);

    // Fetch perp positions for accounts, creating for the liqor if needed
    let liqee_perp_position = liqee.perp_position_mut(perp_market_index)?;
    require!(
        !liqee_perp_position.has_open_taker_fills(),
        MangoError::HasOpenPerpTakerFills
    );

    let liqor_perp_position = liqor
        .ensure_perp_position(perp_market_index, perp_market.settle_token_index)?
        .0;

    // Settle funding
    liqee_perp_position.settle_funding(&perp_market);
    liqor_perp_position.settle_funding(&perp_market);

    // Max settleable on the liqee?
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    liqee_perp_position.update_settle_limit(&perp_market, now_ts);
    let liqee_positive_settle_limit = liqee_perp_position.available_settle_limit(&perp_market).1;

    // The max settleable amount does not need to be constrained by the liqor's perp settle health,
    // because taking over perp pnl decreases liqor health: every unit of pnl taken costs
    // (1-positive_pnl_liq_fee) USDC and only gains init_pnl_asset_weight in perp health.
    let max_settle = I80F48::from(max_quote_transfer);

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
        unweighted_health_per_lot = cm!(price_per_lot
            * (-perp_market.init_base_asset_weight + quote_init_asset_weight * fee_factor));
    } else {
        // liqee_base_lots <= 0
        require_msg!(
            max_base_transfer <= 0,
            "max_base_transfer can't be positive when liqee's base_position is positive"
        );

        // health gets increased by `base * price * perp_init_liab_weight`
        // and reduced by `base * price * (1 + liq_fee) * quote_init_liab_weight`
        let quote_init_liab_weight = I80F48::ONE;
        direction = 1;
        fee_factor = cm!(I80F48::ONE + perp_market.base_liquidation_fee);
        unweighted_health_per_lot = cm!(price_per_lot
            * (perp_market.init_base_liab_weight - quote_init_liab_weight * fee_factor));
    };
    assert!(unweighted_health_per_lot > 0);

    let spot_gain_per_settled = cm!(I80F48::ONE - perp_market.positive_pnl_liquidation_fee);
    let init_overall_asset_weight = perp_market.init_pnl_asset_weight;
    let expected_perp_health = |unweighted: I80F48| {
        if unweighted < 0 {
            unweighted
        } else if unweighted < max_settle {
            cm!(unweighted * spot_gain_per_settled)
        } else {
            let unsettled = cm!(unweighted - max_settle);
            cm!(max_settle * spot_gain_per_settled + unsettled * init_overall_asset_weight)
        }
    };

    //
    // Several steps of perp base position reduction will follow, and they'll update
    // these variables
    //
    let mut base_reduction = 0;
    let mut current_expected_health = liqee_init_health;
    let perp_info = liqee_health_cache.perp_info(perp_market_index)?;
    let mut current_unweighted_perp_health =
        perp_info.unweighted_health_contribution(HealthType::Init);
    let mut current_expected_perp_health = expected_perp_health(current_unweighted_perp_health);

    let initial_weighted_perp_health =
        perp_info.weigh_health_contribution(current_unweighted_perp_health, HealthType::Init);

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
    if current_unweighted_perp_health >= 0 && current_unweighted_perp_health < max_settle {
        let settled_health_per_lot = cm!(unweighted_health_per_lot * spot_gain_per_settled);
        reduce_base(
            "settleable",
            cm!(max_settle - current_unweighted_perp_health),
            settled_health_per_lot,
            &mut current_unweighted_perp_health,
        );
    }

    //
    // Step 3: Above that, perp base positions only benefit account health if the pnl asset weight is positive
    //
    if current_unweighted_perp_health >= max_settle && perp_market.init_pnl_asset_weight > 0 {
        let weighted_health_per_lot =
            cm!(unweighted_health_per_lot * perp_market.init_pnl_asset_weight);
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
    let quote_transfer = cm!(-I80F48::from(base_transfer) * price_per_lot * fee_factor);
    if base_transfer != 0 {
        msg!(
            "transfering: {} base lots and {} quote",
            base_transfer,
            quote_transfer
        );
        liqee_perp_position.record_trade(&mut perp_market, base_transfer, quote_transfer);
        liqor_perp_position.record_trade(&mut perp_market, -base_transfer, -quote_transfer);
    }

    //
    // Step 4: Let the liqor take over positive pnl until the account health is positive,
    // but only while the unweighted perp health is positive (otherwise it would decrease liqee health!)
    //
    let final_weighted_perp_health =
        perp_info.weigh_health_contribution(current_unweighted_perp_health, HealthType::Init);
    let current_actual_health =
        cm!(liqee_init_health - initial_weighted_perp_health + final_weighted_perp_health);
    let step3_possible =
        current_actual_health < 0 && current_unweighted_perp_health > 0 && max_settle > 0;
    let step3_settlement = if step3_possible {
        let health_per_settle = cm!(spot_gain_per_settled - perp_market.init_pnl_asset_weight);
        let settle_for_zero = cm!(-current_actual_health / health_per_settle)
            .checked_ceil()
            .unwrap();
        let liqee_pnl = liqee_perp_position.unsettled_pnl(&perp_market, oracle_price)?;

        // Allow settling *more* than the liqee_positive_settle_limit. In exchange, the liqor
        // also can't settle fully immediately and just takes over a fractional chunk of the limit.
        let settlement = liqee_pnl
            .min(max_settle)
            .min(settle_for_zero)
            .min(current_unweighted_perp_health)
            .max(I80F48::ZERO);
        let limit_transfer = {
            // take care, liqee_limit may be i64::MAX
            let liqee_limit: i128 = liqee_positive_settle_limit.into();
            let settle = settlement.checked_floor().unwrap().to_num::<i128>();
            let total = liqee_pnl.checked_ceil().unwrap().to_num::<i128>();
            let liqor_limit: i64 = cm!(liqee_limit * settle / total).try_into().unwrap();
            I80F48::from(liqor_limit).min(settlement).max(I80F48::ONE)
        };

        // The liqor pays less than the full amount to receive the positive pnl
        let token_transfer = cm!(settlement * spot_gain_per_settled);

        if settlement > 0 {
            liqor_perp_position.record_liquidation_pnl_takeover(settlement, limit_transfer);
            liqee_perp_position.record_settle(settlement);

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
        msg!("pnl: {}, quote = {}", settlement, token_transfer);

        settlement
    } else {
        I80F48::ZERO
    };

    // Skip out if this instruction had nothing to do
    if base_transfer == 0 && step3_settlement == 0 {
        return Ok(());
    }

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

    emit!(PerpLiqBaseAndPositivePnlLog {
        mango_group: ctx.accounts.group.key(),
        perp_market_index: perp_market.perp_market_index,
        liqor: ctx.accounts.liqor.key(),
        liqee: ctx.accounts.liqee.key(),
        base_transfer,
        quote_transfer: quote_transfer.to_bits(),
        price: oracle_price.to_bits(),
    });

    // Check liqee health again
    liqee_health_cache.recompute_perp_info(liqee_perp_position, &perp_market)?;
    let liqee_init_health_after = liqee_health_cache.health(HealthType::Init);
    liqee
        .fixed
        .maybe_recover_from_being_liquidated(liqee_init_health_after);
    require_gte!(liqee_init_health_after, liqee_init_health);
    msg!(
        "liqee health: {} -> {}",
        liqee_init_health,
        liqee_init_health_after
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
