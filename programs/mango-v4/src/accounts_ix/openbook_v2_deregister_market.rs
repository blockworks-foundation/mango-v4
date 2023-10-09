use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::{error::MangoError, state::*};

#[derive(Accounts)]
pub struct OpenbookV2DeregisterMarket<'info> {
    #[account(
        mut,
        has_one = admin,
        constraint = group.load()?.is_testing(),
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2DeregisterMarket) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        constraint = group.load()?.admin == admin.key(),
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        close = sol_destination
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    #[account(
        mut,
        has_one = group,
        constraint = openbook_v2_market.load()?.market_index == index_reservation.load()?.market_index,
        close = sol_destination
    )]
    pub index_reservation: AccountLoader<'info, OpenbookV2MarketIndexReservation>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
