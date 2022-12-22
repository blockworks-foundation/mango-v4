use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::*;

#[derive(Accounts)]
pub struct Serum3CloseOpenOrders<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::Serum3CloseOpenOrders) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // owner is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
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
    /// CHECK: Validated inline by checking against the pubkey stored in the account at #2
    pub open_orders: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,
}
