use mango_v4::accounts_zerocopy::{AccountReader, KeyedAccountReader};
use solana_sdk::{account::AccountSharedData, pubkey::Pubkey};

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
        AccountReader::owner(&self.data)
    }

    fn data(&self) -> &[u8] {
        AccountReader::data(&self.data)
    }
}

impl KeyedAccountReader for KeyedAccountSharedData {
    fn key(&self) -> &Pubkey {
        &self.key
    }
}
