use jsonrpc_core_client::transports::http;

use solana_account_decoder::{UiAccount, UiAccountEncoding};
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_response::{Response, RpcKeyedAccount},
};
use solana_rpc::{rpc::rpc_full::FullClient, rpc::OptionalContext};
use solana_sdk::{account::AccountSharedData, commitment_config::CommitmentConfig, pubkey::Pubkey};

use anyhow::Context;
use futures::{stream, StreamExt};
use log::*;
use std::str::FromStr;
use tokio::time;

use crate::{healthcheck, AnyhowWrap, Config};

#[derive(Clone)]
pub struct AccountUpdate {
    pub pubkey: Pubkey,
    pub slot: u64,
    pub account: AccountSharedData,
}

#[derive(Clone, Default)]
pub struct AccountSnapshot {
    pub accounts: Vec<AccountUpdate>,
}

impl AccountSnapshot {
    pub fn extend_from_gpa_rpc(
        &mut self,
        rpc: Response<Vec<RpcKeyedAccount>>,
    ) -> anyhow::Result<()> {
        self.accounts.reserve(rpc.value.len());
        for a in rpc.value {
            self.accounts.push(AccountUpdate {
                slot: rpc.context.slot,
                pubkey: Pubkey::from_str(&a.pubkey).unwrap(),
                account: a
                    .account
                    .decode()
                    .ok_or(anyhow::anyhow!("could not decode account"))?,
            });
        }
        Ok(())
    }

    pub fn extend_from_gma_rpc(
        &mut self,
        keys: &[Pubkey],
        rpc: Response<Vec<Option<UiAccount>>>,
    ) -> anyhow::Result<()> {
        self.accounts.reserve(rpc.value.len());
        for (&pubkey, a) in keys.iter().zip(rpc.value.iter()) {
            if let Some(ui_account) = a {
                self.accounts.push(AccountUpdate {
                    slot: rpc.context.slot,
                    pubkey,
                    account: ui_account
                        .decode()
                        .ok_or(anyhow::anyhow!("could not decode account"))?,
                });
            }
        }
        Ok(())
    }
}

async fn feed_snapshots(
    config: &Config,
    sender: &async_channel::Sender<AccountSnapshot>,
) -> anyhow::Result<()> {
    let mango_program_id = Pubkey::from_str(&config.mango_program_id)?;

    let rpc_client = http::connect_with_options::<FullClient>(&config.rpc_http_url, true)
        .await
        .map_err_anyhow()?;

    let account_info_config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        commitment: Some(CommitmentConfig::processed()),
        data_slice: None,
    };
    let all_accounts_config = RpcProgramAccountsConfig {
        filters: None,
        with_context: Some(true),
        account_config: account_info_config.clone(),
    };

    // TODO: This way the snapshots are done sequentially, and a failing snapshot prohibits the second one to be attempted

    let mut snapshot = AccountSnapshot::default();

    // Get all accounts of the mango program
    let response = rpc_client
        .get_program_accounts(
            mango_program_id.to_string(),
            Some(all_accounts_config.clone()),
        )
        .await
        .map_err_anyhow()
        .context("error during getProgamAccounts for mango program")?;
    if let OptionalContext::Context(account_snapshot_response) = response {
        snapshot.extend_from_gpa_rpc(account_snapshot_response)?;
    } else {
        anyhow::bail!("did not receive context");
    }

    // Get all the active open orders account keys
    let oo_account_pubkeys =
        snapshot
            .accounts
            .iter()
            .filter_map(|update| {
                if let Ok(mango_account) = healthcheck::load_mango_account::<
                    mango::state::MangoAccount,
                >(
                    mango::state::DataType::MangoAccount, &update.account
                ) {
                    if mango_account.mango_group.to_string() == config.mango_group_id {
                        Some(mango_account)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .flat_map(|mango_account| {
                mango_account
                    .in_margin_basket
                    .iter()
                    .zip(mango_account.spot_open_orders.iter())
                    .filter_map(|(in_basket, oo)| in_basket.then(|| *oo))
            })
            .collect::<Vec<Pubkey>>();

    // Retrieve all the open orders accounts
    let results = stream::iter(oo_account_pubkeys)
        .chunks(config.get_multiple_accounts_count)
        .map(|keys| {
            let rpc_client = &rpc_client;
            let account_info_config = account_info_config.clone();
            async move {
                let string_keys = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
                (
                    keys,
                    rpc_client
                        .get_multiple_accounts(string_keys, Some(account_info_config))
                        .await,
                )
            }
        })
        .buffer_unordered(config.parallel_rpc_requests)
        .collect::<Vec<_>>()
        .await;
    for (keys, result) in results {
        snapshot.extend_from_gma_rpc(
            &keys,
            result
                .map_err_anyhow()
                .context("error during getMultipleAccounts for OpenOrders accounts")?,
        )?;
    }

    sender.send(snapshot).await.expect("sending must succeed");
    Ok(())
}

pub fn start(config: Config, sender: async_channel::Sender<AccountSnapshot>) {
    let mut interval = time::interval(time::Duration::from_secs(config.snapshot_interval_secs));

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            if let Err(err) = feed_snapshots(&config, &sender).await {
                warn!("snapshot error: {:?}", err);
            } else {
                info!("snapshot success");
            };
        }
    });
}
