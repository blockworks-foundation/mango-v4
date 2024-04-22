use anchor_lang::{AccountDeserialize, Discriminator};
use futures::{stream, StreamExt};
use mango_v4::state::{
    Bank, MangoAccount, MangoAccountValue, MintInfo, OpenbookV2Market, PerpMarket, Serum3Market,
};

use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::pubkey::Pubkey;

pub async fn fetch_mango_accounts(
    rpc: &RpcClientAsync,
    program: Pubkey,
    group: Pubkey,
    owner: Pubkey,
) -> anyhow::Result<Vec<(Pubkey, MangoAccountValue)>> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                0,
                MangoAccount::discriminator().to_vec(),
            )),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(8, group.to_bytes().to_vec())),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(40, owner.to_bytes().to_vec())),
        ]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    rpc.get_program_accounts_with_config(&program, config)
        .await?
        .into_iter()
        .map(|(key, account)| Ok((key, MangoAccountValue::from_bytes(&account.data[8..])?)))
        .collect::<Result<Vec<_>, _>>()
}

pub async fn fetch_anchor_account<T: AccountDeserialize>(
    rpc: &RpcClientAsync,
    address: &Pubkey,
) -> anyhow::Result<T> {
    let account = rpc.get_account(address).await?;
    Ok(T::try_deserialize(&mut (&account.data as &[u8]))?)
}

async fn fetch_anchor_accounts<T: AccountDeserialize + Discriminator>(
    rpc: &RpcClientAsync,
    program: Pubkey,
    filters: Vec<RpcFilterType>,
) -> anyhow::Result<Vec<(Pubkey, T)>> {
    let account_type_filter =
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, T::discriminator().to_vec()));
    let config = RpcProgramAccountsConfig {
        filters: Some([vec![account_type_filter], filters].concat()),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    rpc.get_program_accounts_with_config(&program, config)
        .await?
        .into_iter()
        .map(|(key, account)| Ok((key, T::try_deserialize(&mut (&account.data as &[u8]))?)))
        .collect()
}

pub async fn fetch_banks(
    rpc: &RpcClientAsync,
    program: Pubkey,
    group: Pubkey,
) -> anyhow::Result<Vec<(Pubkey, Bank)>> {
    fetch_anchor_accounts::<Bank>(
        rpc,
        program,
        vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8,
            group.to_bytes().to_vec(),
        ))],
    )
    .await
}

pub async fn fetch_mint_infos(
    rpc: &RpcClientAsync,
    program: Pubkey,
    group: Pubkey,
) -> anyhow::Result<Vec<(Pubkey, MintInfo)>> {
    fetch_anchor_accounts::<MintInfo>(
        rpc,
        program,
        vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8,
            group.to_bytes().to_vec(),
        ))],
    )
    .await
}

pub async fn fetch_serum3_markets(
    rpc: &RpcClientAsync,
    program: Pubkey,
    group: Pubkey,
) -> anyhow::Result<Vec<(Pubkey, Serum3Market)>> {
    fetch_anchor_accounts::<Serum3Market>(
        rpc,
        program,
        vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8,
            group.to_bytes().to_vec(),
        ))],
    )
    .await
}

pub async fn fetch_openbook_v2_markets(
    rpc: &RpcClientAsync,
    program: Pubkey,
    group: Pubkey,
) -> anyhow::Result<Vec<(Pubkey, OpenbookV2Market)>> {
    fetch_anchor_accounts::<OpenbookV2Market>(
        rpc,
        program,
        vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8,
            group.to_bytes().to_vec(),
        ))],
    )
    .await
}

pub async fn fetch_perp_markets(
    rpc: &RpcClientAsync,
    program: Pubkey,
    group: Pubkey,
) -> anyhow::Result<Vec<(Pubkey, PerpMarket)>> {
    fetch_anchor_accounts::<PerpMarket>(
        rpc,
        program,
        vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8,
            group.to_bytes().to_vec(),
        ))],
    )
    .await
}

pub async fn fetch_multiple_accounts(
    rpc: &RpcClientAsync,
    keys: &[Pubkey],
) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>> {
    let config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        ..RpcAccountInfoConfig::default()
    };
    Ok(rpc
        .get_multiple_accounts_with_config(keys, config)
        .await?
        .value
        .into_iter()
        .zip(keys.iter())
        .filter(|(maybe_acc, _)| maybe_acc.is_some())
        .map(|(acc, key)| (*key, acc.unwrap().into()))
        .collect())
}

/// Fetch multiple account using one request per chunk of `max_chunk_size` accounts
/// Can execute in parallel up to `parallel_rpc_requests`
///
/// WARNING: some accounts requested may be missing from the result
pub async fn fetch_multiple_accounts_in_chunks(
    rpc: &RpcClientAsync,
    keys: &[Pubkey],
    max_chunk_size: usize,
    parallel_rpc_requests: usize,
) -> anyhow::Result<Vec<(Pubkey, Account)>> {
    let config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        ..RpcAccountInfoConfig::default()
    };

    let raw_results = stream::iter(keys)
        .chunks(max_chunk_size)
        .map(|keys| {
            let account_info_config = config.clone();
            async move {
                let keys = keys.iter().map(|x| **x).collect::<Vec<Pubkey>>();
                let req_res = rpc
                    .get_multiple_accounts_with_config(&keys, account_info_config)
                    .await;

                match req_res {
                    Ok(v) => Ok(keys.into_iter().zip(v.value).collect::<Vec<_>>()),
                    Err(e) => Err(e),
                }
            }
        })
        .buffer_unordered(parallel_rpc_requests)
        .collect::<Vec<_>>()
        .await;

    let result = raw_results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .filter_map(|(pubkey, account_opt)| account_opt.map(|acc| (pubkey, acc)))
        .collect::<Vec<_>>();

    Ok(result)
}
