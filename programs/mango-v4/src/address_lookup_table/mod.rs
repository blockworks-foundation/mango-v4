mod solana_address_lookup_table_instruction;
pub use solana_address_lookup_table_instruction::*;
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};
use std::cell::Ref;
use std::result::Result;
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

pub fn addresses<'a>(table: &'a AccountInfo) -> Result<Ref<'a, [Pubkey]>, ProgramError> {
    Ok(Ref::map(table.try_borrow_data()?, |d| {
        bytemuck::try_cast_slice(&d[LOOKUP_TABLE_META_SIZE..]).unwrap()
    }))
}

pub fn contains(table: &AccountInfo, pubkey: &Pubkey) -> std::result::Result<bool, ProgramError> {
    Ok(addresses(table)?
        .iter()
        .find(|&addr| addr == pubkey)
        .is_some())
}
