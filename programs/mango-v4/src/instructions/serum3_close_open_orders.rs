use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::MangoError;
use crate::state::*;

pub fn serum3_close_open_orders(ctx: Context<Serum3CloseOpenOrders>) -> Result<()> {
    //
    // Validation
    //
    let mut account = ctx.accounts.account.load_full_mut()?;
    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let serum_market = ctx.accounts.serum_market.load()?;

    // Validate open_orders #2
    require!(
        account
            .serum3_orders(serum_market.market_index)?
            .open_orders
            == ctx.accounts.open_orders.key(),
        MangoError::SomeError
    );

    //
    // close OO
    //
    cpi_close_open_orders(ctx.accounts)?;

    // Reduce the in_use_count on the token positions - they no longer need to be forced open.
    // We cannot immediately dust tiny positions because we don't have the banks.
    let (base_position, _) = account.token_position_mut(serum_market.base_token_index)?;
    base_position.decrement_in_use();
    let (quote_position, _) = account.token_position_mut(serum_market.quote_token_index)?;
    quote_position.decrement_in_use();

    // Deactivate the serum open orders account itself
    account.deactivate_serum3_orders(serum_market.market_index)?;

    Ok(())
}

fn cpi_close_open_orders(ctx: &Serum3CloseOpenOrders) -> Result<()> {
    use crate::serum3_cpi;
    let group = ctx.group.load()?;
    serum3_cpi::CloseOpenOrders {
        program: ctx.serum_program.to_account_info(),
        market: ctx.serum_market_external.to_account_info(),
        open_orders: ctx.open_orders.to_account_info(),
        open_orders_authority: ctx.group.to_account_info(),
        sol_destination: ctx.sol_destination.to_account_info(),
    }
    .call(&group)
}
