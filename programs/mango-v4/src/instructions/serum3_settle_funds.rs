use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use fixed::types::I80F48;

use crate::error::*;
use crate::serum3_cpi::load_open_orders_ref;
use crate::state::*;

use super::{apply_vault_difference, OpenOrdersAmounts, OpenOrdersSlim};
use crate::logs::Serum3OpenOrdersBalanceLog;
use crate::logs::{LoanOriginationFeeInstruction, WithdrawLoanOriginationFeeLog};

#[derive(Accounts)]
pub struct Serum3SettleFunds<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::Serum3SettleFunds) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // owner is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account at #2
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_program: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_market_external: UncheckedAccount<'info>,

    // These accounts are forwarded directly to the serum cpi call
    // and are validated there.
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_base_vault: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_quote_vault: UncheckedAccount<'info>,
    /// needed for the automatic settle_funds call
    /// CHECK: Validated by the serum cpi call
    pub market_vault_signer: UncheckedAccount<'info>,

    // token_index and bank.vault == vault is validated inline at #3
    #[account(mut, has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub quote_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, has_one = group)]
    pub base_bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub base_vault: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

/// Settling means moving free funds from the serum3 open orders account
/// back into the mango account wallet.
///
/// There will be free funds on open_orders when an order was triggered.
///
pub fn serum3_settle_funds(ctx: Context<Serum3SettleFunds>) -> Result<()> {
    let serum_market = ctx.accounts.serum_market.load()?;

    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load_full()?;
        // account constraint #1
        require!(
            account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
            MangoError::SomeError
        );

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
    // Charge any open loan origination fees
    //
    {
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
        )?;
    }

    //
    // Settle
    //
    let before_base_vault = ctx.accounts.base_vault.amount;
    let before_quote_vault = ctx.accounts.quote_vault.amount;

    cpi_settle_funds(ctx.accounts)?;

    //
    // After-settle tracking
    //
    {
        ctx.accounts.base_vault.reload()?;
        ctx.accounts.quote_vault.reload()?;
        let after_base_vault = ctx.accounts.base_vault.amount;
        let after_quote_vault = ctx.accounts.quote_vault.amount;

        // Settle cannot decrease vault balances
        require_gte!(after_base_vault, before_base_vault);
        require_gte!(after_quote_vault, before_quote_vault);

        // Credit the difference in vault balances to the user's account
        let mut account = ctx.accounts.account.load_full_mut()?;
        let mut base_bank = ctx.accounts.base_bank.load_mut()?;
        let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
        apply_vault_difference(
            ctx.accounts.account.key(),
            &mut account.borrow_mut(),
            serum_market.market_index,
            &mut base_bank,
            after_base_vault,
            before_base_vault,
            // Since after >= before, we know this can be a deposit
            // and no net borrow check will be necessary, meaning
            // we don't need an oracle price.
            None,
        )?;
        apply_vault_difference(
            ctx.accounts.account.key(),
            &mut account.borrow_mut(),
            serum_market.market_index,
            &mut quote_bank,
            after_quote_vault,
            before_quote_vault,
            None,
        )?;
    }

    let oo_ai = &ctx.accounts.open_orders.as_ref();
    let open_orders = load_open_orders_ref(oo_ai)?;
    let after_oo = OpenOrdersSlim::from_oo(&open_orders);
    emit!(Serum3OpenOrdersBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        base_token_index: serum_market.base_token_index,
        quote_token_index: serum_market.quote_token_index,
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
    market_index: Serum3MarketIndex,
    base_bank: &mut Bank,
    quote_bank: &mut Bank,
    account: &mut MangoAccountRefMut,
    before_oo: &OpenOrdersSlim,
) -> Result<()> {
    let serum3_account = account.serum3_orders_mut(market_index).unwrap();

    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();

    let oo_base_total = before_oo.native_base_total();
    let actualized_base_loan = I80F48::from_num(
        serum3_account
            .base_borrows_without_fee
            .saturating_sub(oo_base_total),
    );
    if actualized_base_loan > 0 {
        serum3_account.base_borrows_without_fee = oo_base_total;

        // now that the loan is actually materialized, charge the loan origination fee
        // note: the withdraw has already happened while placing the order
        let base_token_account = account.token_position_mut(base_bank.token_index)?.0;
        let (_, fee) = base_bank.withdraw_loan_origination_fee(
            base_token_account,
            actualized_base_loan,
            now_ts,
        )?;

        emit!(WithdrawLoanOriginationFeeLog {
            mango_group: *group_pubkey,
            mango_account: *account_pubkey,
            token_index: base_bank.token_index,
            loan_origination_fee: fee.to_bits(),
            instruction: LoanOriginationFeeInstruction::Serum3SettleFunds,
        });
    }

    let serum3_account = account.serum3_orders_mut(market_index).unwrap();
    let oo_quote_total = before_oo.native_quote_total_plus_rebates();
    let actualized_quote_loan = I80F48::from_num::<u64>(
        serum3_account
            .quote_borrows_without_fee
            .saturating_sub(oo_quote_total),
    );
    if actualized_quote_loan > 0 {
        serum3_account.quote_borrows_without_fee = oo_quote_total;

        // now that the loan is actually materialized, charge the loan origination fee
        // note: the withdraw has already happened while placing the order
        let quote_token_account = account.token_position_mut(quote_bank.token_index)?.0;
        let (_, fee) = quote_bank.withdraw_loan_origination_fee(
            quote_token_account,
            actualized_quote_loan,
            now_ts,
        )?;

        emit!(WithdrawLoanOriginationFeeLog {
            mango_group: *group_pubkey,
            mango_account: *account_pubkey,
            token_index: quote_bank.token_index,
            loan_origination_fee: fee.to_bits(),
            instruction: LoanOriginationFeeInstruction::Serum3SettleFunds,
        });
    }

    Ok(())
}

fn cpi_settle_funds(ctx: &Serum3SettleFunds) -> Result<()> {
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
    }
    .call(&group)
}
