use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::state::*;

#[derive(Accounts)]
pub struct Serum3DeregisterMarket<'info> {
    #[account(
        mut,
        constraint = group.load()?.is_testing(),
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        close = sol_destination
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,

    /// CHECK: Unused account
    #[account(
        mut,
        seeds = [b"Serum3Index".as_ref(), group.key().as_ref(), &serum_market.load()?.market_index.to_le_bytes()],
        bump
    )]
    pub index_reservation: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn serum3_deregister_market(ctx: Context<Serum3DeregisterMarket>) -> Result<()> {
    close_unsafe(
        ctx.accounts.index_reservation.to_account_info(),
        ctx.accounts.sol_destination.to_account_info(),
    );
    Ok(())
}

fn close_unsafe<'info>(info: AccountInfo<'info>, sol_destination: AccountInfo<'info>) {
    // Transfer tokens from the account to the sol_destination.
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() =
        dest_starting_lamports.checked_add(info.lamports()).unwrap();
    **info.lamports.borrow_mut() = 0;

    // Does NOT prevent reinit attacks in any way
}
