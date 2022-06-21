use jsonrpc_core_client::transports::http;

use solana_account_decoder::{UiAccount, UiAccountEncoding};
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_response::{Response, RpcKeyedAccount},
};
use solana_rpc::{
    rpc::OptionalContext,
    rpc::{rpc_accounts::AccountsDataClient, rpc_minimal::MinimalClient},
};
use solana_sdk::{account::AccountSharedData, commitment_config::CommitmentConfig, pubkey::Pubkey};

use anyhow::Context;
use futures::{stream, StreamExt};
use log::*;
use std::str::FromStr;
use tokio::time;

use crate::{util::is_mango_account, AnyhowWrap, Config, FIRST_WEBSOCKET_SLOT};

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
                    .ok_or_else(|| anyhow::anyhow!("could not decode account"))?,
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
                        .ok_or_else(|| anyhow::anyhow!("could not decode account"))?,
                });
            }
        }
        Ok(())
    }
}

async fn feed_snapshots(
    config: &Config,
    mango_pyth_oracles: Vec<Pubkey>,
    sender: &async_channel::Sender<AccountSnapshot>,
) -> anyhow::Result<()> {
    let mango_program_id = Pubkey::from_str(&config.mango_program_id)?;
    let mango_group_id = Pubkey::from_str(&config.mango_group_id)?;

    let rpc_client = http::connect_with_options::<AccountsDataClient>(&config.rpc_http_url, true)
        .await
        .map_err_anyhow()?;

    let account_info_config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        commitment: Some(CommitmentConfig::finalized()),
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

    // Get all the pyth oracles referred to by mango banks
    let results: Vec<(
        Vec<Pubkey>,
        Result<Response<Vec<Option<UiAccount>>>, jsonrpc_core_client::RpcError>,
    )> = stream::iter(mango_pyth_oracles)
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
                .context("error during getMultipleAccounts for Pyth Oracles")?,
        )?;
    }

    // Get all the active open orders account keys
    let oo_account_pubkeys = snapshot
        .accounts
        .iter()
        .filter_map(|update| is_mango_account(&update.account, &mango_program_id, &mango_group_id))
        .flat_map(|mango_account| {
            mango_account
                .serum3
                .iter_active()
                .map(|serum3account| serum3account.open_orders)
        })
        .collect::<Vec<Pubkey>>();

    // Retrieve all the open orders accounts
    let results: Vec<(
        Vec<Pubkey>,
        Result<Response<Vec<Option<UiAccount>>>, jsonrpc_core_client::RpcError>,
    )> = stream::iter(oo_account_pubkeys)
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

pub fn start(
    config: Config,
    mango_pyth_oracles: Vec<Pubkey>,
    sender: async_channel::Sender<AccountSnapshot>,
) {
    let mut poll_wait_first_snapshot = time::interval(time::Duration::from_secs(2));
    let mut interval_between_snapshots =
        time::interval(time::Duration::from_secs(config.snapshot_interval_secs));

    tokio::spawn(async move {
        let rpc_client = http::connect_with_options::<MinimalClient>(&config.rpc_http_url, true)
            .await
            .expect("always Ok");

        loop {
            poll_wait_first_snapshot.tick().await;

            let epoch_info = rpc_client
                .get_epoch_info(Some(CommitmentConfig::finalized()))
                .await
                .expect("always Ok");
            log::debug!("latest slot for snapshot {}", epoch_info.absolute_slot);

            match FIRST_WEBSOCKET_SLOT.get() {
                Some(first_websocket_slot) => {
                    if first_websocket_slot < &epoch_info.absolute_slot {
                        log::debug!("continuing to fetch snapshot now, first websocket feed slot {} is older than latest snapshot slot {}",first_websocket_slot,  epoch_info.absolute_slot);
                        break;
                    }
                }
                None => {}
            }
        }

        loop {
            interval_between_snapshots.tick().await;
            if let Err(err) = feed_snapshots(&config, mango_pyth_oracles.clone(), &sender).await {
                warn!("snapshot error: {:?}", err);
            } else {
                info!("snapshot success");
            };
        }
    });
}
