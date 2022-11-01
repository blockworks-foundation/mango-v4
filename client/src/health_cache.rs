use crate::{AccountFetcher, MangoGroupContext};
use anyhow::Context;
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::state::{FixedOrderAccountRetriever, HealthCache, MangoAccountValue};

pub fn new(
    context: &MangoGroupContext,
    account_fetcher: &impl AccountFetcher,
    account: &MangoAccountValue,
) -> anyhow::Result<HealthCache> {
    let active_token_len = account.active_token_positions().count();
    let active_perp_len = account.active_perp_positions().count();

    let metas = context.derive_health_check_remaining_account_metas(account, vec![], false)?;
    let accounts = metas
        .iter()
        .map(|meta| {
            Ok(KeyedAccountSharedData::new(
                meta.pubkey,
                account_fetcher.fetch_raw_account(&meta.pubkey)?,
            ))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let retriever = FixedOrderAccountRetriever {
        ais: accounts,
        n_banks: active_token_len,
        n_perps: active_perp_len,
        begin_perp: active_token_len * 2,
        begin_serum3: active_token_len * 2 + active_perp_len,
    };
    mango_v4::state::new_health_cache(&account.borrow(), &retriever).context("make health cache")
}
