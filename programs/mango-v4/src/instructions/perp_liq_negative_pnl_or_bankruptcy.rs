use anchor_lang::prelude::*;
use anchor_spl::token;
use checked_math as cm;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::{compute_health, new_health_cache, HealthType, ScanningAccountRetriever};
use crate::logs::{
    emit_perp_balances, PerpLiqBankruptcyLog, PerpLiqNegativePnlOrBankruptcyLog, TokenBalanceLog,
};
use crate::state::*;

pub fn perp_liq_negative_pnl_or_bankruptcy(
    ctx: Context<PerpLiqNegativePnlOrBankruptcy>,
    max_liab_transfer: u64,
) -> Result<()> {
    let mango_group = ctx.accounts.group.key();

    let (perp_market_index, settle_token_index) = {
        let perp_market = ctx.accounts.perp_market.load()?;
        (
            perp_market.perp_market_index,
            perp_market.settle_token_index,
        )
    };

    require_keys_neq!(ctx.accounts.liqor.key(), ctx.accounts.liqee.key());
    let mut liqee = ctx.accounts.liqee.load_full_mut()?;
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

    let mut liqee_health_cache = {
        let retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, &mango_group)
            .context("create account retriever")?;
        new_health_cache(&liqee.borrow(), &retriever)?
    };
    let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);
    let liqee_settle_health = liqee_health_cache.perp_settle_health();
    liqee_health_cache.require_after_phase2_liquidation()?;

    if !liqee.check_liquidatable(&liqee_health_cache)? {
        return Ok(());
    }

    // check positions exist/create them, done early for nicer error messages
    {
        liqee.perp_position(perp_market_index)?;
        liqee.token_position(settle_token_index)?;
        liqor.ensure_perp_position(perp_market_index, settle_token_index)?;
        liqor.ensure_token_position(settle_token_index)?;
    }

    let mut settle_bank = ctx.accounts.settle_bank.load_mut()?;
    // account constraint #2
    require!(
        settle_bank.token_index == settle_token_index,
        MangoError::InvalidBank
    );

    // Get oracle price for market. Price is validated inside
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let oracle_price = perp_market.oracle_price(
        &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
        None, // staleness checked in health
    )?;

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    //
    // Step 1: Allow the liqor to take over ("settle") negative liqee pnl.
    //
    // The only limitation is the liqee's perp_settle_health and its perp pnl settle limit.
    //
    let settlement;
    let max_settlement_liqee;
    {
        let liqee_perp_position = liqee.perp_position_mut(perp_market_index)?;
        let liqor_perp_position = liqor.perp_position_mut(perp_market_index)?;
        liqee_perp_position.settle_funding(&perp_market);
        liqor_perp_position.settle_funding(&perp_market);

        let liqee_pnl = liqee_perp_position.unsettled_pnl(&perp_market, oracle_price)?;
        // TODO: deal with positive liqee pnl! Maybe another instruction?
        require!(liqee_pnl < 0, MangoError::ProfitabilityMismatch);

        // Get settleable pnl on the liqee
        liqee_perp_position.update_settle_limit(&perp_market, now_ts);
        let liqee_settleable_pnl =
            liqee_perp_position.apply_pnl_settle_limit(&perp_market, liqee_pnl);

        max_settlement_liqee = liqee_settle_health
            .min(-liqee_settleable_pnl)
            .max(I80F48::ZERO);
        settlement = max_settlement_liqee
            .min(I80F48::from(max_liab_transfer))
            .max(I80F48::ZERO);
        if settlement > 0 {
            liqor_perp_position.record_liquidation_quote_change(-settlement);
            liqee_perp_position.record_settle(-settlement);

            // Update the accounts' perp_spot_transfer statistics.
            let settlement_i64 = settlement.round_to_zero().checked_to_num::<i64>().unwrap();
            cm!(liqor_perp_position.perp_spot_transfers += settlement_i64);
            cm!(liqee_perp_position.perp_spot_transfers -= settlement_i64);
            cm!(liqor.fixed.perp_spot_transfers += settlement_i64);
            cm!(liqee.fixed.perp_spot_transfers -= settlement_i64);

            // Transfer token balance
            let liqor_token_position = liqor.token_position_mut(settle_token_index)?.0;
            let liqee_token_position = liqee.token_position_mut(settle_token_index)?.0;
            settle_bank.deposit(liqor_token_position, settlement, now_ts)?;
            settle_bank.withdraw_without_fee(
                liqee_token_position,
                settlement,
                now_ts,
                oracle_price,
            )?;
            liqee_health_cache.adjust_token_balance(&settle_bank, -settlement)?;

            emit!(PerpLiqNegativePnlOrBankruptcyLog {
                mango_group,
                liqee: ctx.accounts.liqee.key(),
                liqor: ctx.accounts.liqor.key(),
                perp_market_index,
                settlement: settlement.to_bits(),
            });

            msg!("liquidated pnl = {}", settlement);
        }
    };
    let max_liab_transfer = cm!(I80F48::from(max_liab_transfer) - settlement);

    //
    // Step 2: bankruptcy
    //
    // Remaining pnl that brings the account into negative init health is either:
    // - taken by the liqor in exchange for spot from the insurance fund, or
    // - wiped away and socialized among all perp participants
    //
    let insurance_transfer = if settlement == max_settlement_liqee {
        let liqee_perp_position = liqee.perp_position_mut(perp_market_index)?;
        let liqee_pnl = liqee_perp_position.unsettled_pnl(&perp_market, oracle_price)?;

        let max_liab_transfer_from_liqee =
            (-liqee_pnl).min(-liqee_liq_end_health).max(I80F48::ZERO);
        let liab_transfer = max_liab_transfer_from_liqee
            .min(max_liab_transfer)
            .max(I80F48::ZERO);

        // Available insurance fund coverage
        let insurance_vault_amount = if perp_market.elligible_for_group_insurance_fund() {
            ctx.accounts.insurance_vault.amount
        } else {
            0
        };

        let liquidation_fee_factor = cm!(I80F48::ONE + perp_market.base_liquidation_fee);

        // Amount given to the liqor from the insurance fund
        let insurance_transfer = cm!(liab_transfer * liquidation_fee_factor)
            .checked_ceil()
            .unwrap()
            .checked_to_num::<u64>()
            .unwrap()
            .min(insurance_vault_amount);

        let insurance_transfer_i80f48 = I80F48::from(insurance_transfer);
        let insurance_fund_exhausted = insurance_transfer == insurance_vault_amount;

        // Amount of negative perp pnl transfered to the liqor
        let insurance_liab_transfer =
            cm!(insurance_transfer_i80f48 / liquidation_fee_factor).min(liab_transfer);

        // Try using the insurance fund if possible
        if insurance_transfer > 0 {
            require_keys_eq!(settle_bank.mint, ctx.accounts.insurance_vault.mint);

            // move insurance assets into quote bank
            let group = ctx.accounts.group.load()?;
            let group_seeds = group_seeds!(group);
            token::transfer(
                ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
                insurance_transfer,
            )?;

            // credit the liqor with quote tokens
            let (liqor_quote, _, _) = liqor.ensure_token_position(settle_token_index)?;
            settle_bank.deposit(liqor_quote, insurance_transfer_i80f48, now_ts)?;

            // transfer perp quote loss from the liqee to the liqor
            let liqor_perp_position = liqor.perp_position_mut(perp_market_index)?;
            liqee_perp_position.record_settle(-insurance_liab_transfer);
            liqor_perp_position.record_liquidation_quote_change(-insurance_liab_transfer);
        }

        // Socialize loss if the insurance fund is exhausted
        // At this point, we don't care about the liqor's requested max_liab_tranfer
        let remaining_liab = max_liab_transfer_from_liqee - insurance_liab_transfer;
        let mut socialized_loss = I80F48::ZERO;
        let (starting_long_funding, starting_short_funding) =
            (perp_market.long_funding, perp_market.short_funding);
        if insurance_fund_exhausted && remaining_liab > 0 {
            perp_market.socialize_loss(-remaining_liab)?;
            liqee_perp_position.record_settle(-remaining_liab);
            socialized_loss = remaining_liab;
        }

        emit!(PerpLiqBankruptcyLog {
            mango_group,
            liqee: ctx.accounts.liqee.key(),
            liqor: ctx.accounts.liqor.key(),
            perp_market_index: perp_market.perp_market_index,
            insurance_transfer: insurance_transfer_i80f48.to_bits(),
            socialized_loss: socialized_loss.to_bits(),
            starting_long_funding: starting_long_funding.to_bits(),
            starting_short_funding: starting_short_funding.to_bits(),
            ending_long_funding: perp_market.long_funding.to_bits(),
            ending_short_funding: perp_market.short_funding.to_bits(),
        });

        insurance_transfer
    } else {
        0
    };

    //
    // Log positions aftewards
    //
    if settlement > 0 || insurance_transfer > 0 {
        let liqor_token_position = liqor.token_position(settle_token_index)?;
        emit!(TokenBalanceLog {
            mango_group,
            mango_account: ctx.accounts.liqor.key(),
            token_index: settle_token_index,
            indexed_position: liqor_token_position.indexed_position.to_bits(),
            deposit_index: settle_bank.deposit_index.to_bits(),
            borrow_index: settle_bank.borrow_index.to_bits(),
        });
    }

    if settlement > 0 {
        let liqee_token_position = liqee.token_position(settle_token_index)?;
        emit!(TokenBalanceLog {
            mango_group,
            mango_account: ctx.accounts.liqee.key(),
            token_index: settle_token_index,
            indexed_position: liqee_token_position.indexed_position.to_bits(),
            deposit_index: settle_bank.deposit_index.to_bits(),
            borrow_index: settle_bank.borrow_index.to_bits(),
        });
    }

    let liqee_perp_position = liqee.perp_position(perp_market_index)?;
    let liqor_perp_position = liqor.perp_position(perp_market_index)?;
    emit_perp_balances(
        mango_group,
        ctx.accounts.liqor.key(),
        liqor_perp_position,
        &perp_market,
    );
    emit_perp_balances(
        mango_group,
        ctx.accounts.liqee.key(),
        liqee_perp_position,
        &perp_market,
    );

    // Check liqee health again: bankruptcy would improve health
    liqee_health_cache.recompute_perp_info(liqee_perp_position, &perp_market)?;
    let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);
    liqee
        .fixed
        .maybe_recover_from_being_liquidated(liqee_liq_end_health);

    drop(perp_market);
    drop(settle_bank);

    // Check liqor's health
    if !liqor.fixed.is_in_health_region() {
        let account_retriever =
            ScanningAccountRetriever::new(ctx.remaining_accounts, &mango_group)?;
        let liqor_health = compute_health(&liqor.borrow(), HealthType::Init, &account_retriever)
            .context("compute liqor health")?;
        require!(liqor_health >= 0, MangoError::HealthMustBePositive);
    }

    Ok(())
}
