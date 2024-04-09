pub use crate::chain_data_fetcher::AccountFetcher;
pub use mango_feeds_connector::snapshot::*;

use jsonrpc_core_client::transports::http;

use mango_v4::accounts_zerocopy::*;
use mango_v4::state::{MangoAccountFixed, MangoAccountLoadedRef};
use solana_client::rpc_config::RpcContextConfig;
use solana_rpc::rpc::rpc_minimal::MinimalClient;
use solana_sdk::{account::AccountSharedData, commitment_config::CommitmentConfig, pubkey::Pubkey};

use anyhow::Context;
use futures::{stream, StreamExt};
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::*;

use crate::account_update_stream::{AccountUpdate, Message, SnapshotType};
use crate::AnyhowWrap;

pub fn is_mango_account<'a>(
    account: &'a AccountSharedData,
    group_id: &Pubkey,
) -> Option<MangoAccountLoadedRef<'a>> {
    // check owner, discriminator
    let fixed = account.load::<MangoAccountFixed>().ok()?;
    if fixed.group != *group_id {
        return None;
    }

    let data = account.data();
    MangoAccountLoadedRef::from_bytes(&data[8..]).ok()
}

#[derive(Default)]
struct AccountSnapshot {
    accounts: Vec<AccountUpdate>,
}

impl AccountSnapshot {
    pub fn extend_from_gpa_rpc(&mut self, rpc: SnapshotProgramAccounts) -> anyhow::Result<()> {
        self.accounts.reserve(rpc.accounts.len());
        for a in rpc.accounts {
            self.accounts.push(AccountUpdate {
                slot: rpc.slot,
                pubkey: Pubkey::from_str(&a.pubkey).unwrap(),
                account: a
                    .account
                    .decode()
                    .ok_or_else(|| anyhow::anyhow!("could not decode account"))?,
                reception_time: Instant::now(),
            });
        }
        Ok(())
    }

    pub fn extend_from_gma_rpc(&mut self, rpc: SnapshotMultipleAccounts) -> anyhow::Result<()> {
        self.accounts.reserve(rpc.accounts.len());
        for (key, a) in rpc.accounts.iter() {
            if let Some(ui_account) = a {
                self.accounts.push(AccountUpdate {
                    slot: rpc.slot,
                    pubkey: Pubkey::from_str(key)?,
                    account: ui_account
                        .decode()
                        .ok_or_else(|| anyhow::anyhow!("could not decode account"))?,
                    reception_time: Instant::now(),
                });
            }
        }
        Ok(())
    }
}

pub struct Config {
    pub rpc_http_url: String,
    pub mango_group: Pubkey,
    pub get_multiple_accounts_count: usize,
    pub parallel_rpc_requests: usize,
    pub snapshot_interval: Duration,
    pub min_slot: u64,
}

async fn feed_snapshots(
    config: &Config,
    mango_oracles: Vec<Pubkey>,
    sender: &async_channel::Sender<Message>,
) -> anyhow::Result<()> {
    // TODO: This way the snapshots are done sequentially, and a failing snapshot prohibits the second one to be attempted

    let mut snapshot = AccountSnapshot::default();

    // Get all accounts of the mango program
    let response = get_snapshot_gpa(config.rpc_http_url.to_string(), mango_v4::id().to_string())
        .await
        .map_err_anyhow()
        .context("error during getProgamAccounts for mango program")?;
    snapshot.extend_from_gpa_rpc(response)?;

    // Get all the pyth oracles referred to by mango banks
    let results: Vec<anyhow::Result<SnapshotMultipleAccounts>> = stream::iter(mango_oracles)
        .chunks(config.get_multiple_accounts_count)
        .map(|keys| async move {
            let string_keys = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
            get_snapshot_gma(config.rpc_http_url.to_string(), string_keys).await
        })
        .buffer_unordered(config.parallel_rpc_requests)
        .collect::<Vec<_>>()
        .await;
    for result in results {
        snapshot.extend_from_gma_rpc(
            result
                .map_err_anyhow()
                .context("error during getMultipleAccounts for Pyth Oracles")?,
        )?;
    }

    // Get all the active open orders account keys
    let oo_account_pubkeys = snapshot
        .accounts
        .iter()
        .filter_map(|update| is_mango_account(&update.account, &config.mango_group))
        .flat_map(|mango_account| {
            mango_account
                .active_serum3_orders()
                .map(|serum3account| serum3account.open_orders)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<Pubkey>>();

    // Retrieve all the open orders accounts
    let results: Vec<anyhow::Result<SnapshotMultipleAccounts>> = stream::iter(oo_account_pubkeys)
        .chunks(config.get_multiple_accounts_count)
        .map(|keys| async move {
            let string_keys = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
            get_snapshot_gma(config.rpc_http_url.to_string(), string_keys).await
        })
        .buffer_unordered(config.parallel_rpc_requests)
        .collect::<Vec<_>>()
        .await;
    for result in results {
        snapshot.extend_from_gma_rpc(
            result
                .map_err_anyhow()
                .context("error during getMultipleAccounts for OpenOrders accounts")?,
        )?;
    }

    sender
        .send(Message::Snapshot(snapshot.accounts, SnapshotType::Full))
        .await
        .expect("sending must succeed");
    Ok(())
}

pub fn start(
    config: Config,
    mango_oracles: Vec<Pubkey>,
    sender: async_channel::Sender<Message>,
) -> JoinHandle<()> {
    let mut poll_wait_first_snapshot = crate::delay_interval(time::Duration::from_secs(2));
    let mut interval_between_snapshots = crate::delay_interval(config.snapshot_interval);

    let snapshot_job = tokio::spawn(async move {
        let rpc_client = http::connect_with_options::<MinimalClient>(&config.rpc_http_url, true)
            .await
            .expect("always Ok");

        // Wait for slot to exceed min_slot
        loop {
            poll_wait_first_snapshot.tick().await;

            let epoch_info = rpc_client
                .get_epoch_info(Some(RpcContextConfig {
                    commitment: Some(CommitmentConfig::finalized()),
                    min_context_slot: None,
                }))
                .await
                .expect("always Ok");
            debug!("latest slot for snapshot {}", epoch_info.absolute_slot);

            if epoch_info.absolute_slot > config.min_slot {
                debug!("continuing to fetch snapshot now, min_slot {} is older than latest epoch slot {}", config.min_slot, epoch_info.absolute_slot);
                break;
            }
        }

        loop {
            interval_between_snapshots.tick().await;
            if let Err(err) = feed_snapshots(&config, mango_oracles.clone(), &sender).await {
                warn!("snapshot error: {:?}", err);
            } else {
                info!("snapshot success");
            };
        }
    });

    snapshot_job
}
