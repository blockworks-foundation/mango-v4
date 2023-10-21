use mango_feeds_connector::account_fetcher::AccountFetcherFeeds;
use mango_feeds_connector::account_fetchers::{CachedAccountFetcher, RpcAccountFetcher};
use solana_sdk::account::AccountSharedData;
use solana_sdk::pubkey::Pubkey;


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
        self.feeds_fetch_raw_account(address).await
            .map(|(acc, _slot)| acc)
    }

    async fn fetch_raw_account_lookup_table(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        self.fetch_raw_account(address).await
    }

    async fn fetch_program_accounts(&self, program: &Pubkey, discriminator: [u8; 8]) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>> {
        self.feeds_fetch_program_accounts(program, discriminator).await
            .map(|(accs, _slot)| accs)
    }
}

#[async_trait::async_trait]
impl<T: AccountFetcherFeeds + 'static> AccountFetcher for CachedAccountFetcher<T> {
    async fn fetch_raw_account(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        self.feeds_fetch_raw_account(address).await
            .map(|(acc, _slot)| acc)
    }

    async fn fetch_raw_account_lookup_table(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        self.fetch_raw_account(address).await
    }

    async fn fetch_program_accounts(&self, program: &Pubkey, discriminator: [u8; 8]) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>> {
        self.feeds_fetch_program_accounts(program, discriminator).await
            .map(|(accs, _slot)| accs)
    }
}