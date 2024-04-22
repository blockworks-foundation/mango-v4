use anchor_lang::prelude::*;
use openbook_v2::cpi::accounts::{CancelOrder, SettleFunds};

use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::instructions::openbook_v2_place_order::apply_settle_changes;
use crate::instructions::openbook_v2_settle_funds::charge_loan_origination_fees;
use crate::logs::{emit_stack, OpenbookV2OpenOrdersBalanceLog};
use crate::serum3_cpi::OpenOrdersAmounts;
use crate::serum3_cpi::OpenOrdersSlim;
use crate::state::*;
use crate::util::clock_now;

pub fn openbook_v2_liq_force_cancel_orders(
    ctx: Context<OpenbookV2LiqForceCancelOrders>,
    limit: u8,
) -> Result<()> {
    //
    // Validation
    //
    let openbook_market = ctx.accounts.openbook_v2_market.load()?;
    {
        let account = ctx.accounts.account.load_full()?;

        // Validate open_orders #2
        require!(
            account
                .openbook_v2_orders(openbook_market.market_index)?
                .open_orders
                == ctx.accounts.open_orders.key(),
            MangoError::SomeError
        );

        // Validate banks and vaults #3
        let quote_bank = ctx.accounts.quote_bank.load()?;
        require!(
            quote_bank.vault == ctx.accounts.quote_vault.key(),
            MangoError::SomeError
        );
        require!(
            quote_bank.token_index == openbook_market.quote_token_index,
            MangoError::SomeError
        );
        let base_bank = ctx.accounts.base_bank.load()?;
        require!(
            base_bank.vault == ctx.accounts.base_vault.key(),
            MangoError::SomeError
        );
        require!(
            base_bank.token_index == openbook_market.base_token_index,
            MangoError::SomeError
        );
    }

    let (now_ts, now_slot) = clock_now();

    //
    // Early return if if liquidation is not allowed or if market is not in force close
    //
    let mut health_cache = {
        let mut account = ctx.accounts.account.load_full_mut()?;
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow(), now_slot)?;
        let health_cache = new_health_cache(&account.borrow(), &retriever, now_ts)
            .context("create health cache")?;

        let liquidatable = account.check_liquidatable(&health_cache)?;
        let can_force_cancel = !account.fixed.is_operational()
            || liquidatable == CheckLiquidatable::Liquidatable
            || openbook_market.is_force_close();
        if !can_force_cancel {
            return Ok(());
        }

        health_cache
    };

    //
    // Charge any open loan origination fees
    //
    let openbook_market_external = ctx.accounts.openbook_v2_market_external.load()?;
    let base_lot_size: u64 = openbook_market_external.base_lot_size.try_into().unwrap();
    let quote_lot_size: u64 = openbook_market_external.quote_lot_size.try_into().unwrap();
    let before_oo = {
        let open_orders = ctx.accounts.open_orders.load()?;
        let before_oo = OpenOrdersSlim::from_oo_v2(&open_orders, base_lot_size, quote_lot_size);
        let mut account = ctx.accounts.account.load_full_mut()?;
        let mut base_bank = ctx.accounts.base_bank.load_mut()?;
        let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
        charge_loan_origination_fees(
            &ctx.accounts.group.key(),
            &ctx.accounts.account.key(),
            openbook_market.market_index,
            &mut base_bank,
            &mut quote_bank,
            &mut account.borrow_mut(),
            &before_oo,
            None,
            None,
        )?;

        before_oo
    };

    //
    // Before-settle tracking
    //
    let before_base_vault = ctx.accounts.base_vault.amount;
    let before_quote_vault = ctx.accounts.quote_vault.amount;

    //
    // Cancel all and settle
    //
    let mango_account_seeds_data = ctx.accounts.account.load()?.pda_seeds();
    let seeds = &mango_account_seeds_data.signer_seeds();
    cpi_cancel_all_orders(ctx.accounts, &[seeds], limit)?;
    // this requires a mut ctx.accounts.account for no reason
    drop(openbook_market_external);
    cpi_settle_funds(ctx.accounts, &[seeds])?;

    //
    // After-settle tracking
    //
    let after_oo;
    {
        let open_orders = ctx.accounts.open_orders.load()?;
        after_oo = OpenOrdersSlim::from_oo_v2(&open_orders, base_lot_size, quote_lot_size);

        emit_stack(OpenbookV2OpenOrdersBalanceLog {
            mango_group: ctx.accounts.group.key(),
            mango_account: ctx.accounts.account.key(),
            market_index: openbook_market.market_index,
            base_token_index: openbook_market.base_token_index,
            quote_token_index: openbook_market.quote_token_index,
            base_total: after_oo.native_base_total(),
            base_free: after_oo.native_base_free(),
            quote_total: after_oo.native_quote_total(),
            quote_free: after_oo.native_quote_free(),
            referrer_rebates_accrued: after_oo.native_rebates(),
        });
    };

    ctx.accounts.base_vault.reload()?;
    ctx.accounts.quote_vault.reload()?;
    let after_base_vault = ctx.accounts.base_vault.amount;
    let after_quote_vault = ctx.accounts.quote_vault.amount;

    let mut account = ctx.accounts.account.load_full_mut()?;
    let mut base_bank = ctx.accounts.base_bank.load_mut()?;
    let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
    let group = ctx.accounts.group.load()?;
    let open_orders = ctx.accounts.open_orders.load()?;
    apply_settle_changes(
        &group,
        ctx.accounts.account.key(),
        &mut account.borrow_mut(),
        &mut base_bank,
        &mut quote_bank,
        &openbook_market,
        before_base_vault,
        before_quote_vault,
        &before_oo,
        after_base_vault,
        after_quote_vault,
        &after_oo,
        Some(&mut health_cache),
        true,
        None,
        &open_orders,
    )?;

    //
    // Health check at the end
    //
    let liq_end_health = health_cache.health(HealthType::LiquidationEnd);
    account
        .fixed
        .maybe_recover_from_being_liquidated(liq_end_health);

    Ok(())
}

fn cpi_cancel_all_orders(
    ctx: &OpenbookV2LiqForceCancelOrders,
    seeds: &[&[&[u8]]],
    limit: u8,
) -> Result<()> {
    let group = ctx.group.load()?;
    let cpi_accounts = CancelOrder {
        market: ctx.openbook_v2_market_external.to_account_info(),
        open_orders_account: ctx.open_orders.to_account_info(),
        signer: ctx.account.to_account_info(),
        bids: ctx.bids.to_account_info(),
        asks: ctx.asks.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.openbook_v2_program.to_account_info(),
        cpi_accounts,
        seeds,
    );

    // todo-pan: maybe allow passing side for cu opt
    openbook_v2::cpi::cancel_all_orders(cpi_ctx, None, limit)
}

fn cpi_settle_funds(ctx: &OpenbookV2LiqForceCancelOrders, seeds: &[&[&[u8]]]) -> Result<()> {
    let group = ctx.group.load()?;
    let cpi_accounts = SettleFunds {
        penalty_payer: ctx.payer.to_account_info(),
        market: ctx.openbook_v2_market_external.to_account_info(),
        market_authority: ctx.market_vault_signer.to_account_info(),
        market_base_vault: ctx.market_base_vault.to_account_info(),
        market_quote_vault: ctx.market_quote_vault.to_account_info(),
        user_base_account: ctx.base_vault.to_account_info(),
        user_quote_account: ctx.quote_vault.to_account_info(),
        referrer_account: Some(ctx.quote_vault.to_account_info()),
        token_program: ctx.token_program.to_account_info(),
        owner: ctx.account.to_account_info(),
        open_orders_account: ctx.open_orders.to_account_info(),
        system_program: ctx.system_program.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.openbook_v2_program.to_account_info(),
        cpi_accounts,
        seeds,
    );

    openbook_v2::cpi::settle_funds(cpi_ctx)
}
