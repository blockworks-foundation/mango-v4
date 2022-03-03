use anchor_lang::prelude::*;

// This struct describes which address lookup table can be used to pass
// the accounts that are relevant for this mint. The idea is that clients
// can load this account to figure out which address maps to use when calling
// instructions that need banks/oracles for all active positions.
#[account(zero_copy)]
pub struct MintInfo {
    // TODO: none of these pubkeys are needed, remove?
    pub mint: Pubkey,
    pub bank: Pubkey,
    pub vault: Pubkey,
    pub oracle: Pubkey,

    // describe what address map relevant accounts are found on
    pub address_lookup_table: Pubkey,
    pub address_lookup_table_bank_index: u8,
    pub address_lookup_table_oracle_index: u8,
}
