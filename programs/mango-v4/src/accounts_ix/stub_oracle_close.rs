use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::{error::MangoError, state::*};

#[derive(Accounts)]
pub struct StubOracleClose<'info> {
    #[account(
        has_one = admin,
        constraint = group.load()?.is_ix_enabled(IxGate::StubOracleClose) @ MangoError::IxIsDisabled,
        constraint = group.load()?.is_testing(),
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    // match stub oracle to group
    #[account(
        mut,
        has_one = group,
        close = sol_destination
    )]
    pub oracle: AccountLoader<'info, StubOracle>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
