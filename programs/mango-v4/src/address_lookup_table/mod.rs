mod solana_address_lookup_table_instruction;
pub use solana_address_lookup_table_instruction::*;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

pub fn id() -> Pubkey {
    Pubkey::from_str(&"AddressLookupTab1e1111111111111111111111111").unwrap()
}

/// The maximum number of addresses that a lookup table can hold
pub const LOOKUP_TABLE_MAX_ADDRESSES: usize = 256;

/// The serialized size of lookup table metadata
pub const LOOKUP_TABLE_META_SIZE: usize = 56;

pub const LOOKUP_TABLE_MAX_ACCOUNT_SIZE: usize =
    LOOKUP_TABLE_META_SIZE + LOOKUP_TABLE_MAX_ADDRESSES * solana_program::pubkey::PUBKEY_BYTES;
