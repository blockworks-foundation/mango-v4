use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::Context;

use anchor_client::ClientError;
use anchor_lang::AccountDeserialize;

use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_sdk::account::{AccountSharedData, ReadableAccount};
use solana_sdk::pubkey::Pubkey;

use mango_v4::state::MangoAccountValue;

#[async_trait::async_trait(?Send)]
pub trait AccountFetcher: Sync + Send {
    async fn fetch_raw_account(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData>;
    async fn fetch_raw_account_lookup_table(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<AccountSharedData> {
        self.fetch_raw_account(address).await
    }
    async fn fetch_program_accounts(
        &self,
        program: &Pubkey,
        discriminator: [u8; 8],
    ) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>>;
}

// Can't be in the trait, since then it would no longer be object-safe...
pub async fn account_fetcher_fetch_anchor_account<T: AccountDeserialize>(
    fetcher: &dyn AccountFetcher,
    address: &Pubkey,
) -> anyhow::Result<T> {
    let account = fetcher.fetch_raw_account(address).await?;
    let mut data: &[u8] = &account.data();
    T::try_deserialize(&mut data)
        .with_context(|| format!("deserializing anchor account {}", address))
}

// Can't be in the trait, since then it would no longer be object-safe...
pub async fn account_fetcher_fetch_mango_account(
    fetcher: &dyn AccountFetcher,
    address: &Pubkey,
) -> anyhow::Result<MangoAccountValue> {
    let account = fetcher.fetch_raw_account(address).await?;
    let data: &[u8] = &account.data();
    MangoAccountValue::from_bytes(&data[8..])
        .with_context(|| format!("deserializing mango account {}", address))
}

pub struct RpcAccountFetcher {
    pub rpc: RpcClientAsync,
}

#[async_trait::async_trait(?Send)]
impl AccountFetcher for RpcAccountFetcher {
    async fn fetch_raw_account(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        self.rpc
            .get_account_with_commitment(address, self.rpc.commitment())
            .await
            .with_context(|| format!("fetch account {}", *address))?
            .value
            .ok_or(ClientError::AccountNotFound)
            .with_context(|| format!("fetch account {}", *address))
            .map(Into::into)
    }

    async fn fetch_program_accounts(
        &self,
        program: &Pubkey,
        discriminator: [u8; 8],
    ) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>> {
        use solana_account_decoder::UiAccountEncoding;
        use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
        use solana_client::rpc_filter::{Memcmp, RpcFilterType};
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                0,
                discriminator.to_vec(),
            ))]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(self.rpc.commitment()),
                ..RpcAccountInfoConfig::default()
            },
            with_context: Some(true),
        };
        let accs = self
            .rpc
            .get_program_accounts_with_config(program, config)
            .await?;
        // convert Account -> AccountSharedData
        Ok(accs
            .into_iter()
            .map(|(pk, acc)| (pk, acc.into()))
            .collect::<Vec<_>>())
    }
}

struct AccountCache {
    accounts: HashMap<Pubkey, AccountSharedData>,
    keys_for_program_and_discriminator: HashMap<(Pubkey, [u8; 8]), Vec<Pubkey>>,
}

impl AccountCache {
    fn clear(&mut self) {
        self.accounts.clear();
        self.keys_for_program_and_discriminator.clear();
    }
}

pub struct CachedAccountFetcher<T: AccountFetcher> {
    fetcher: T,
    cache: Mutex<AccountCache>,
}

impl<T: AccountFetcher> CachedAccountFetcher<T> {
    pub fn new(fetcher: T) -> Self {
        Self {
            fetcher,
            cache: Mutex::new(AccountCache {
                accounts: HashMap::new(),
                keys_for_program_and_discriminator: HashMap::new(),
            }),
        }
    }

    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}

#[async_trait::async_trait(?Send)]
impl<T: AccountFetcher> AccountFetcher for CachedAccountFetcher<T> {
    async fn fetch_raw_account(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(account) = cache.accounts.get(address) {
            return Ok(account.clone());
        }
        let account = self.fetcher.fetch_raw_account(address).await?;
        cache.accounts.insert(*address, account.clone());
        Ok(account)
    }

    async fn fetch_program_accounts(
        &self,
        program: &Pubkey,
        discriminator: [u8; 8],
    ) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>> {
        let cache_key = (*program, discriminator);
        let mut cache = self.cache.lock().unwrap();
        if let Some(accounts) = cache.keys_for_program_and_discriminator.get(&cache_key) {
            return Ok(accounts
                .iter()
                .map(|pk| (*pk, cache.accounts.get(&pk).unwrap().clone()))
                .collect::<Vec<_>>());
        }
        let accounts = self
            .fetcher
            .fetch_program_accounts(program, discriminator)
            .await?;
        cache
            .keys_for_program_and_discriminator
            .insert(cache_key, accounts.iter().map(|(pk, _)| *pk).collect());
        for (pk, acc) in accounts.iter() {
            cache.accounts.insert(*pk, acc.clone());
        }
        Ok(accounts)
    }
}
