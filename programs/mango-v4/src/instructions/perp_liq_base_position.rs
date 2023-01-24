use anchor_lang::prelude::*;
use checked_math as cm;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::*;
use crate::state::*;

use crate::logs::{emit_perp_balances, PerpLiqBasePositionLog};

#[derive(Accounts)]
pub struct PerpLiqBasePosition<'info> {
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
}

pub fn perp_liq_base_position(
    ctx: Context<PerpLiqBasePosition>,
    mut max_base_transfer: i64,
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
    let base_lot_size = I80F48::from(perp_market.base_lot_size);

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

    // Take over the liqee's base in exchange for quote
    let liqee_base_lots = liqee_perp_position.base_position_lots();
    require_msg!(liqee_base_lots != 0, "liqee base position is zero");
    // Each lot the base position gets closer to 0, the unweighted perp health contribution
    // increases by this amount.
    let unweighted_health_per_lot;
    // -1 (liqee base lots decrease) or +1 (liqee base lots increase)
    let direction: i64;
    // Either 1+fee or 1-fee, depending on direction.
    let fee_factor;
    if liqee_base_lots > 0 {
        require_msg!(
            max_base_transfer > 0,
            "max_base_transfer must be positive when liqee's base_position is positive"
        );

        // the unweighted perp health contribution gets reduced by `base * price * perp_init_asset_weight`
        // and increased by `base * price * (1 - liq_fee) * quote_init_asset_weight`
        let quote_init_asset_weight = I80F48::ONE;
        direction = -1;
        fee_factor = cm!(I80F48::ONE - perp_market.liquidation_fee);
        unweighted_health_per_lot = cm!(price_per_lot
            * (-perp_market.init_base_asset_weight + quote_init_asset_weight * fee_factor));
    } else {
        // liqee_base_lots < 0
        require_msg!(
            max_base_transfer < 0,
            "max_base_transfer must be negative when liqee's base_position is positive"
        );

        // health gets increased by `base * price * perp_init_liab_weight`
        // and reduced by `base * price * (1 + liq_fee) * quote_init_liab_weight`
        let quote_init_liab_weight = I80F48::ONE;
        direction = 1;
        fee_factor = cm!(I80F48::ONE + perp_market.liquidation_fee);
        unweighted_health_per_lot = cm!(price_per_lot
            * (perp_market.init_base_liab_weight - quote_init_liab_weight * fee_factor));
    };
    assert!(unweighted_health_per_lot > 0);

    // Step 1: bring a negative perp health contribution to >= 0 (as long as overall init health is
    // negative enough). In that region the overall liab weight is guaranteed to be 1.
    let unweighted_perp_init = liqee_health_cache
        .perp_info(perp_market_index)?
        .unweighted_health_contribution(HealthType::Init);
    let step1_health_limit = unweighted_perp_init
        .max(liqee_init_health)
        .min(I80F48::ZERO);
    let step1_base = cm!(-step1_health_limit / unweighted_health_per_lot)
        .checked_ceil()
        .unwrap()
        .checked_to_num::<i64>()
        .unwrap()
        .min(liqee_base_lots.abs())
        .min(max_base_transfer.abs());
    let step1_unweighted_perp_init_change =
        cm!(I80F48::from(step1_base) * unweighted_health_per_lot);
    let unweighted_perp_init_after_step1 =
        cm!(unweighted_perp_init + step1_unweighted_perp_init_change);
    let liqee_init_health_after_step1 = if step1_base > 0 {
        if unweighted_perp_init_after_step1 > 0 {
            // the unweighted negative amount is completely compensated, the overshoot is weighted
            cm!(liqee_init_health - unweighted_perp_init
                + unweighted_perp_init_after_step1 * perp_market.init_pnl_asset_weight)
        } else {
            // partially compensated negative unweighted amount
            cm!(liqee_init_health + step1_unweighted_perp_init_change)
        }
    } else {
        liqee_init_health
    };
    msg!(
        "step1: {} lots, health {} -> {}, unweighted perp {} -> {}",
        step1_base,
        liqee_init_health,
        liqee_init_health_after_step1,
        unweighted_perp_init,
        unweighted_perp_init_after_step1
    );

    // Step 2: in markets with overall weight >0 it's possible to liquidate further
    let step2_health_limit = liqee_init_health_after_step1.min(I80F48::ZERO);
    let step2_base =
        if unweighted_perp_init_after_step1 >= 0 && perp_market.init_pnl_asset_weight > 0 {
            let weighted_health_per_lot =
                cm!(unweighted_health_per_lot * perp_market.init_pnl_asset_weight);
            cm!(-step2_health_limit / weighted_health_per_lot)
                .checked_ceil()
                .unwrap()
                .checked_to_num::<i64>()
                .unwrap()
                .min(liqee_base_lots.abs() - step1_base)
                .min(max_base_transfer.abs() - step1_base)
        } else {
            0
        };
    msg!("step2: {} lots", step2_base);

    // Execute the transfer. This is essentially a forced trade and updates the
    // liqee and liqors entry and break even prices.
    let base_transfer = cm!(direction * (step1_base + step2_base));
    let quote_transfer = cm!(-I80F48::from(base_transfer) * price_per_lot * fee_factor);
    msg!(
        "transfering: {} base lots and {} quote",
        base_transfer,
        quote_transfer
    );

    require!(
        base_transfer != 0,
        MangoError::NoLiquidatablePerpBasePosition
    );

    liqee_perp_position.record_trade(&mut perp_market, base_transfer, quote_transfer);
    liqor_perp_position.record_trade(&mut perp_market, -base_transfer, -quote_transfer);

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

    emit!(PerpLiqBasePositionLog {
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
    let liqee_init_health = liqee_health_cache.health(HealthType::Init);
    liqee
        .fixed
        .maybe_recover_from_being_liquidated(liqee_init_health);

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
