use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn serum3_create_open_orders(ctx: Context<Serum3CreateOpenOrders>) -> Result<()> {
    cpi_init_open_orders(ctx.accounts)?;

    let serum_market = ctx.accounts.serum_market.load()?;

    let mut account = ctx.accounts.account.load_full_mut()?;
    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let serum_account = account.create_serum3_orders(serum_market.market_index)?;
    serum_account.open_orders = ctx.accounts.open_orders.key();
    serum_account.base_token_index = serum_market.base_token_index;
    serum_account.quote_token_index = serum_market.quote_token_index;

    // Make it so that the token_account_map for the base and quote currency
    // stay permanently blocked. Otherwise users may end up in situations where
    // they can't settle a market because they don't have free token_account_map!
    let (quote_position, _, _) = account.ensure_token_position(serum_market.quote_token_index)?;
    quote_position.increment_in_use();
    let (base_position, _, _) = account.ensure_token_position(serum_market.base_token_index)?;
    base_position.increment_in_use();

    Ok(())
}

fn cpi_init_open_orders(ctx: &Serum3CreateOpenOrders) -> Result<()> {
    use crate::serum3_cpi;
    let group = ctx.group.load()?;
    serum3_cpi::InitOpenOrders {
        program: ctx.serum_program.to_account_info(),
        market: ctx.serum_market_external.to_account_info(),
        open_orders: ctx.open_orders.to_account_info(),
        open_orders_authority: ctx.group.to_account_info(),
        rent: ctx.rent.to_account_info(),
    }
    .call(&group)
}
