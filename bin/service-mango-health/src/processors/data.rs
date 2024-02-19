use crate::configuration::Configuration;
use crate::processors::data::DataEvent::{AccountUpdate, Other, Snapshot};
use anchor_client::Cluster;
use async_channel::Receiver;
use itertools::Itertools;
use log::warn;
use mango_v4::accounts_zerocopy::LoadZeroCopy;
use mango_v4_client::account_update_stream::Message;
use mango_v4_client::snapshot_source::is_mango_account;
use mango_v4_client::{
    account_update_stream, chain_data, keypair_from_cli, snapshot_source, websocket_source, Client,
    MangoGroupContext, TransactionBuilderConfig,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

pub struct DataProcessor {
    pub receiver: async_channel::Receiver<DataEvent>,
    pub job: JoinHandle<()>,
    pub chain_data: Arc<RwLock<chain_data::ChainData>>,
}

pub enum DataEvent {
    Other,
    Snapshot(SnapshotEvent),
    AccountUpdate(AccountUpdateEvent),
}

pub struct SnapshotEvent {
    pub received_at: Instant,
    pub accounts: Vec<Pubkey>,
}

pub struct AccountUpdateEvent {
    pub received_at: Instant,
    pub account: Pubkey,
}

impl DataProcessor {
    pub async fn init(
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<DataProcessor> {
        let mango_group = Pubkey::from_str(&configuration.mango_group)?;
        let mango_stream = Self::init_mango_source(configuration).await?;
        let (sender, receiver) = async_channel::unbounded::<DataEvent>();

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
                        let received_at = Instant::now();

                        msg.update_chain_data(&mut chain_data_clone.write().unwrap());
                        let event = Self::parse_message(msg, received_at, mango_group);

                        if event.is_none() {
                            continue;
                        }

                        let res = sender.try_send(event.unwrap());
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
            receiver,
            job,
            chain_data: chain_data.clone(),
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
        received_at: Instant,
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

        return return Some(Other);
    }

    async fn init_mango_source(configuration: &Configuration) -> anyhow::Result<Receiver<Message>> {
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
                rpc_ws_url: configuration.source.rpc_ws_url.clone(),
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
        snapshot_source::start(
            snapshot_source::Config {
                rpc_http_url: configuration.source.snapshot.rpc_http_url.clone(),
                mango_group,
                get_multiple_accounts_count: 100,
                parallel_rpc_requests: 10,
                snapshot_interval: Duration::from_secs(5 * 60),
                min_slot: first_websocket_slot + 10,
            },
            mango_oracles,
            account_update_sender,
        );

        Ok(account_update_receiver)
    }
}
