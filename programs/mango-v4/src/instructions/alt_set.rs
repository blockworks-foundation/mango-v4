use anchor_lang::prelude::*;
use solana_address_lookup_table_program as solana_alt;

use crate::accounts_ix::*;
use crate::error::*;

/// Register or replace an address lookup table
pub fn alt_set(ctx: Context<AltSet>, index: u8) -> Result<()> {
    let alt_bytes = ctx.accounts.address_lookup_table.try_borrow_data()?;
    solana_alt::state::AddressLookupTable::deserialize(&alt_bytes)
        .map_err(|e| error_msg!("could not deserialize alt: {}", e))?;

    // FUTURE: When the solana feature
    //   relax_authority_signer_check_for_lookup_table_creation
    //   "relax authority signer check for lookup table creation #27205"
    // is enabled (introduced in b79abb4fab62da487c6834926ef309d4c1b69011)
    // we can require ALTs to have the group as owner.
    /*
    require_msg!(
        alt_data.meta.authority.is_some(),
        "alt must have an authority"
    );
    require_keys_eq!(alt_data.meta.authority.unwrap(), ctx.accounts.group.key());
    */

    let mut group = ctx.accounts.group.load_mut()?;
    group.address_lookup_tables[index as usize] = ctx.accounts.address_lookup_table.key();
    Ok(())
}
