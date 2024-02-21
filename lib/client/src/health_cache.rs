use crate::{AccountFetcher, FallbackOracleConfig, MangoGroupContext};
use anyhow::Context;
use futures::{stream, StreamExt, TryStreamExt};
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::health::{FixedOrderAccountRetriever, HealthCache};
use mango_v4::state::{pyth_mainnet_sol_oracle, pyth_mainnet_usdc_oracle, MangoAccountValue};

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn new(
    context: &MangoGroupContext,
    fallback_config: &FallbackOracleConfig,
    account_fetcher: &dyn AccountFetcher,
    account: &MangoAccountValue,
) -> anyhow::Result<HealthCache> {
    let active_token_len = account.active_token_positions().count();
    let active_perp_len = account.active_perp_positions().count();

    let fallback_keys = context
        .derive_fallback_oracle_keys(fallback_config, account_fetcher)
        .await?;
    let (metas, _health_cu) = context.derive_health_check_remaining_account_metas(
        account,
        vec![],
        vec![],
        vec![],
        fallback_keys,
    )?;
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
        begin_fallback_oracles: metas.len(),
        usdc_oracle_index: metas
            .iter()
            .position(|m| m.pubkey == pyth_mainnet_usdc_oracle::ID),
        sol_oracle_index: metas
            .iter()
            .position(|m| m.pubkey == pyth_mainnet_sol_oracle::ID),
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

    let (metas, _health_cu) = context.derive_health_check_remaining_account_metas(
        account,
        vec![],
        vec![],
        vec![],
        HashMap::new(),
    )?;
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
        begin_fallback_oracles: metas.len(),
        usdc_oracle_index: None,
        sol_oracle_index: None,
    };
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    mango_v4::health::new_health_cache(&account.borrow(), &retriever, now_ts)
        .context("make health cache")
}
