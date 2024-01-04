use crate::{AccountFetcher, MangoGroupContext};
use anyhow::Context;
use futures::{stream, StreamExt, TryStreamExt};
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::health::{FixedOrderAccountRetriever, HealthCache};
use mango_v4::state::MangoAccountValue;

use std::time::{SystemTime, UNIX_EPOCH};

pub async fn new(
    context: &MangoGroupContext,
    account_fetcher: &impl AccountFetcher,
    account: &MangoAccountValue,
) -> anyhow::Result<HealthCache> {
    let active_token_len = account.active_token_positions().count();
    let active_perp_len = account.active_perp_positions().count();

    let (metas, _health_cu) =
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
        begin_fallback_oracles: metas.len(), // TODO: add support for fallback oracle accounts
        usd_oracle_index: None,
        sol_oracle_index: None,
    };
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    mango_v4::health::new_health_cache(&account.borrow(), &retriever, now_ts)
        .context("make health cache")
}

pub fn new_sync(
    context: &MangoGroupContext,
    account_fetcher: &crate::chain_data::AccountFetcher,
    account: &MangoAccountValue,
) -> anyhow::Result<HealthCache> {
    let active_token_len = account.active_token_positions().count();
    let active_perp_len = account.active_perp_positions().count();

    let (metas, _health_cu) =
        context.derive_health_check_remaining_account_metas(account, vec![], vec![], vec![])?;
    let accounts = metas
        .iter()
        .map(|meta| {
            Ok(KeyedAccountSharedData::new(
                meta.pubkey,
                account_fetcher.fetch_raw(&meta.pubkey)?,
            ))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let retriever = FixedOrderAccountRetriever {
        ais: accounts,
        n_banks: active_token_len,
        n_perps: active_perp_len,
        begin_perp: active_token_len * 2,
        begin_serum3: active_token_len * 2 + active_perp_len * 2,
        staleness_slot: None,
        begin_fallback_oracles: metas.len(), // TODO: add support for fallback oracle accounts
        usd_oracle_index: None,
        sol_oracle_index: None,
    };
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    mango_v4::health::new_health_cache(&account.borrow(), &retriever, now_ts)
        .context("make health cache")
}
