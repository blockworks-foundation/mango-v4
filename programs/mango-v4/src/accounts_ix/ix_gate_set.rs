use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct IxGateSet<'info> {
    #[account(
        mut,
        // group <-> admin relation is checked at #1
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,
}
