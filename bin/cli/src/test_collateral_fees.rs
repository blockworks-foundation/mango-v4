use anchor_lang::prelude::AccountInfo;
use itertools::Itertools;
use mango_v4::accounts_zerocopy::LoadZeroCopy;
use mango_v4::instructions::token_charge_collateral_fees_internal;
use mango_v4::state::{DynamicAccount, Group};
use mango_v4_client::snapshot_source::is_mango_account;
use mango_v4_client::{
    account_update_stream, chain_data, snapshot_source, websocket_source, Client, MangoGroupContext,
};
use solana_sdk::account::ReadableAccount;
use solana_sdk::pubkey::Pubkey;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub async fn run(client: &Client, mango_group: Pubkey) -> anyhow::Result<()> {
    let rpc_async = client.rpc_async();
    let group_context = MangoGroupContext::new_from_rpc(&rpc_async, mango_group).await?;

    let rpc_url = client.config().cluster.url().to_string();
    let ws_url = client.config().cluster.ws_url().to_string();

    let slot = client.rpc_async().get_slot().await?;
    let ts = chrono::Utc::now().timestamp() as u64;

    let extra_accounts = group_context
        .tokens
        .values()
        .map(|value| value.oracle)
        .chain(group_context.perp_markets.values().map(|p| p.oracle))
        .chain(group_context.tokens.values().flat_map(|value| value.vaults))
        .chain(group_context.address_lookup_tables.iter().copied())
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
        extra_accounts.clone(),
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
        extra_accounts,
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

    let group = &chain_data.account(&mango_group).unwrap().account.clone();
    let group = group.load::<Group>()?;

    let chain_data = Arc::new(RwLock::new(chain_data));

    let account_fetcher = Arc::new(chain_data::AccountFetcher {
        chain_data: chain_data.clone(),
        rpc: client.new_rpc_async(),
    });

    for (key, data) in chain_data.read().unwrap().iter_accounts() {
        if let Some(account) = is_mango_account(&data.account, &mango_group) {
            // let dyn_part = account.dynamic.clone();
            // let dyn_part = RefCell::new(*dyn_part);
            let fixed = account.fixed.clone();
            let fixed_cell = RefCell::new(fixed);
            let mut account = DynamicAccount {
                header: account.header,
                fixed: fixed_cell.borrow_mut(),
                dynamic: account.dynamic.iter().map(|x| *x).collect::<Vec<u8>>(),
            };

            let acc = account_fetcher.fetch_mango_account(key)?;

            let (health_remaining_ams, _) = group_context
                .derive_health_check_remaining_account_metas(
                    &acc,
                    vec![],
                    vec![],
                    vec![],
                    HashMap::new(),
                )
                .unwrap();

            let mut remaining_accounts: Vec<_> = health_remaining_ams
                .into_iter()
                .map(|x| {
                    let xx = account_fetcher.fetch_raw(&x.pubkey).unwrap();
                    TestAccount::new(
                        xx.data().iter().map(|x| *x).collect(),
                        x.pubkey,
                        *xx.owner(),
                    )
                })
                .collect();

            let remaining_accounts = remaining_accounts
                .iter_mut()
                .map(|x| return x.as_account_info())
                .collect::<Vec<_>>();

            let mut out = HashMap::new();

            // Act like it was never charged, but not initial call (0)
            account.borrow_mut().fixed.last_collateral_fee_charge = 1;

            match token_charge_collateral_fees_internal(
                account,
                group,
                remaining_accounts.as_slice(),
                mango_group,
                *key,
                (ts, slot),
                Some(&mut out),
            ) {
                Ok(_) => {
                    for (x, fee) in out {
                        println!(
                            "{} -> Token: {} => {} ({} $)",
                            key,
                            group_context.tokens.get(&x).unwrap().name,
                            fee.0 / 2,
                            fee.1 / 2
                        );
                    }
                }
                Err(e) => {
                    println!("{} -> Error: {:?}", key, e);
                }
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
pub struct TestAccount {
    pub bytes: Vec<u8>,
    pub pubkey: Pubkey,
    pub owner: Pubkey,
    pub lamports: u64,
}

impl TestAccount {
    pub fn new(bytes: Vec<u8>, pubkey: Pubkey, owner: Pubkey) -> Self {
        Self {
            bytes,
            owner,
            pubkey,
            lamports: 0,
        }
    }

    pub fn as_account_info(&mut self) -> AccountInfo {
        AccountInfo {
            key: &self.pubkey,
            owner: &self.owner,
            lamports: Rc::new(RefCell::new(&mut self.lamports)),
            data: Rc::new(RefCell::new(&mut self.bytes)),
            is_signer: false,
            is_writable: false,
            executable: false,
            rent_epoch: 0,
        }
    }
}
