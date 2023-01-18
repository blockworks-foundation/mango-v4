use crate::{AccountFetcher, MangoGroupContext};
use anyhow::Context;
use futures::{stream, StreamExt, TryStreamExt};
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::health::{FixedOrderAccountRetriever, HealthCache};
use mango_v4::state::MangoAccountValue;

pub async fn new(
    context: &MangoGroupContext,
    account_fetcher: &impl AccountFetcher,
    account: &MangoAccountValue,
) -> anyhow::Result<HealthCache> {
    let active_token_len = account.active_token_positions().count();
    let active_perp_len = account.active_perp_positions().count();

    let metas =
        context.derive_health_check_remaining_account_metas(account, vec![], vec![], vec![])?;
    let accounts: anyhow::Result<Vec<KeyedAccountSharedData>> = stream::iter(metas.iter())
        .then(|meta| async {
            Ok(KeyedAccountSharedData::new(
                meta.pubkey,
                account_fetcher.fetch_raw_account(&meta.pubkey).await?,
            ))
        })
        .try_collect()
        .await;

    let retriever = FixedOrderAccountRetriever {
        ais: accounts?,
        n_banks: active_token_len,
        n_perps: active_perp_len,
        begin_perp: active_token_len * 2,
        begin_serum3: active_token_len * 2 + active_perp_len * 2,
        staleness_slot: None,
    };
    mango_v4::health::new_health_cache(&account.borrow(), &retriever).context("make health cache")
}
