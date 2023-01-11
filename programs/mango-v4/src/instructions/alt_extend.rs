use anchor_lang::prelude::*;
use solana_address_lookup_table_program as solana_alt;

use crate::address_lookup_table_program;
use crate::state::*;

#[derive(Accounts)]
pub struct AltExtend<'info> {
    #[account(
        has_one = admin,
        constraint = group.load()?.is_operational()
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,
    pub payer: Signer<'info>,

    /// CHECK: ALT address is checked inline
    #[account(
        mut,
        owner = solana_alt::ID,
    )]
    pub address_lookup_table: UncheckedAccount<'info>,
}

/// Add addresses to a registered lookup table
///
/// NOTE: This only works for ALTs that have the group as owner, see alt_set.
pub fn alt_extend(ctx: Context<AltExtend>, index: u8, new_addresses: Vec<Pubkey>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    require_keys_eq!(
        group.address_lookup_tables[index as usize],
        ctx.accounts.address_lookup_table.key()
    );

    let group_seeds = group_seeds!(group);
    address_lookup_table_program::cpi_extend(
        ctx.accounts.address_lookup_table.to_account_info(),
        ctx.accounts.group.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        &[group_seeds],
        new_addresses,
    )?;

    Ok(())
}
