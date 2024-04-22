use anchor_lang::prelude::*;
use openbook_v2::cpi::accounts::{CreateOpenOrdersAccount, CreateOpenOrdersIndexer};

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

fn is_initialized(account: &UncheckedAccount) -> bool {
    let data: &[u8] = &(account.try_borrow_data().unwrap());
    if data.len() < 8 {
        return false;
    }

    let mut disc_bytes = [0u8; 8];
    disc_bytes.copy_from_slice(&data[..8]);
    let discriminator = u64::from_le_bytes(disc_bytes);
    if discriminator != 0 {
        return false;
    }

    return true;
}

pub fn openbook_v2_create_open_orders(ctx: Context<OpenbookV2CreateOpenOrders>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    {
        let account = ctx.accounts.account.load()?;
        let account_seeds = mango_account_seeds!(account);

        // create indexer if not exists
        if !is_initialized(&ctx.accounts.open_orders_indexer) {
            cpi_init_open_orders_indexer(ctx.accounts, &[account_seeds])?;
        }

        // create open orders account
        cpi_init_open_orders_account(ctx.accounts, &[account_seeds])?;
    }

    let mut account = ctx.accounts.account.load_full_mut()?;
    let openbook_market = ctx.accounts.openbook_v2_market.load()?;

    // account constraint #1
    require!(
        account
            .fixed
            .is_owner_or_delegate(ctx.accounts.authority.key()),
        MangoError::SomeError
    );

    let openbook_market_external = ctx.accounts.openbook_v2_market_external.load()?;

    // add oo to mango account
    let open_orders_account = account.create_openbook_v2_orders(openbook_market.market_index)?;
    open_orders_account.open_orders = ctx.accounts.open_orders_account.key();
    open_orders_account.base_token_index = openbook_market.base_token_index;
    open_orders_account.quote_token_index = openbook_market.quote_token_index;
    open_orders_account.base_lot_size = openbook_market_external.base_lot_size;
    open_orders_account.quote_lot_size = openbook_market_external.quote_lot_size;

    // Make it so that the token_account_map for the base and quote currency
    // stay permanently blocked. Otherwise users may end up in situations where
    // they can't settle a market because they don't have free token_account_map!
    let (quote_position, _, _) =
        account.ensure_token_position(openbook_market.quote_token_index)?;
    quote_position.increment_in_use();
    let (base_position, _, _) = account.ensure_token_position(openbook_market.base_token_index)?;
    base_position.increment_in_use();

    Ok(())
}

fn cpi_init_open_orders_indexer(
    ctx: &OpenbookV2CreateOpenOrders,
    seeds: &[&[&[u8]]],
) -> Result<()> {
    let cpi_accounts = CreateOpenOrdersIndexer {
        payer: ctx.payer.to_account_info(),
        owner: ctx.account.to_account_info(),
        open_orders_indexer: ctx.open_orders_indexer.to_account_info(),
        system_program: ctx.system_program.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.openbook_v2_program.to_account_info(),
        cpi_accounts,
        seeds,
    );

    openbook_v2::cpi::create_open_orders_indexer(cpi_ctx)
}

fn cpi_init_open_orders_account(
    ctx: &OpenbookV2CreateOpenOrders,
    seeds: &[&[&[u8]]],
) -> Result<()> {
    let group = ctx.group.load()?;
    let cpi_accounts = CreateOpenOrdersAccount {
        payer: ctx.payer.to_account_info(),
        owner: ctx.account.to_account_info(),
        delegate_account: Some(ctx.group.to_account_info()),
        open_orders_indexer: ctx.open_orders_indexer.to_account_info(),
        open_orders_account: ctx.open_orders_account.to_account_info(),
        market: ctx.openbook_v2_market_external.to_account_info(),
        system_program: ctx.system_program.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.openbook_v2_program.to_account_info(),
        cpi_accounts,
        seeds,
    );

    openbook_v2::cpi::create_open_orders_account(cpi_ctx, "OpenOrders".to_owned())
}
