use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;
use std::mem::size_of;

// TODO: Assuming we allow up to 65536 different tokens
pub type TokenIndex = u16;
pub const QUOTE_TOKEN_INDEX: TokenIndex = 0;

#[account(zero_copy)]
#[derive(Debug)]
pub struct Group {
    // ABI: Clients rely on this being at offset 8
    pub creator: Pubkey,

    // ABI: Clients rely on this being at offset 40
    pub group_num: u32,

    pub admin: Pubkey,

    // TODO: unused, use case - listing shit tokens with conservative parameters (mostly defaults)
    pub fast_listing_admin: Pubkey,

    pub padding: [u8; 4],

    pub insurance_vault: Pubkey,
    pub insurance_mint: Pubkey,

    pub bump: u8,

    pub testing: u8,

    pub version: u8,

    pub padding2: [u8; 5],
    pub reserved: [u8; 8],
}
const_assert_eq!(size_of::<Group>(), 32 * 5 + 4 + 4 + 1 * 2 + 6 + 8);
const_assert_eq!(size_of::<Group>() % 8, 0);

impl Group {
    pub fn is_testing(&self) -> bool {
        self.testing == 1
    }

    pub fn multiple_banks_supported(&self) -> bool {
        self.is_testing() || self.version > 0
    }

    pub fn serum3_supported(&self) -> bool {
        self.is_testing() || self.version > 0
    }

    pub fn perps_supported(&self) -> bool {
        self.is_testing() || self.version > 0
    }
}

// note: using creator instead of admin, since admin can be changed
#[macro_export]
macro_rules! group_seeds {
    ( $group:expr ) => {
        &[
            b"Group".as_ref(),
            $group.creator.as_ref(),
            &$group.group_num.to_le_bytes(),
            &[$group.bump],
        ]
    };
}

pub use group_seeds;
