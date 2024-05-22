use anchor_lang::prelude::*;

use openbook_v2::cpi::accounts::{CloseOpenOrdersAccount, CloseOpenOrdersIndexer};

use crate::accounts_ix::*;
use crate::error::MangoError;
use crate::state::*;

pub fn openbook_v2_close_open_orders(ctx: Context<OpenbookV2CloseOpenOrders>) -> Result<()> {
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
                == ctx.accounts.open_orders_account.key(),
            MangoError::SomeError
        );

        // Validate banks #3
        let quote_bank = ctx.accounts.quote_bank.load()?;
        let base_bank = ctx.accounts.base_bank.load()?;
        require_eq!(
            quote_bank.token_index,
            openbook_market.quote_token_index,
            MangoError::SomeError
        );
        require_eq!(
            base_bank.token_index,
            openbook_market.base_token_index,
            MangoError::SomeError
        );
    }
    //
    // close OO
    //
    {
        let account = ctx.accounts.account.load()?;
        let seeds = mango_account_seeds!(account);
        cpi_close_open_orders(ctx.accounts, &[seeds])?;
    }

    // Reduce the in_use_count on the token positions - they no longer need to be forced open.
    // Also dust the position since we have banks now
    let now_ts: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
    let account_pubkey = ctx.accounts.account.key();
    let mut account = ctx.accounts.account.load_full_mut()?;
    let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
    let mut base_bank = ctx.accounts.base_bank.load_mut()?;
    account.token_decrement_dust_deactivate(&mut quote_bank, now_ts, account_pubkey)?;
    account.token_decrement_dust_deactivate(&mut base_bank, now_ts, account_pubkey)?;

    // Deactivate the open orders account itself
    account.deactivate_openbook_v2_orders(openbook_market.market_index)?;

    Ok(())
}

fn cpi_close_open_orders(ctx: &OpenbookV2CloseOpenOrders, seeds: &[&[&[u8]]]) -> Result<()> {
    let cpi_accounts = CloseOpenOrdersAccount {
        owner: ctx.account.to_account_info(),
        open_orders_indexer: ctx.open_orders_indexer.to_account_info(),
        open_orders_account: ctx.open_orders_account.to_account_info(),
        sol_destination: ctx.sol_destination.to_account_info(),
        system_program: ctx.system_program.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.openbook_v2_program.to_account_info(),
        cpi_accounts,
        seeds,
    );

    openbook_v2::cpi::close_open_orders_account(cpi_ctx)?;

    // close indexer too if it's empty, will be recreated if create_open_orders is called again
    if !ctx.open_orders_indexer.has_active_open_orders_accounts() {
        let cpi_accounts = CloseOpenOrdersIndexer {
            owner: ctx.account.to_account_info(),
            open_orders_indexer: ctx.open_orders_indexer.to_account_info(),
            sol_destination: ctx.sol_destination.to_account_info(),
            token_program: ctx.token_program.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.openbook_v2_program.to_account_info(),
            cpi_accounts,
            seeds,
        );
        openbook_v2::cpi::close_open_orders_indexer(cpi_ctx)?;
    }

    Ok(())
}
