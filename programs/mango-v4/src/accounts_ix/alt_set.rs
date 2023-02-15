use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use solana_address_lookup_table_program as solana_alt;

#[derive(Accounts)]
pub struct AltSet<'info> {
    #[account(
        mut,
        has_one = admin,
        constraint = group.load()?.is_ix_enabled(IxGate::AltSet) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    /// CHECK: ALT authority is checked inline
    #[account(
        mut,
        owner = solana_alt::ID,
    )]
    pub address_lookup_table: UncheckedAccount<'info>,
}
