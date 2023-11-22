use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::instructions::apply_settle_changes;
use crate::instructions::charge_loan_origination_fees;
use crate::logs::{emit_stack, Serum3OpenOrdersBalanceLogV2};
use crate::serum3_cpi::{load_open_orders_ref, OpenOrdersAmounts, OpenOrdersSlim};
use crate::state::*;

pub fn serum3_liq_force_cancel_orders(
    ctx: Context<Serum3LiqForceCancelOrders>,
    limit: u8,
) -> Result<()> {
    //
    // Validation
    //
    let serum_market = ctx.accounts.serum_market.load()?;
    {
        let account = ctx.accounts.account.load_full()?;

        // Validate open_orders #2
        require!(
            account
                .serum3_orders(serum_market.market_index)?
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
            quote_bank.token_index == serum_market.quote_token_index,
            MangoError::SomeError
        );
        let base_bank = ctx.accounts.base_bank.load()?;
        require!(
            base_bank.vault == ctx.accounts.base_vault.key(),
            MangoError::SomeError
        );
        require!(
            base_bank.token_index == serum_market.base_token_index,
            MangoError::SomeError
        );
    }

    //
    // Early return if if liquidation is not allowed or if market is not in force close
    //
    let mut health_cache = {
        let mut account = ctx.accounts.account.load_full_mut()?;
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
        let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
        let health_cache = new_health_cache(&account.borrow(), &retriever, now_ts)
            .context("create health cache")?;

        let liquidatable = account.check_liquidatable(&health_cache)?;
        let can_force_cancel = !account.fixed.is_operational()
            || liquidatable == CheckLiquidatable::Liquidatable
            || serum_market.is_force_close();
        if !can_force_cancel {
            return Ok(());
        }

        health_cache
    };

    //
    // Charge any open loan origination fees
    //
    let before_oo = {
        let open_orders = load_open_orders_ref(ctx.accounts.open_orders.as_ref())?;
        let before_oo = OpenOrdersSlim::from_oo(&open_orders);
        let mut account = ctx.accounts.account.load_full_mut()?;
        let mut base_bank = ctx.accounts.base_bank.load_mut()?;
        let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
        charge_loan_origination_fees(
            &ctx.accounts.group.key(),
            &ctx.accounts.account.key(),
            serum_market.market_index,
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
    cpi_cancel_all_orders(ctx.accounts, limit)?;
    cpi_settle_funds(ctx.accounts)?;

    //
    // After-settle tracking
    //
    let after_oo;
    {
        let oo_ai = &ctx.accounts.open_orders.as_ref();
        let open_orders = load_open_orders_ref(oo_ai)?;
        after_oo = OpenOrdersSlim::from_oo(&open_orders);

        emit_stack(Serum3OpenOrdersBalanceLogV2 {
            mango_group: ctx.accounts.group.key(),
            mango_account: ctx.accounts.account.key(),
            market_index: serum_market.market_index,
            base_token_index: serum_market.base_token_index,
            quote_token_index: serum_market.quote_token_index,
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
    apply_settle_changes(
        &group,
        ctx.accounts.account.key(),
        &mut account.borrow_mut(),
        &mut base_bank,
        &mut quote_bank,
        &serum_market,
        before_base_vault,
        before_quote_vault,
        &before_oo,
        after_base_vault,
        after_quote_vault,
        &after_oo,
        Some(&mut health_cache),
        true,
        None,
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

fn cpi_cancel_all_orders(ctx: &Serum3LiqForceCancelOrders, limit: u8) -> Result<()> {
    use crate::serum3_cpi;
    let group = ctx.group.load()?;
    serum3_cpi::CancelOrder {
        program: ctx.serum_program.to_account_info(),
        market: ctx.serum_market_external.to_account_info(),
        bids: ctx.market_bids.to_account_info(),
        asks: ctx.market_asks.to_account_info(),
        event_queue: ctx.market_event_queue.to_account_info(),

        open_orders: ctx.open_orders.to_account_info(),
        open_orders_authority: ctx.group.to_account_info(),
    }
    .cancel_all(&group, limit)
}

fn cpi_settle_funds(ctx: &Serum3LiqForceCancelOrders) -> Result<()> {
    use crate::serum3_cpi;
    let group = ctx.group.load()?;
    serum3_cpi::SettleFunds {
        program: ctx.serum_program.to_account_info(),
        market: ctx.serum_market_external.to_account_info(),
        open_orders: ctx.open_orders.to_account_info(),
        open_orders_authority: ctx.group.to_account_info(),
        base_vault: ctx.market_base_vault.to_account_info(),
        quote_vault: ctx.market_quote_vault.to_account_info(),
        user_base_wallet: ctx.base_vault.to_account_info(),
        user_quote_wallet: ctx.quote_vault.to_account_info(),
        vault_signer: ctx.market_vault_signer.to_account_info(),
        token_program: ctx.token_program.to_account_info(),
        rebates_quote_wallet: ctx.quote_vault.to_account_info(),
    }
    .call(&group)
}
