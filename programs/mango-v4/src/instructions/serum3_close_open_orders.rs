use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::*;

#[derive(Accounts)]
pub struct Serum3CloseOpenOrders<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.owner == owner.key() || account.load()?.delegate == owner.key(),
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_program: UncheckedAccount<'info>,
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_market_external: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account
    pub open_orders: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,
}

pub fn serum3_close_open_orders(ctx: Context<Serum3CloseOpenOrders>) -> Result<()> {
    //
    // Validation
    //
    let mut account = ctx.accounts.account.load_mut()?;
    let serum_market = ctx.accounts.serum_market.load()?;
    require!(account.is_bankrupt == 0, MangoError::IsBankrupt);
    // Validate open_orders
    require!(
        account
            .serum3
            .find(serum_market.market_index)
            .ok_or_else(|| error!(MangoError::SomeError))?
            .open_orders
            == ctx.accounts.open_orders.key(),
        MangoError::SomeError
    );

    //
    // close OO
    //
    cpi_close_open_orders(ctx.accounts)?;

    // TODO: decrement in_use_count on the base token and quote token
    account.serum3.deactivate(serum_market.market_index)?;

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
