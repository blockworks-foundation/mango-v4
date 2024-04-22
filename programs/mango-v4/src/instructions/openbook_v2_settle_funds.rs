use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::error::*;
use crate::serum3_cpi::{OpenOrdersAmounts, OpenOrdersSlim};
use crate::state::*;
use openbook_v2::cpi::accounts::SettleFunds;

use crate::accounts_ix::*;
use crate::instructions::openbook_v2_place_order::apply_settle_changes;
use crate::logs::{
    emit_stack, LoanOriginationFeeInstruction, OpenbookV2OpenOrdersBalanceLog, WithdrawLoanLog,
};

use crate::accounts_zerocopy::AccountInfoRef;

/// Settling means moving free funds from the open orders account
/// back into the mango account wallet.
///
/// There will be free funds on open_orders when an order was triggered.
///
pub fn openbook_v2_settle_funds<'info>(
    ctx: Context<OpenbookV2SettleFunds>,
    fees_to_dao: bool,
) -> Result<()> {
    let openbook_market = ctx.accounts.openbook_v2_market.load()?;

    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load_full()?;
        // account constraint #1
        require!(
            account
                .fixed
                .is_owner_or_delegate(ctx.accounts.authority.key()),
            MangoError::SomeError
        );

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

        // Validate oracles #4
        require_keys_eq!(
            base_bank.oracle,
            ctx.accounts.base_oracle.key(),
            MangoError::SomeError
        );
        require_keys_eq!(
            quote_bank.oracle,
            ctx.accounts.quote_oracle.key(),
            MangoError::SomeError
        );
    }

    //
    // Charge any open loan origination fees
    //
    let base_lot_size: u64;
    let quote_lot_size: u64;
    let before_oo;
    {
        let openbook_market_external = ctx.accounts.openbook_v2_market_external.load()?;
        base_lot_size = openbook_market_external.base_lot_size.try_into().unwrap();
        quote_lot_size = openbook_market_external.quote_lot_size.try_into().unwrap();

        let open_orders = ctx.accounts.open_orders.load()?;
        before_oo = OpenOrdersSlim::from_oo_v2(&open_orders, base_lot_size, quote_lot_size);
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
            Some(&ctx.accounts.base_oracle.to_account_info()),
            Some(&ctx.accounts.quote_oracle.to_account_info()),
        )?;
    }

    //
    // Settle
    //
    let before_base_vault = ctx.accounts.base_vault.amount;
    let before_quote_vault = ctx.accounts.quote_vault.amount;
    let mango_account_seeds_data = ctx.accounts.account.load()?.pda_seeds();
    let seeds = &mango_account_seeds_data.signer_seeds();
    cpi_settle_funds(ctx.accounts, &[seeds])?;

    //
    // After-settle tracking
    //
    let after_oo = {
        let open_orders = ctx.accounts.open_orders.load()?;
        OpenOrdersSlim::from_oo_v2(&open_orders, base_lot_size, quote_lot_size)
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
        None,
        fees_to_dao,
        Some(&ctx.accounts.quote_oracle.to_account_info()),
        &open_orders,
    )?;

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

    Ok(())
}

// Charge fees if the potential borrows are bigger than the funds on the open orders account
pub fn charge_loan_origination_fees(
    group_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    market_index: OpenbookV2MarketIndex,
    base_bank: &mut Bank,
    quote_bank: &mut Bank,
    account: &mut MangoAccountRefMut,
    before_oo: &OpenOrdersSlim,
    base_oracle: Option<&AccountInfo>,
    quote_oracle: Option<&AccountInfo>,
) -> Result<()> {
    let openbook_v2_orders = account.openbook_v2_orders_mut(market_index).unwrap();

    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();

    let oo_base_total = before_oo.native_base_total();
    let actualized_base_loan = I80F48::from_num(
        openbook_v2_orders
            .base_borrows_without_fee
            .saturating_sub(oo_base_total),
    );
    if actualized_base_loan > 0 {
        openbook_v2_orders.base_borrows_without_fee = oo_base_total;

        // now that the loan is actually materialized, charge the loan origination fee
        // note: the withdraw has already happened while placing the order
        let base_token_account = account.token_position_mut(base_bank.token_index)?.0;
        let withdraw_result = base_bank.withdraw_loan_origination_fee(
            base_token_account,
            actualized_base_loan,
            now_ts,
        )?;

        let base_oracle_price = base_oracle
            .map(|ai| {
                let ai_ref = &AccountInfoRef::borrow(ai)?;
                base_bank.oracle_price(
                    &OracleAccountInfos::from_reader(ai_ref),
                    Some(Clock::get()?.slot),
                )
            })
            .transpose()?;

        emit_stack(WithdrawLoanLog {
            mango_group: *group_pubkey,
            mango_account: *account_pubkey,
            token_index: base_bank.token_index,
            loan_amount: withdraw_result.loan_amount.to_bits(),
            loan_origination_fee: withdraw_result.loan_origination_fee.to_bits(),
            instruction: LoanOriginationFeeInstruction::OpenbookV2SettleFunds,
            price: base_oracle_price.map(|p| p.to_bits()),
        });
    }

    let openbook_v2_account = account.openbook_v2_orders_mut(market_index).unwrap();
    let oo_quote_total = before_oo.native_quote_total();
    let actualized_quote_loan = I80F48::from_num::<u64>(
        openbook_v2_account
            .quote_borrows_without_fee
            .saturating_sub(oo_quote_total),
    );
    if actualized_quote_loan > 0 {
        openbook_v2_account.quote_borrows_without_fee = oo_quote_total;

        // now that the loan is actually materialized, charge the loan origination fee
        // note: the withdraw has already happened while placing the order
        let quote_token_account = account.token_position_mut(quote_bank.token_index)?.0;
        let withdraw_result = quote_bank.withdraw_loan_origination_fee(
            quote_token_account,
            actualized_quote_loan,
            now_ts,
        )?;

        let quote_oracle_price = quote_oracle
            .map(|ai| {
                let ai_ref = &AccountInfoRef::borrow(ai)?;
                quote_bank.oracle_price(
                    &OracleAccountInfos::from_reader(ai_ref),
                    Some(Clock::get()?.slot),
                )
            })
            .transpose()?;

        emit_stack(WithdrawLoanLog {
            mango_group: *group_pubkey,
            mango_account: *account_pubkey,
            token_index: quote_bank.token_index,
            loan_amount: withdraw_result.loan_amount.to_bits(),
            loan_origination_fee: withdraw_result.loan_origination_fee.to_bits(),
            instruction: LoanOriginationFeeInstruction::OpenbookV2SettleFunds,
            price: quote_oracle_price.map(|p| p.to_bits()),
        });
    }

    Ok(())
}

fn cpi_settle_funds<'info>(ctx: &OpenbookV2SettleFunds<'info>, seeds: &[&[&[u8]]]) -> Result<()> {
    let cpi_accounts = SettleFunds {
        penalty_payer: ctx.authority.to_account_info(),
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
