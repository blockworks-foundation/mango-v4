use crate::{error::MangoError, state::*};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct StubOracleSet<'info> {
    #[account(
        has_one = admin,
        constraint = group.load()?.is_ix_enabled(IxGate::StubOracleSet) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub oracle: AccountLoader<'info, StubOracle>,
}
