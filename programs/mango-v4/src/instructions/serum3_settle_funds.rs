use std::borrow::BorrowMut;

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use fixed::types::I80F48;

use crate::error::*;
use crate::serum3_cpi::load_open_orders_ref;
use crate::state::*;

use super::{apply_vault_difference, OpenOrdersReserved, OpenOrdersSlim};

#[derive(Accounts)]
pub struct Serum3SettleFunds<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account
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

    // token_index and bank.vault == vault is validated inline
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
        let account = ctx.accounts.account.load()?;
        require!(
            account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
            MangoError::SomeError
        );

        require!(!account.fixed.is_bankrupt(), MangoError::IsBankrupt);

        // Validate open_orders
        require!(
            account
                .serum3_find(serum_market.market_index)
                .ok_or_else(|| error!(MangoError::SomeError))?
                .open_orders
                == ctx.accounts.open_orders.key(),
            MangoError::SomeError
        );

        // Validate banks and vaults
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
    // Before-order tracking
    //

    let before_base_vault = ctx.accounts.base_vault.amount;
    let before_quote_vault = ctx.accounts.quote_vault.amount;

    //
    // Settle
    //
    {
        let open_orders = load_open_orders_ref(ctx.accounts.open_orders.as_ref())?;
        cpi_settle_funds(ctx.accounts)?;

        let after_oo = OpenOrdersSlim::from_oo(&open_orders);
        let mut account = ctx.accounts.account.load_mut()?;
        let mut base_bank = ctx.accounts.base_bank.load_mut()?;
        let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
        charge_maybe_fees(
            serum_market.market_index,
            &mut base_bank,
            &mut quote_bank,
            &mut account.borrow_mut(),
            &after_oo,
        )?;
    }

    //
    // After-order tracking
    //
    {
        ctx.accounts.base_vault.reload()?;
        ctx.accounts.quote_vault.reload()?;
        let after_base_vault = ctx.accounts.base_vault.amount;
        let after_quote_vault = ctx.accounts.quote_vault.amount;

        // Charge the difference in vault balances to the user's account
        let mut account = ctx.accounts.account.load_mut()?;
        let mut base_bank = ctx.accounts.base_bank.load_mut()?;
        let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
        apply_vault_difference(
            &mut account.borrow_mut(),
            &mut base_bank,
            after_base_vault,
            before_base_vault,
            &mut quote_bank,
            after_quote_vault,
            before_quote_vault,
        )?
        .deactivate_inactive_token_accounts(&mut account.borrow_mut());
    }

    Ok(())
}

// if reserved is less than cached, charge loan fee on the difference
pub fn charge_maybe_fees(
    market_index: Serum3MarketIndex,
    coin_bank: &mut Bank,
    pc_bank: &mut Bank,
    account: &mut MangoAccountRefMut,
    after_oo: &OpenOrdersSlim,
) -> Result<()> {
    let serum3_account = account.serum3_find_mut(market_index).unwrap();

    let maybe_actualized_coin_loan = I80F48::from_num::<u64>(
        serum3_account
            .previous_native_coin_reserved
            .saturating_sub(after_oo.native_coin_reserved()),
    );

    if maybe_actualized_coin_loan > 0 {
        serum3_account.previous_native_coin_reserved = after_oo.native_coin_reserved();

        // loan origination fees
        let coin_token_account = account.token_get_mut(coin_bank.token_index)?.0;
        let coin_token_native = coin_token_account.native(&coin_bank);

        if coin_token_native.is_negative() {
            let actualized_loan = coin_token_native.abs().min(maybe_actualized_coin_loan);
            // note: the withdraw has already happened while placing the order
            // now that the loan is actually materialized (since the fill having taken place)
            // charge the loan origination fee
            coin_bank
                .borrow_mut()
                .withdraw_loan_origination_fee(coin_token_account, actualized_loan)?;
        }
    }

    let serum3_account = account.serum3_find_mut(market_index).unwrap();
    let maybe_actualized_pc_loan = I80F48::from_num::<u64>(
        serum3_account
            .previous_native_pc_reserved
            .saturating_sub(after_oo.native_pc_reserved()),
    );

    if maybe_actualized_pc_loan > 0 {
        serum3_account.previous_native_pc_reserved = after_oo.native_pc_reserved();

        // loan origination fees
        let pc_token_account = account.token_get_mut(pc_bank.token_index)?.0;
        let pc_token_native = pc_token_account.native(pc_bank);

        if pc_token_native.is_negative() {
            let actualized_loan = pc_token_native.abs().min(maybe_actualized_pc_loan);
            // note: the withdraw has already happened while placing the order
            // now that the loan is actually materialized (since the fill having taken place)
            // charge the loan origination fee
            pc_bank
                .borrow_mut()
                .withdraw_loan_origination_fee(pc_token_account, actualized_loan)?;
        }
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
