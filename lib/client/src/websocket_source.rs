use anchor_lang::Discriminator;
use jsonrpc_core::futures::StreamExt;
use jsonrpc_core_client::transports::ws;

use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
    rpc_response::{RpcKeyedAccount, RpcResponseContext},
};
use solana_rpc::rpc_pubsub::RpcSolPubSubClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};

use anyhow::Context;
use std::time::{Duration, Instant};
use tokio_stream::StreamMap;
use tracing::*;

use crate::account_update_stream::{AccountUpdate, ChainSlotUpdate, Message};
use crate::AnyhowWrap;

pub struct Config {
    pub rpc_ws_url: String,
    pub serum_programs: Vec<Pubkey>,
    pub open_orders_authority: Pubkey,
}

async fn feed_data(
    config: &Config,
    mango_oracles: Vec<Pubkey>,
    sender: async_channel::Sender<Message>,
) -> anyhow::Result<()> {
    let connect = ws::try_connect::<RpcSolPubSubClient>(&config.rpc_ws_url).map_err_anyhow()?;
    let client = connect.await.map_err_anyhow()?;

    let account_info_config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        commitment: Some(CommitmentConfig::processed()),
        data_slice: None,
        min_context_slot: None,
    };
    let all_accounts_config = RpcProgramAccountsConfig {
        filters: None,
        with_context: Some(true),
        account_config: account_info_config.clone(),
    };
    let serum_oo_accounts_config = RpcProgramAccountsConfig {
        // filter for only OpenOrders with v4 authority
        filters: Some(vec![
            RpcFilterType::DataSize(3228), // open orders size
            // "serum" + u64 that is Initialized (1) + OpenOrders (4)
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                // new_base58_encoded() does not work with old RPC nodes
                0,
                [0x73, 0x65, 0x72, 0x75, 0x6d, 5, 0, 0, 0, 0, 0, 0, 0].to_vec(),
            )),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                45,
                config.open_orders_authority.to_bytes().to_vec(),
            )),
        ]),
        with_context: Some(true),
        account_config: account_info_config.clone(),
    };
    let obv2_oo_accounts_config = RpcProgramAccountsConfig {
        // filter for only OpenOrders with the delegate as the mango group
        // (the individual mango accounts are the owners)
        filters: Some(vec![
            RpcFilterType::DataSize(
                8 + std::mem::size_of::<openbook_v2::state::OpenOrdersAccount>() as u64,
            ),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                0,
                openbook_v2::state::OpenOrdersAccount::DISCRIMINATOR.to_vec(),
            )),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                96,
                config.open_orders_authority.to_bytes().to_vec(),
            )),
        ]),
        with_context: Some(true),
        account_config: account_info_config.clone(),
    };
    let mut mango_sub = client
        .program_subscribe(
            mango_v4::id().to_string(),
            Some(all_accounts_config.clone()),
        )
        .map_err_anyhow()?;

    let mut mango_oracles_sub_map = StreamMap::new();
    for oracle in mango_oracles.into_iter() {
        mango_oracles_sub_map.insert(
            oracle,
            client
                .account_subscribe(
                    oracle.to_string(),
                    Some(RpcAccountInfoConfig {
                        encoding: Some(UiAccountEncoding::Base64),
                        commitment: Some(CommitmentConfig::processed()),
                        data_slice: None,
                        min_context_slot: None,
                    }),
                )
                .map_err_anyhow()?,
        );
    }

    let mut spot_oo_sub_map = StreamMap::new();
    for serum_program in config.serum_programs.iter() {
        spot_oo_sub_map.insert(
            *serum_program,
            client
                .program_subscribe(
                    serum_program.to_string(),
                    Some(serum_oo_accounts_config.clone()),
                )
                .map_err_anyhow()?,
        );
    }
    spot_oo_sub_map.insert(
        openbook_v2::id(),
        client
            .program_subscribe(openbook_v2::id().to_string(), Some(obv2_oo_accounts_config))
            .map_err_anyhow()?,
    );

    // Make sure the serum3_oo_sub_map does not exit when there's no serum_programs
    let _unused_serum_sender;
    if config.serum_programs.is_empty() {
        let (sender, receiver) = jsonrpc_core::futures::channel::mpsc::unbounded();
        _unused_serum_sender = sender;
        spot_oo_sub_map.insert(
            Pubkey::default(),
            jsonrpc_core_client::TypedSubscriptionStream::new(receiver, "foo"),
        );
    }

    let mut slot_sub = client.slots_updates_subscribe().map_err_anyhow()?;

    loop {
        tokio::select! {
            message = mango_sub.next() => {
                if let Some(data) = message {
                    let response = data.map_err_anyhow()?;
                    sender.send(Message::Account(AccountUpdate::from_rpc(response)?)).await.expect("sending must succeed");
                } else {
                    warn!("mango stream closed");
                    return Ok(());
                }
            },
            message = mango_oracles_sub_map.next() => {
                if let Some(data) = message {
                    let response = data.1.map_err_anyhow()?;
                    let response = solana_client::rpc_response::Response{ context: RpcResponseContext{ slot: response.context.slot, api_version: None }, value: RpcKeyedAccount{ pubkey: data.0.to_string(), account:  response.value} } ;
                    sender.send(Message::Account(AccountUpdate::from_rpc(response)?)).await.expect("sending must succeed");
                } else {
                    warn!("oracle stream closed");
                    return Ok(());
                }
            },
            message = spot_oo_sub_map.next() => {
                if let Some(data) = message {
                    let response = data.1.map_err_anyhow()?;
                    sender.send(Message::Account(AccountUpdate::from_rpc(response)?)).await.expect("sending must succeed");
                } else {
                    warn!("spot oo stream closed");
                    return Ok(());
                }
            },
            message = slot_sub.next() => {
                if let Some(data) = message {
                    sender.send(Message::Slot(ChainSlotUpdate{
                        slot_update: data.map_err_anyhow()?,
                        reception_time: Instant::now()
                    })).await.expect("sending must succeed");
                } else {
                    warn!("slot update stream closed");
                    return Ok(());
                }
            },
            _ = tokio::time::sleep(Duration::from_secs(60)) => {
                warn!("websocket timeout");
                return Ok(())
            }
        }
    }
}

pub fn start(config: Config, mango_oracles: Vec<Pubkey>, sender: async_channel::Sender<Message>) {
    tokio::spawn(async move {
        // if the websocket disconnects, we get no data in a while etc, reconnect and try again
        loop {
            info!("connecting to solana websocket streams");
            let out = feed_data(&config, mango_oracles.clone(), sender.clone());
            let result = out.await;
            if let Err(err) = result {
                warn!("websocket stream error: {err}");
            }
        }
    });
}

pub async fn get_next_create_bank_slot(
    receiver: async_channel::Receiver<Message>,
    timeout: Duration,
) -> anyhow::Result<u64> {
    let start = std::time::Instant::now();
    loop {
        let elapsed = start.elapsed();
        if elapsed > timeout {
            anyhow::bail!(
                "did not receive a slot from the websocket connection in {}s",
                timeout.as_secs()
            );
        }
        let remaining_timeout = timeout - elapsed;

        let msg = match tokio::time::timeout(remaining_timeout, receiver.recv()).await {
            // timeout
            Err(_) => continue,
            // channel close
            Ok(Err(err)) => {
                return Err(err).context("while waiting for first slot from websocket connection");
            }
            // success
            Ok(Ok(msg)) => msg,
        };

        match msg {
            Message::Slot(slot_update) => {
                if let solana_client::rpc_response::SlotUpdate::CreatedBank { slot, .. } =
                    *slot_update.slot_update
                {
                    return Ok(slot);
                }
            }
            _ => {}
        }
    }
}
