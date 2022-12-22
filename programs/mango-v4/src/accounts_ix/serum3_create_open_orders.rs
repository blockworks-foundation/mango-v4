use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Serum3CreateOpenOrders<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::Serum3CreateOpenOrders) @ MangoError::IxIsDisabled,
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

    // initialized by this instruction via cpi to serum
    #[account(
        init,
        seeds = [b"Serum3OO".as_ref(), account.key().as_ref(), serum_market.key().as_ref()],
        bump,
        payer = payer,
        owner = serum_program.key(),
        // 12 is the padding serum uses for accounts ("serum" prefix, "padding" postfix)
        space = 12 + std::mem::size_of::<serum_dex::state::OpenOrders>(),
    )]
    /// CHECK: Newly created by serum cpi call
    pub open_orders: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
