use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::{state::*, error::MangoError};

#[derive(Accounts)]
pub struct Serum3DeregisterMarket<'info> {
    #[account(
        mut,
        has_one = admin,
        constraint = group.load()?.is_testing(),
        constraint = group.load()?.is_operational() @ MangoError::GroupIsHalted
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        close = sol_destination
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,

    #[account(
        mut,
        has_one = group,
        constraint = serum_market.load()?.market_index == index_reservation.load()?.market_index,
        close = sol_destination
    )]
    pub index_reservation: AccountLoader<'info, Serum3MarketIndexReservation>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn serum3_deregister_market(_ctx: Context<Serum3DeregisterMarket>) -> Result<()> {
    Ok(())
}
