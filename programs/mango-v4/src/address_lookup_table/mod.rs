use anchor_lang::prelude::*;
mod solana_address_lookup_table_instruction;
pub use solana_address_lookup_table_instruction::*;
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};
use std::str::FromStr;

pub fn id() -> Pubkey {
    Pubkey::from_str(&"AddressLookupTab1e1111111111111111111111111").unwrap()
}

/// The maximum number of addresses that a lookup table can hold
pub const LOOKUP_TABLE_MAX_ADDRESSES: usize = 256;

/// The serialized size of lookup table metadata
pub const LOOKUP_TABLE_META_SIZE: usize = 56;

pub const LOOKUP_TABLE_MAX_ACCOUNT_SIZE: usize =
    LOOKUP_TABLE_META_SIZE + LOOKUP_TABLE_MAX_ADDRESSES * PUBKEY_BYTES;

pub fn addresses<'a>(table: &[u8]) -> &[Pubkey] {
    bytemuck::try_cast_slice(&table[LOOKUP_TABLE_META_SIZE..]).unwrap()
}

pub fn contains(table: &[u8], pubkey: &Pubkey) -> bool {
    addresses(table)
        .iter()
        .find(|&addr| addr == pubkey)
        .is_some()
}

pub fn extend<'info>(
    lookup_table_ai: AccountInfo<'info>,
    authority_ai: AccountInfo<'info>,
    payer_ai: AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
    new_addresses: Vec<Pubkey>,
) -> std::result::Result<(), ProgramError> {
    let instruction = extend_lookup_table(
        lookup_table_ai.key(),
        authority_ai.key(),
        payer_ai.key(),
        new_addresses,
    );
    let account_infos = [
        lookup_table_ai,
        authority_ai,
        payer_ai,
    ];
    solana_program::program::invoke_signed(&instruction, &account_infos, signer_seeds)
}
