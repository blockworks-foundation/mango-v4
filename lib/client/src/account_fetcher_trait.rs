use solana_sdk::account::AccountSharedData;
use solana_sdk::pubkey::Pubkey;


#[async_trait::async_trait]
pub trait AccountFetcher2: Sync + Send {
    async fn fetch_raw_account(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData>;
    async fn fetch_program_accounts(
        &self,
        program: &Pubkey,
        discriminator: [u8; 8],
    ) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>>;
}

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

