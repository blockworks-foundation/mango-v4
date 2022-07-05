use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::{accounts_zerocopy::LoadZeroCopyRef, error::MangoError};

use super::{Bank, TokenIndex};

pub const MAX_BANKS: usize = 6;

// This struct describes which address lookup table can be used to pass
// the accounts that are relevant for this mint. The idea is that clients
// can load this account to figure out which address maps to use when calling
// instructions that need banks/oracles for all active positions.
#[account(zero_copy)]
#[derive(Debug)]
pub struct MintInfo {
    // TODO: none of these pubkeys are needed, remove?
    pub group: Pubkey,
    pub mint: Pubkey,
    pub banks: [Pubkey; MAX_BANKS],
    pub vaults: [Pubkey; MAX_BANKS],
    pub oracle: Pubkey,
    pub address_lookup_table: Pubkey,

    pub token_index: TokenIndex,

    // describe what address map relevant accounts are found on
    pub address_lookup_table_bank_index: u8,
    pub address_lookup_table_oracle_index: u8,

    pub reserved: [u8; 4],
}
const_assert_eq!(
    size_of::<MintInfo>(),
    MAX_BANKS * 2 * 32 + 4 * 32 + 2 + 2 + 4
);
const_assert_eq!(size_of::<MintInfo>() % 8, 0);

impl MintInfo {
    // used for health purposes
    pub fn first_bank(&self) -> Pubkey {
        self.banks[0]
    }

    pub fn first_vault(&self) -> Pubkey {
        self.vaults[0]
    }

    pub fn verify_banks_ais(&self, all_bank_ais: &[AccountInfo]) -> Result<()> {
        let total_banks = self
            .banks
            .iter()
            .filter(|bank| *bank != &Pubkey::default())
            .count();
        require_eq!(total_banks, all_bank_ais.len());

        for (idx, ai) in all_bank_ais.iter().enumerate() {
            match ai.load::<Bank>() {
                Ok(bank) => {
                    if self.token_index != bank.token_index
                        || self.group != bank.group
                        // todo: just below check should be enough, above 2 checks are superfluous and defensive
                        || self.banks[idx] != ai.key()
                    {
                        return Err(error!(MangoError::SomeError));
                    }
                }
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }
}
