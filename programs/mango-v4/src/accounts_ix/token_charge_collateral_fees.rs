use crate::error::MangoError;
use crate::state::*;
use anchor_lang::prelude::*;

/// Charges collateral fees on an account
#[derive(Accounts)]
pub struct TokenChargeCollateralFees<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
}
