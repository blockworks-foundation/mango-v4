use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;
use std::mem::size_of;

// TODO: Assuming we allow up to 65536 different tokens
pub type TokenIndex = u16;

// TODO: Should we call this `Group` instead of `Group`? And `Account` instead of `MangoAccount`?
#[account(zero_copy)]
pub struct Group {
    // Relying on Anchor's discriminator be sufficient for our versioning needs?
    // pub meta_data: MetaData,
    pub admin: Pubkey,

    pub bump: u8,
    pub reserved: [u8; 7],
}
const_assert_eq!(size_of::<Group>(), 40);
const_assert_eq!(size_of::<Group>() % 8, 0);

#[macro_export]
macro_rules! group_seeds {
    ( $group:expr ) => {
        &[b"Group".as_ref(), $group.admin.as_ref(), &[$group.bump]]
    };
}

pub use group_seeds;
