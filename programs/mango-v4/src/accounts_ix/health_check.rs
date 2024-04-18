use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum HealthCheckKind {
    Maint = 0b0000,
    Init = 0b0010,
    LiquidationEnd = 0b0100,
    MaintRatio = 0b0001,
    InitRatio = 0b0011,
    LiquidationEndRatio = 0b0101,
}

#[derive(Accounts)]
pub struct HealthCheck<'info> {
    #[account(
    constraint = group.load()?.is_ix_enabled(IxGate::HealthCheck) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
    mut,
    has_one = group,
    constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
}
