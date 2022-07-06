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
    pub admin: Pubkey,

    // ABI: Clients rely on this being at offset 40
    pub group_num: u32,

    pub padding: [u8; 4],

    pub insurance_vault: Pubkey,
    pub insurance_mint: Pubkey,

    pub bump: u8,
    // Only support closing/deregistering groups, stub oracles, tokens, and markets
    // if testing == 1
    pub testing: u8,
    pub padding2: [u8; 6],
    pub reserved: [u8; 8],
}
const_assert_eq!(size_of::<Group>(), 32 * 3 + 4 + 4 + 1 * 2 + 6 + 8);
const_assert_eq!(size_of::<Group>() % 8, 0);

#[macro_export]
macro_rules! group_seeds {
    ( $group:expr ) => {
        &[
            b"Group".as_ref(),
            $group.admin.as_ref(),
            &$group.group_num.to_le_bytes(),
            &[$group.bump],
        ]
    };
}

pub use group_seeds;
