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
        has_one = group
        // liqor_owner is checked at #1
    )]
    pub liqor: AccountLoader<'info, MangoAccountFixed>,
    pub liqor_owner: Signer<'info>,

    #[account(mut, has_one = group)]
    pub liqee: AccountLoader<'info, MangoAccountFixed>,
}

pub fn perp_liq_base_position(
    ctx: Context<PerpLiqBasePosition>,
    max_base_transfer: i64,
) -> Result<()> {
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

    // Once maint_health falls below 0, we want to start liquidating,
    // we want to allow liquidation to continue until init_health is positive,
    // to prevent constant oscillation between the two states
    if liqee.being_liquidated() {
        if liqee
            .fixed
            .maybe_recover_from_being_liquidated(liqee_init_health)
        {
            msg!("Liqee init_health above zero");
            return Ok(());
        }
    } else {
        let maint_health = liqee_health_cache.health(HealthType::Maint);
        require!(
            maint_health < I80F48::ZERO,
            MangoError::HealthMustBeNegative
        );
        liqee.fixed.set_being_liquidated(true);
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
    let liqor_perp_position = liqor
        .ensure_perp_position(perp_market_index, perp_market.settle_token_index)?
        .0;
    let liqee_base_lots = liqee_perp_position.base_position_lots();

    // Settle funding
    liqee_perp_position.settle_funding(&perp_market);
    liqor_perp_position.settle_funding(&perp_market);

    // Take over the liqee's base in exchange for quote
    require_msg!(liqee_base_lots != 0, "liqee base position is zero");
    let (base_transfer, quote_transfer) =
        if liqee_base_lots > 0 {
            require_msg!(
                max_base_transfer > 0,
                "max_base_transfer must be positive when liqee's base_position is positive"
            );

            // health gets reduced by `base * price * perp_init_asset_weight`
            // and increased by `base * price * (1 - liq_fee) * quote_init_asset_weight`
            let quote_init_asset_weight = I80F48::ONE;
            let fee_factor = cm!(I80F48::ONE - perp_market.liquidation_fee);
            let health_per_lot = cm!(price_per_lot
                * (-perp_market.init_asset_weight + quote_init_asset_weight * fee_factor));

            // number of lots to transfer to bring health to zero, rounded up
            let base_transfer_for_zero: i64 = cm!(-liqee_init_health / health_per_lot)
                .checked_ceil()
                .unwrap()
                .checked_to_num()
                .unwrap();

            let base_transfer = base_transfer_for_zero
                .min(liqee_base_lots)
                .min(max_base_transfer)
                .max(0);
            let quote_transfer = cm!(-I80F48::from(base_transfer) * price_per_lot * fee_factor);

            (base_transfer, quote_transfer) // base > 0, quote < 0
        } else {
            // liqee_base_lots < 0
            require_msg!(
                max_base_transfer < 0,
                "max_base_transfer must be negative when liqee's base_position is positive"
            );

            // health gets increased by `base * price * perp_init_liab_weight`
            // and reduced by `base * price * (1 + liq_fee) * quote_init_liab_weight`
            let quote_init_liab_weight = I80F48::ONE;
            let fee_factor = cm!(I80F48::ONE + perp_market.liquidation_fee);
            let health_per_lot = cm!(price_per_lot
                * (perp_market.init_liab_weight - quote_init_liab_weight * fee_factor));

            // (negative) number of lots to transfer to bring health to zero, rounded away from zero
            let base_transfer_for_zero: i64 = cm!(liqee_init_health / health_per_lot)
                .checked_floor()
                .unwrap()
                .checked_to_num()
                .unwrap();

            let base_transfer = base_transfer_for_zero
                .max(liqee_base_lots)
                .max(max_base_transfer)
                .min(0);
            let quote_transfer = cm!(-I80F48::from(base_transfer) * price_per_lot * fee_factor);

            (base_transfer, quote_transfer) // base < 0, quote > 0
        };

    // Execute the transfer. This is essentially a forced trade and updates the
    // liqee and liqors entry and break even prices.
    liqee_perp_position.record_trade(&mut perp_market, -base_transfer, -quote_transfer);
    liqor_perp_position.record_trade(&mut perp_market, base_transfer, quote_transfer);

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
        base_transfer: base_transfer,
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
