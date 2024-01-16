use anchor_lang::AccountDeserialize;
use itertools::Itertools;
use mango_v4::accounts_zerocopy::LoadZeroCopy;
use mango_v4::serum3_cpi::{load_open_orders_bytes, OpenOrdersSlim};
use mango_v4_client::{
    account_update_stream, chain_data, snapshot_source, websocket_source, Client, MangoGroupContext,
};
use solana_sdk::account::{AccountSharedData, ReadableAccount};
use solana_sdk::pubkey::Pubkey;

use std::fs;
use std::path::Path;
use std::time::Duration;

pub async fn save_snapshot(
    mango_group: Pubkey,
    client: Client,
    output: String,
) -> anyhow::Result<()> {
    let out_path = Path::new(&output);
    if out_path.exists() {
        anyhow::bail!("path {output} exists already");
    }
    fs::create_dir_all(out_path).unwrap();

    let rpc_url = client.config().cluster.url().to_string();
    let ws_url = client.config().cluster.ws_url().to_string();

    let group_context = MangoGroupContext::new_from_rpc(client.rpc_async(), mango_group).await?;

    let oracles_and_vaults = group_context
        .tokens
        .values()
        .map(|value| value.oracle)
        .chain(group_context.perp_markets.values().map(|p| p.oracle))
        .chain(group_context.tokens.values().flat_map(|value| value.vaults))
        .unique()
        .filter(|pk| *pk != Pubkey::default())
        .collect::<Vec<Pubkey>>();

    let serum_programs = group_context
        .serum3_markets
        .values()
        .map(|s3| s3.serum_program)
        .unique()
        .collect_vec();

    let (account_update_sender, account_update_receiver) =
        async_channel::unbounded::<account_update_stream::Message>();

    // Sourcing account and slot data from solana via websockets
    websocket_source::start(
        websocket_source::Config {
            rpc_ws_url: ws_url.clone(),
            serum_programs,
            open_orders_authority: mango_group,
        },
        oracles_and_vaults.clone(),
        account_update_sender.clone(),
    );

    let first_websocket_slot = websocket_source::get_next_create_bank_slot(
        account_update_receiver.clone(),
        Duration::from_secs(10),
    )
    .await?;

    // Getting solana account snapshots via jsonrpc
    snapshot_source::start(
        snapshot_source::Config {
            rpc_http_url: rpc_url.clone(),
            mango_group,
            get_multiple_accounts_count: 100,
            parallel_rpc_requests: 10,
            snapshot_interval: Duration::from_secs(6000),
            min_slot: first_websocket_slot + 10,
        },
        oracles_and_vaults,
        account_update_sender,
    );

    let mut chain_data = chain_data::ChainData::new();

    use account_update_stream::Message;
    loop {
        let message = account_update_receiver
            .recv()
            .await
            .expect("channel not closed");

        message.update_chain_data(&mut chain_data);

        match message {
            Message::Account(_) => {}
            Message::Snapshot(snapshot) => {
                for slot in snapshot.iter().map(|a| a.slot).unique() {
                    chain_data.update_slot(chain_data::SlotData {
                        slot,
                        parent: None,
                        status: chain_data::SlotStatus::Rooted,
                        chain: 0,
                    });
                }
                break;
            }
            _ => {}
        }
    }

    // Write out all the data
    use base64::Engine;
    use serde_json::json;
    let b64 = base64::engine::general_purpose::STANDARD;
    for (pk, account) in chain_data.iter_accounts_rooted() {
        let debug = to_debug(&account.account);
        let data = json!({
            "address": pk.to_string(),
            "slot": account.slot,
            // mimic an rpc response
            "account": {
                "owner": account.account.owner().to_string(),
                "data": [b64.encode(account.account.data()), "base64"],
                "lamports": account.account.lamports(),
                "executable": account.account.executable(),
                "rentEpoch": account.account.rent_epoch(),
                "size": account.account.data().len(),
            },
            "debug": debug,
        })
        .to_string();
        fs::write(out_path.join(format!("{}.json", pk)), data)?;
    }

    Ok(())
}

fn to_debug(account: &AccountSharedData) -> Option<String> {
    use mango_v4::state::*;
    if account.owner() == &mango_v4::ID {
        let mut bytes = account.data();
        if let Ok(mango_account) = MangoAccount::try_deserialize(&mut bytes) {
            return Some(format!("{mango_account:?}"));
        }
    }
    if let Ok(d) = account.load::<Bank>() {
        return Some(format!("{d:?}"));
    }
    if let Ok(d) = account.load::<Group>() {
        return Some(format!("{d:?}"));
    }
    if let Ok(d) = account.load::<MintInfo>() {
        return Some(format!("{d:?}"));
    }
    if let Ok(d) = account.load::<PerpMarket>() {
        return Some(format!("{d:?}"));
    }
    if let Ok(d) = account.load::<Serum3Market>() {
        return Some(format!("{d:?}"));
    }
    // TODO: owner check...
    if &account.data()[0..5] == b"serum" {
        if let Ok(oo) = load_open_orders_bytes(account.data()) {
            return Some(format!("{:?}", OpenOrdersSlim::from_oo(oo)));
        }
    }
    // BookSide? EventQueue?
    None
}
