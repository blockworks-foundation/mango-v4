use anyhow::Context;
use solana_sdk::account::ReadableAccount;
use solana_sdk::pubkey::Pubkey;
use mango_v4::state::MangoAccountValue;
use crate::AccountFetcher;

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
