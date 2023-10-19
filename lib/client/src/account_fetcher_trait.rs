use solana_sdk::account::AccountSharedData;
use solana_sdk::pubkey::Pubkey;
use crate::account_fetchers::{AccountFetcherFeeds, CachedAccountFetcher, RpcAccountFetcher};


#[async_trait::async_trait]
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

#[async_trait::async_trait]
impl AccountFetcher for RpcAccountFetcher {
    async fn fetch_raw_account(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        self.fetch_raw_account(address).await
    }

    async fn fetch_raw_account_lookup_table(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        self.fetch_raw_account_lookup_table(address).await
    }

    async fn fetch_program_accounts(&self, program: &Pubkey, discriminator: [u8; 8]) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>> {
        self.fetch_program_accounts(program, discriminator).await
    }
}

#[async_trait::async_trait]
impl<T: AccountFetcherFeeds> AccountFetcher for CachedAccountFetcher<T> {
    async fn fetch_raw_account(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        self.fetch_raw_account(address).await
    }

    async fn fetch_raw_account_lookup_table(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        self.fetch_raw_account_lookup_table(address).await
    }

    async fn fetch_program_accounts(&self, program: &Pubkey, discriminator: [u8; 8]) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>> {
        self.fetch_program_accounts(program, discriminator).await
    }
}