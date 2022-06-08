use std::{collections::HashMap, str::FromStr};

use crate::{cm, health::FixedOrderAccountRetrieverForAccountSharedData};

use arrayref::array_ref;
use client::MangoClient;
use mango_v4::state::{
    determine_oracle_type, Bank, HealthType, MangoAccount, MintInfo, OracleType, PerpMarketIndex,
    StubOracle, TokenIndex, QUOTE_DECIMALS,
};

use {
    crate::chain_data::ChainData,
    anyhow::Context,
    fixed::types::I80F48,
    log::*,
    solana_sdk::account::{AccountSharedData, ReadableAccount},
    solana_sdk::pubkey::Pubkey,
};

// FUTURE: It'd be very nice if I could map T to the DataType::T constant!
pub fn load_mango_account<
    T: anchor_lang::AccountDeserialize + anchor_lang::Discriminator + bytemuck::Pod,
>(
    account: &AccountSharedData,
) -> anyhow::Result<&T> {
    let data = account.data();

    let disc_bytes = array_ref![data, 0, 8];
    if disc_bytes != &T::discriminator() {
        anyhow::bail!(
            "unexpected disc expected {:?}, got {:?}",
            T::discriminator(),
            disc_bytes
        );
    }

    if data.len() != 8 + std::mem::size_of::<T>() {
        anyhow::bail!(
            "bad account size expected {}, got {}",
            8 + std::mem::size_of::<T>(),
            data.len(),
        );
    }

    return Ok(bytemuck::try_from_bytes(&data[8..]).expect("always Ok"));
}

fn load_mango_account_from_chain<
    'a,
    T: anchor_lang::AccountDeserialize + anchor_lang::Discriminator + bytemuck::Pod,
>(
    chain_data: &'a ChainData,
    pubkey: &Pubkey,
) -> anyhow::Result<&'a T> {
    load_mango_account::<T>(
        chain_data
            .account(pubkey)
            .context("retrieving account from chain")?,
    )
}

pub fn compute_health(
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

    // collect OO for active serum markets
    let mut serum_oos = account
        .serum3
        .iter_active()
        .map(|&s| (s.open_orders, chain_data.account(&s.open_orders).unwrap()))
        .collect::<Vec<(Pubkey, &AccountSharedData)>>();
    let serum_len = serum_oos.len();

    // collect active perp markets
    let mut perp_markets_ = account
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

    let banks_len = banks.len();
    let oracles_len = oracles.len();
    health_accounts.append(&mut banks);
    health_accounts.append(&mut oracles);
    health_accounts.append(&mut serum_oos);
    health_accounts.append(&mut perp_markets_);

    let retriever = FixedOrderAccountRetrieverForAccountSharedData {
        ais: &health_accounts[..],
        n_banks: banks_len,
        begin_serum3: cm!(banks_len + oracles_len),
        begin_perp: cm!(banks_len + oracles_len + serum_len),
    };

    let health_cache = crate::health::compute_health_detail(account, &retriever, health_type, true)
        .expect("error building health cache");

    health_cache.health(health_type)
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

        let maint_health = compute_health(
            chain_data,
            mint_infos,
            perp_markets,
            account,
            HealthType::Maint,
        )
        .expect("always ok");

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
                    // TODO: refactor oracle.rs in program code to work on just plain &[u8]] and not just AccountInfo
                    let price = {
                        let oracle_type = determine_oracle_type(oracle.data())?;
                        match oracle_type {
                            OracleType::Stub => load_mango_account::<StubOracle>(oracle)?.price,
                            OracleType::Pyth => {
                                let price_struct =
                                    pyth_sdk_solana::load_price(oracle.data()).unwrap();
                                let price = I80F48::from_num(price_struct.price);
                                let decimals = (price_struct.expo as i32)
                                    .checked_add(QUOTE_DECIMALS)
                                    .unwrap()
                                    .checked_sub(bank.mint_decimals as i32)
                                    .unwrap();
                                let decimal_adj =
                                    I80F48::from_num(10_u32.pow(decimals.abs() as u32));
                                if decimals < 0 {
                                    cm!(price / decimal_adj)
                                } else {
                                    cm!(price * decimal_adj)
                                }
                            }
                        }
                    };
                    Ok((token.token_index, bank, token.native(bank) * price))
                })
                .collect::<anyhow::Result<Vec<(TokenIndex, &Bank, I80F48)>>>()?;
            tokens.sort_by(|a, b| a.2.cmp(&b.2));
            if tokens.len() < 2 {
                continue;
            }
            let (asset_token_index, _asset_bank, _asset_price) = tokens.last().unwrap();
            let (liab_token_index, _liab_bank, _liab_price) = tokens.first().unwrap();

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
