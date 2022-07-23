use std::collections::HashMap;
use std::sync::Mutex;

use anchor_client::ClientError;

use anchor_lang::AccountDeserialize;

use solana_client::rpc_client::RpcClient;

use anyhow::Context;
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;

pub trait AccountFetcher: Sync + Send {
    fn fetch_raw_account(&self, address: Pubkey) -> anyhow::Result<Account>;
}

// Can't be in the trait, since then it would no longer be object-safe...
pub fn account_fetcher_fetch_anchor_account<T: AccountDeserialize>(
    fetcher: &dyn AccountFetcher,
    address: Pubkey,
) -> anyhow::Result<T> {
    let account = fetcher.fetch_raw_account(address)?;
    let mut data: &[u8] = &account.data;
    T::try_deserialize(&mut data).with_context(|| format!("deserializing account {}", address))
}

pub struct RpcAccountFetcher {
    pub rpc: RpcClient,
}

impl AccountFetcher for RpcAccountFetcher {
    fn fetch_raw_account(&self, address: Pubkey) -> anyhow::Result<Account> {
        self.rpc
            .get_account_with_commitment(&address, self.rpc.commitment())
            .with_context(|| format!("fetch account {}", address))?
            .value
            .ok_or(ClientError::AccountNotFound)
            .with_context(|| format!("fetch account {}", address))
    }
}

pub struct CachedAccountFetcher<T: AccountFetcher> {
    fetcher: T,
    cache: Mutex<HashMap<Pubkey, Account>>,
}

impl<T: AccountFetcher> CachedAccountFetcher<T> {
    pub fn new(fetcher: T) -> Self {
        Self {
            fetcher,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}

impl<T: AccountFetcher> AccountFetcher for CachedAccountFetcher<T> {
    fn fetch_raw_account(&self, address: Pubkey) -> anyhow::Result<Account> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(account) = cache.get(&address) {
            return Ok(account.clone());
        }
        let account = self.fetcher.fetch_raw_account(address)?;
        cache.insert(address, account.clone());
        Ok(account)
    }
}
