use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Serum3SettleFunds<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    // Validated inline
    #[account(mut)]
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,
    pub serum_program: UncheckedAccount<'info>,
    #[account(mut)]
    pub serum_market_external: UncheckedAccount<'info>,

    // These accounts are forwarded directly to the serum cpi call
    // and are validated there.
    #[account(mut)]
    pub market_base_vault: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_quote_vault: UncheckedAccount<'info>,
    // needed for the automatic settle_funds call
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
    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load()?;
        let serum_market = ctx.accounts.serum_market.load()?;

        // Validate open_orders
        require!(
            account
                .serum3_account_map
                .find(serum_market.market_index)
                .ok_or(error!(MangoError::SomeError))?
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
    cpi_settle_funds(&ctx.accounts)?;

    //
    // After-order tracking
    //
    ctx.accounts.base_vault.reload()?;
    ctx.accounts.quote_vault.reload()?;
    let after_base_vault = ctx.accounts.base_vault.amount;
    let after_quote_vault = ctx.accounts.quote_vault.amount;

    // Charge the difference in vault balances to the user's account
    {
        let mut account = ctx.accounts.account.load_mut()?;

        let mut base_bank = ctx.accounts.base_bank.load_mut()?;
        let base_position = account.token_account_map.get_mut(base_bank.token_index)?;
        base_bank.change(base_position, (after_base_vault - before_base_vault) as i64)?;

        let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
        let quote_position = account.token_account_map.get_mut(quote_bank.token_index)?;
        quote_bank.change(
            quote_position,
            (after_quote_vault - before_quote_vault) as i64,
        )?;
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
