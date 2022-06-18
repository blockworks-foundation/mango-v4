use std::{collections::HashMap, str::FromStr};

use crate::account_shared_data::KeyedAccountSharedData;

use client::MangoClient;
use mango_v4::accounts_zerocopy::LoadZeroCopy;
use mango_v4::state::{
    compute_health, oracle_price, Bank, FixedOrderAccountRetriever, HealthType, MangoAccount,
    MintInfo, PerpMarketIndex, TokenIndex,
};

use {
    crate::chain_data::ChainData, anyhow::Context, fixed::types::I80F48, log::*,
    solana_sdk::account::AccountSharedData, solana_sdk::pubkey::Pubkey,
};

pub fn load_mango_account<T: anchor_lang::ZeroCopy + anchor_lang::Owner>(
    account: &AccountSharedData,
) -> anyhow::Result<&T> {
    account.load::<T>().map_err(|e| e.into())
}

fn load_mango_account_from_chain<'a, T: anchor_lang::ZeroCopy + anchor_lang::Owner>(
    chain_data: &'a ChainData,
    pubkey: &Pubkey,
) -> anyhow::Result<&'a T> {
    load_mango_account::<T>(
        chain_data
            .account(pubkey)
            .context("retrieving account from chain")?,
    )
}

pub fn compute_health_(
    chain_data: &ChainData,
    mint_infos: &HashMap<TokenIndex, Pubkey>,
    perp_markets: &HashMap<PerpMarketIndex, Pubkey>,
    account: &MangoAccount,
    health_type: HealthType,
) -> anchor_lang::Result<I80F48> {
    let mut health_accounts = vec![];
    let mut banks = vec![];
    let mut oracles = vec![];

    // collect banks and oracles for active token positions
    for position in account.tokens.iter_active() {
        let mint_info = load_mango_account_from_chain::<MintInfo>(
            chain_data,
            mint_infos
                .get(&position.token_index)
                .expect("mint_infos cache missing entry"),
        )
        .unwrap();

        banks.push((
            mint_info.bank,
            chain_data
                .account(&mint_info.bank)
                .expect("chain data is missing bank"),
        ));
        oracles.push((
            mint_info.oracle,
            chain_data
                .account(&mint_info.oracle)
                .expect("chain data is missing oracle"),
        ));
    }

    // collect active perp markets
    let mut perp_markets = account
        .perps
        .iter_active_accounts()
        .map(|&s| {
            (
                *perp_markets
                    .get(&s.market_index)
                    .expect("perp markets cache is missing entry"),
                chain_data
                    .account(
                        perp_markets
                            .get(&s.market_index)
                            .expect("perp markets cache is missing entry"),
                    )
                    .expect("chain data is missing perp market"),
            )
        })
        .collect::<Vec<(Pubkey, &AccountSharedData)>>();
    let active_perp_len = perp_markets.len();

    // collect OO for active serum markets
    let mut serum_oos = account
        .serum3
        .iter_active()
        .map(|&s| (s.open_orders, chain_data.account(&s.open_orders).unwrap()))
        .collect::<Vec<(Pubkey, &AccountSharedData)>>();

    let active_token_len = banks.len();
    health_accounts.append(&mut banks);
    health_accounts.append(&mut oracles);
    health_accounts.append(&mut perp_markets);
    health_accounts.append(&mut serum_oos);

    let retriever = FixedOrderAccountRetriever {
        ais: health_accounts
            .into_iter()
            .map(|asd| KeyedAccountSharedData::new(asd.0, asd.1.clone()))
            .collect::<Vec<_>>(),
        n_banks: active_token_len,
        begin_perp: active_token_len * 2,
        begin_serum3: active_token_len * 2 + active_perp_len,
    };
    compute_health(account, health_type, &retriever)
}

#[allow(clippy::too_many_arguments)]
pub fn process_accounts<'a>(
    mango_client: &MangoClient,
    chain_data: &ChainData,
    accounts: impl Iterator<Item = &'a Pubkey>,
    mint_infos: &HashMap<TokenIndex, Pubkey>,
    perp_markets: &HashMap<PerpMarketIndex, Pubkey>,
) -> anyhow::Result<()> {
    for pubkey in accounts {
        let account_result = load_mango_account_from_chain::<MangoAccount>(chain_data, pubkey);
        let account = match account_result {
            Ok(account) => account,
            Err(err) => {
                warn!("could not load account {}: {:?}", pubkey, err);
                continue;
            }
        };

        // compute maint health for account
        let maint_health = compute_health_(
            chain_data,
            mint_infos,
            perp_markets,
            account,
            HealthType::Maint,
        )
        .expect("always ok");

        // try liquidating
        if maint_health.is_negative() {
            // find asset and liab tokens
            let mut tokens = account
                .tokens
                .iter_active()
                .map(|token| {
                    let mint_info_pk = mint_infos.get(&token.token_index).expect("always Ok");
                    let mint_info =
                        load_mango_account_from_chain::<MintInfo>(chain_data, mint_info_pk)?;
                    let bank = load_mango_account_from_chain::<Bank>(chain_data, &mint_info.bank)?;
                    let oracle = chain_data.account(&mint_info.oracle)?;
                    let price = oracle_price(
                        &KeyedAccountSharedData::new(mint_info.oracle, oracle.clone()),
                        bank.mint_decimals,
                    )?;
                    Ok((token.token_index, bank, token.native(bank) * price))
                })
                .collect::<anyhow::Result<Vec<(TokenIndex, &Bank, I80F48)>>>()?;
            tokens.sort_by(|a, b| a.2.cmp(&b.2));
            if tokens.len() < 2 {
                anyhow::bail!(format!(
                    "mango account {}, has less than 2 active tokens",
                    pubkey
                ));
            }
            let (asset_token_index, _asset_bank, _asset_price) = tokens.last().unwrap();
            let (liab_token_index, _liab_bank, _liab_price) = tokens.first().unwrap();

            //
            // TODO: log liqor's assets in UI form
            // TODO: log liquee's liab_needed, need to refactor program code to be able to be accessed from client side
            // TODO: swap inherited liabs to desired asset for liqor
            //
            let sig = mango_client.liq_token_with_token(
                (pubkey, account),
                *asset_token_index,
                *liab_token_index,
                {
                    // max liab liqor can provide
                    // let fresh_liqor = load_mango_account_from_chain::<MangoAccount>(
                    //     chain_data,
                    //     &mango_client.mango_account_cache.0,
                    // )?;
                    // fresh_liqor
                    //     .tokens
                    //     .find(*liab_token_index)
                    //     .unwrap()
                    //     .native(&_liab_bank)
                    I80F48::from_str("0.0000001").unwrap()
                },
            );
            match sig {
                Ok(sig) => log::info!(
                    "Liquidated {}..., maint_health was {}, tx sig {:?}",
                    &pubkey.to_string()[..3],
                    maint_health,
                    sig
                ),
                Err(err) => {
                    log::error!("{:?}", err)
                }
            }
        }
    }

    Ok(())
}
