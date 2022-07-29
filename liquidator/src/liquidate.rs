use crate::account_shared_data::KeyedAccountSharedData;
use crate::ChainDataAccountFetcher;

use client::{AccountFetcher, MangoClient, MangoClientError, MangoGroupContext};
use mango_v4::state::{
    new_health_cache, oracle_price, Bank, FixedOrderAccountRetriever, HealthCache, HealthType,
    MangoAccountValue, TokenIndex,
};

use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

pub fn new_health_cache_(
    context: &MangoGroupContext,
    account_fetcher: &ChainDataAccountFetcher,
    account: &MangoAccountValue,
) -> anyhow::Result<HealthCache> {
    let active_token_len = account.token_iter_active().count();
    let active_perp_len = account.perp_iter_active_accounts().count();

    let metas = context.derive_health_check_remaining_account_metas(account, None, false)?;
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
        begin_perp: active_token_len * 2,
        begin_serum3: active_token_len * 2 + active_perp_len,
    };
    new_health_cache(&account.borrow(), &retriever).context("make health cache")
}

#[allow(clippy::too_many_arguments)]
pub fn process_account(
    mango_client: &MangoClient,
    account_fetcher: &ChainDataAccountFetcher,
    pubkey: &Pubkey,
) -> anyhow::Result<()> {
    // TODO: configurable
    let min_health_ratio = I80F48::from_num(50.0f64);
    let quote_token_index = 0;

    let account = account_fetcher.fetch_mango_account(pubkey)?;
    let maint_health = new_health_cache_(&mango_client.context, account_fetcher, &account)
        .expect("always ok")
        .health(HealthType::Maint);

    if maint_health >= 0 && !account.is_bankrupt() {
        return Ok(());
    }

    log::trace!(
        "possible candidate: {}, with owner: {}, maint health: {}, bankrupt: {}",
        pubkey,
        account.fixed.owner,
        maint_health,
        account.is_bankrupt(),
    );

    // Fetch a fresh account and re-compute
    // This is -- unfortunately -- needed because the websocket streams seem to not
    // be great at providing timely updates to the account data.
    let account = account_fetcher.fetch_fresh_mango_account(pubkey)?;
    let maint_health = new_health_cache_(&mango_client.context, account_fetcher, &account)
        .expect("always ok")
        .health(HealthType::Maint);

    // find asset and liab tokens
    let mut tokens = account
        .token_iter_active()
        .map(|token_position| {
            let token = mango_client.context.token(token_position.token_index);
            let bank = account_fetcher.fetch::<Bank>(&token.mint_info.first_bank())?;
            let oracle = account_fetcher.fetch_raw_account(token.mint_info.oracle)?;
            let price = oracle_price(
                &KeyedAccountSharedData::new(token.mint_info.oracle, oracle.into()),
                bank.oracle_config.conf_filter,
                bank.mint_decimals,
            )?;
            Ok((
                token_position.token_index,
                bank,
                token_position.native(&bank) * price,
            ))
        })
        .collect::<anyhow::Result<Vec<(TokenIndex, Bank, I80F48)>>>()?;
    tokens.sort_by(|a, b| a.2.cmp(&b.2));

    let get_max_liab_transfer = |source, target| -> anyhow::Result<I80F48> {
        let mut liqor = account_fetcher
            .fetch_fresh_mango_account(&mango_client.mango_account_address)
            .context("getting liquidator account")?;

        // Ensure the tokens are activated, so they appear in the health cache and
        // max_swap_source() will work.
        liqor.token_get_mut_or_create(source)?;
        liqor.token_get_mut_or_create(target)?;

        let health_cache =
            new_health_cache_(&mango_client.context, account_fetcher, &liqor).expect("always ok");
        let amount = health_cache
            .max_swap_source_for_health_ratio(source, target, min_health_ratio)
            .context("getting max_swap_source")?;
        Ok(amount)
    };

    // try liquidating
    if account.is_bankrupt() {
        if tokens.is_empty() {
            anyhow::bail!("mango account {}, is bankrupt has no active tokens", pubkey);
        }
        let (liab_token_index, _liab_bank, _liab_price) = tokens.first().unwrap();

        let max_liab_transfer = get_max_liab_transfer(*liab_token_index, quote_token_index)?;

        let sig = mango_client
            .liq_token_bankruptcy((pubkey, &account), *liab_token_index, max_liab_transfer)
            .context("sending liq_token_bankruptcy")?;
        log::info!(
            "Liquidated bankruptcy for {}..., maint_health was {}, tx sig {:?}",
            &pubkey.to_string()[..3],
            maint_health,
            sig
        );
    } else if maint_health.is_negative() {
        if tokens.len() < 2 {
            anyhow::bail!("mango account {}, has less than 2 active tokens", pubkey);
        }
        let (asset_token_index, _asset_bank, _asset_price) = tokens.last().unwrap();
        let (liab_token_index, _liab_bank, _liab_price) = tokens.first().unwrap();

        let max_liab_transfer = get_max_liab_transfer(*liab_token_index, *asset_token_index)
            .context("getting max_liab_transfer")?;

        //
        // TODO: log liqor's assets in UI form
        // TODO: log liquee's liab_needed, need to refactor program code to be able to be accessed from client side
        // TODO: swap inherited liabs to desired asset for liqor
        //
        let sig = mango_client
            .liq_token_with_token(
                (pubkey, &account),
                *asset_token_index,
                *liab_token_index,
                max_liab_transfer,
            )
            .context("sending liq_token_with_token")?;
        log::info!(
            "Liquidated token with token for {}..., maint_health was {}, tx sig {:?}",
            &pubkey.to_string()[..3],
            maint_health,
            sig
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn process_accounts<'a>(
    mango_client: &MangoClient,
    account_fetcher: &ChainDataAccountFetcher,
    accounts: impl Iterator<Item = &'a Pubkey>,
) -> anyhow::Result<()> {
    for pubkey in accounts {
        match process_account(mango_client, account_fetcher, pubkey) {
            Err(err) => {
                // Not all errors need to be raised to the user's attention.
                let mut log_level = log::Level::Error;

                // Simulation errors due to liqee precondition failures on the liquidation instructions
                // will commonly happen if our liquidator is late or if there are chain forks.
                match err.downcast_ref::<MangoClientError>() {
                    Some(MangoClientError::SendTransactionPreflightFailure { logs }) => {
                        if logs.contains("HealthMustBeNegative") || logs.contains("IsNotBankrupt") {
                            log_level = log::Level::Trace;
                        }
                    }
                    _ => {}
                };
                log::log!(log_level, "liquidating account {}: {:?}", pubkey, err);
            }
            _ => {}
        };
    }

    Ok(())
}
