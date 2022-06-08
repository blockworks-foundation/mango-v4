use mango_v4::accounts_zerocopy::{AccountReader, KeyedAccountReader};
use solana_sdk::{
    account::{AccountSharedData, ReadableAccount},
    pubkey::Pubkey,
};

/// A Ref to an AccountSharedData - makes AccountSharedData compatible with AccountReader
pub struct AccountSharedDataRef<'a> {
    pub key: Pubkey,
    pub owner: &'a Pubkey,
    pub data: &'a [u8],
}

impl<'a> AccountSharedDataRef<'a> {
    pub fn borrow(key: Pubkey, asd: &'a AccountSharedData) -> anchor_lang::Result<Self> {
        Ok(Self {
            key,
            owner: asd.owner(),
            data: asd.data(),
        })
    }
}

impl<'a> AccountReader for AccountSharedDataRef<'a> {
    fn owner(&self) -> &Pubkey {
        self.owner
    }

    fn data(&self) -> &[u8] {
        &self.data
    }
}

impl<'a> KeyedAccountReader for AccountSharedDataRef<'a> {
    fn key(&self) -> &Pubkey {
        &self.key
    }
}
