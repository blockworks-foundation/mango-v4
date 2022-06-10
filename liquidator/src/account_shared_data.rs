use mango_v4::accounts_zerocopy::{AccountReader, KeyedAccountReader};
use solana_sdk::{
    account::{AccountSharedData, ReadableAccount},
    pubkey::Pubkey,
};

#[derive(Clone)]
pub struct KeyedAccountSharedData {
    pub key: Pubkey,
    pub data: AccountSharedData,
}

impl KeyedAccountSharedData {
    pub fn new(key: Pubkey, data: AccountSharedData) -> Self {
        Self { key, data }
    }
}

impl AccountReader for KeyedAccountSharedData {
    fn owner(&self) -> &Pubkey {
        self.data.owner()
    }

    fn data(&self) -> &[u8] {
        self.data.data()
    }
}

impl KeyedAccountReader for KeyedAccountSharedData {
    fn key(&self) -> &Pubkey {
        &self.key
    }
}

impl AccountReader for AccountSharedData {
    fn owner(&self) -> &Pubkey {
        self.owner()
    }

    fn data(&self) -> &[u8] {
        self.data()
    }
}
