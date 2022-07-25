use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::error::*;
use crate::instructions::apply_vault_difference;
use crate::state::*;

#[derive(Accounts)]
pub struct Serum3LiqForceCancelOrders<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,

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
    pub market_bids: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_asks: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_event_queue: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_base_vault: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_quote_vault: UncheckedAccount<'info>,
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

pub fn serum3_liq_force_cancel_orders(
    ctx: Context<Serum3LiqForceCancelOrders>,
    limit: u8,
) -> Result<()> {
    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load()?;
        require!(!account.fixed.is_bankrupt(), MangoError::IsBankrupt);
        let serum_market = ctx.accounts.serum_market.load()?;

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

    // TODO: do the correct health / being_liquidated check
    {
        let account = ctx.accounts.account.load()?;

        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
        let health = compute_health(&account.borrow(), HealthType::Maint, &retriever)?;
        msg!("health: {}", health);
        require!(health < 0, MangoError::SomeError);
    }

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
    }
    .call(&group)
}
