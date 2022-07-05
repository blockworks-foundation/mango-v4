use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Serum3CancelAllOrders<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_owner_or_delegate(owner.key()),
    )]
    pub account: AccountLoader<'info, MangoAccount>,
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
    pub market_bids: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_asks: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_event_queue: UncheckedAccount<'info>,
}

pub fn serum3_cancel_all_orders(ctx: Context<Serum3CancelAllOrders>, limit: u8) -> Result<()> {
    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load()?;
        require!(account.is_bankrupt == 0, MangoError::IsBankrupt);

        let serum_market = ctx.accounts.serum_market.load()?;

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
    }

    //
    // Cancel
    //
    cpi_cancel_all_orders(ctx.accounts, limit)?;

    Ok(())
}

fn cpi_cancel_all_orders(ctx: &Serum3CancelAllOrders, limit: u8) -> Result<()> {
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
