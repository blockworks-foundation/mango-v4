use crate::configuration::Configuration;
use crate::processors::data::DataEvent::{AccountUpdate, Other, Snapshot};
use async_channel::Receiver;
use chrono::Utc;
use itertools::Itertools;
use mango_v4_client::account_update_stream::Message;
use mango_v4_client::snapshot_source::is_mango_account;
use mango_v4_client::{
    account_update_stream, chain_data, snapshot_source, websocket_source, MangoGroupContext,
};
use services_mango_lib::fail_or_retry;
use services_mango_lib::retry_counter::RetryCounter;
use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::warn;

pub struct DataProcessor {
    pub channel: tokio::sync::broadcast::Sender<DataEvent>,
    pub jobs: Vec<JoinHandle<()>>,
    pub chain_data: Arc<RwLock<chain_data::ChainData>>,
}

#[derive(Clone, Debug)]
pub enum DataEvent {
    Other,
    Snapshot(SnapshotEvent),
    AccountUpdate(AccountUpdateEvent),
}

#[derive(Clone, Debug)]
pub struct SnapshotEvent {
    pub received_at: chrono::DateTime<Utc>,
    pub accounts: Vec<Pubkey>,
}

#[derive(Clone, Debug)]
pub struct AccountUpdateEvent {
    pub received_at: chrono::DateTime<Utc>,
    pub account: Pubkey,
}

impl DataProcessor {
    pub async fn init(
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<DataProcessor> {
        let mut retry_counter = RetryCounter::new(2);
        let mango_group = Pubkey::from_str(&configuration.mango_group)?;
        let (mango_stream, snapshot_job) =
            fail_or_retry!(retry_counter, Self::init_mango_source(configuration).await)?;
        let (sender, _) = tokio::sync::broadcast::channel(8192);
        let sender_clone = sender.clone();

        // The representation of current on-chain account data
        let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
        let chain_data_clone = chain_data.clone();

        let job = tokio::spawn(async move {
            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down data processor...");
                    break;
                }
                tokio::select! {
                    Ok(msg) = mango_stream.recv() => {
                        let received_at = Utc::now();

                        msg.update_chain_data(&mut chain_data_clone.write().unwrap());

                        if sender_clone.receiver_count() == 0 {
                            continue;
                        }

                        let event = Self::parse_message(msg, received_at, mango_group);

                        if event.is_none() {
                            continue;
                        }

                        let res = sender_clone.send(event.unwrap());
                        if res.is_err() {
                            break;
                        }
                    },
                    else => {
                        warn!("mango update channel err");
                        break;
                    }
                }
            }
        });

        let result = DataProcessor {
            channel: sender,
            jobs: vec![job, snapshot_job],
            chain_data,
        };

        Ok(result)
    }

    fn new_rpc_async(configuration: &Configuration) -> RpcClientAsync {
        let commitment = CommitmentConfig::processed();
        RpcClientAsync::new_with_timeout_and_commitment(
            configuration.rpc_http_url.clone(),
            Duration::from_secs(60),
            commitment,
        )
    }

    fn parse_message(
        message: Message,
        received_at: chrono::DateTime<Utc>,
        mango_group: Pubkey,
    ) -> Option<DataEvent> {
        match message {
            Message::Account(account_write) => {
                if is_mango_account(&account_write.account, &mango_group).is_some() {
                    return Some(AccountUpdate(AccountUpdateEvent {
                        account: account_write.pubkey,
                        received_at,
                    }));
                }
            }
            Message::Snapshot(snapshot) => {
                let mut result = Vec::new();
                for update in snapshot.iter() {
                    if is_mango_account(&update.account, &mango_group).is_some() {
                        result.push(update.pubkey);
                    }
                }

                return Some(Snapshot(SnapshotEvent {
                    accounts: result,
                    received_at,
                }));
            }
            _ => {}
        };

        return Some(Other);
    }

    async fn init_mango_source(
        configuration: &Configuration,
    ) -> anyhow::Result<(Receiver<Message>, JoinHandle<()>)> {
        //
        // Client setup
        //
        let rpc_async = Self::new_rpc_async(configuration);

        let mango_group = Pubkey::from_str(&configuration.mango_group)?;
        let group_context = MangoGroupContext::new_from_rpc(&rpc_async, mango_group).await?;

        let mango_oracles = group_context
            .tokens
            .values()
            .map(|value| value.oracle)
            .chain(group_context.perp_markets.values().map(|p| p.oracle))
            .unique()
            .collect::<Vec<Pubkey>>();

        let serum_programs = group_context
            .serum3_markets
            .values()
            .map(|s3| s3.serum_program)
            .unique()
            .collect_vec();

        let (account_update_sender, account_update_receiver) =
            async_channel::unbounded::<account_update_stream::Message>();

        websocket_source::start(
            websocket_source::Config {
                rpc_ws_url: configuration.rpc_ws_url.clone(),
                serum_programs,
                open_orders_authority: mango_group,
            },
            mango_oracles.clone(),
            account_update_sender.clone(),
        );

        let first_websocket_slot = websocket_source::get_next_create_bank_slot(
            account_update_receiver.clone(),
            Duration::from_secs(10),
        )
        .await?;

        // Getting solana account snapshots via jsonrpc
        // FUTURE: of what to fetch a snapshot - should probably take as an input
        let snapshot_job = snapshot_source::start(
            snapshot_source::Config {
                rpc_http_url: configuration.rpc_http_url.clone(),
                mango_group,
                get_multiple_accounts_count: 100,
                parallel_rpc_requests: 10,
                snapshot_interval: Duration::from_secs(configuration.snapshot_interval_secs),
                min_slot: first_websocket_slot + 10,
            },
            mango_oracles,
            account_update_sender,
        );

        Ok((account_update_receiver, snapshot_job))
    }
}
