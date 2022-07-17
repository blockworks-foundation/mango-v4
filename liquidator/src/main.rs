use std::collections::HashMap;
use std::sync::Arc;

use crate::chain_data::*;
use crate::util::{is_mango_account, is_mango_bank, is_mint_info, is_perp_market};

use anchor_client::Cluster;
use client::MangoClient;
use log::*;
use mango_v4::state::{PerpMarketIndex, TokenIndex};

use once_cell::sync::OnceCell;
use serde_derive::Deserialize;

use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::keypair;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;

pub mod account_shared_data;
pub mod chain_data;
pub mod liquidate;
pub mod metrics;
pub mod snapshot_source;
pub mod util;
pub mod websocket_source;

// jemalloc seems to be better at keeping the memory footprint reasonable over
// longer periods of time
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

// first slot received from websocket feed
static FIRST_WEBSOCKET_SLOT: OnceCell<u64> = OnceCell::new();

trait AnyhowWrap {
    type Value;
    fn map_err_anyhow(self) -> anyhow::Result<Self::Value>;
}

impl<T, E: std::fmt::Debug> AnyhowWrap for Result<T, E> {
    type Value = T;
    fn map_err_anyhow(self) -> anyhow::Result<Self::Value> {
        self.map_err(|err| anyhow::anyhow!("{:?}", err))
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub rpc_ws_url: String,
    pub rpc_http_url: String,
    pub mango_program_id: String,
    pub pyth_program_id: String,
    pub mango_group_id: String,
    pub mango_signer_id: String,
    pub serum_program_id: String,
    pub snapshot_interval_secs: u64,
    // how many getMultipleAccounts requests to send in parallel
    pub parallel_rpc_requests: usize,
    // typically 100 is the max number for getMultipleAccounts
    pub get_multiple_accounts_count: usize,

    // FUTURE: split mango client and feed config
    // mango client specific
    pub payer: String,
    pub mango_account_name: String,
}

pub fn encode_address(addr: &Pubkey) -> String {
    bs58::encode(&addr.to_bytes()).into_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("requires a config file argument");
        return Ok(());
    }

    let config: Config = {
        let mut file = File::open(&args[1])?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        toml::from_str(&contents).unwrap()
    };

    let mango_group_id = Pubkey::from_str(&config.mango_group_id)?;

    //
    // mango client setup
    //
    let mango_client = {
        let payer = keypair::read_keypair_file(&config.payer).unwrap();

        let rpc_url = config.rpc_http_url.to_owned();
        let ws_url = rpc_url.replace("https", "wss");

        let cluster = Cluster::Custom(rpc_url, ws_url);
        let commitment = CommitmentConfig::confirmed();

        Arc::new(MangoClient::new(
            cluster,
            commitment,
            mango_group_id,
            payer,
            &config.mango_account_name,
        )?)
    };

    // TODO: this is all oracles, not just pyth!
    let mango_pyth_oracles = mango_client
        .context
        .tokens
        .values()
        .map(|value| value.mint_info.oracle)
        .collect::<Vec<Pubkey>>();

    //
    // feed setup
    //
    // FUTURE: decouple feed setup and liquidator business logic
    // feed should send updates to a channel which liquidator can consume
    let mango_program_id = Pubkey::from_str(&config.mango_program_id)?;

    solana_logger::setup_with_default("info");
    info!("startup");

    let metrics = metrics::start();

    // Sourcing account and slot data from solana via websockets
    // FUTURE: websocket feed should take which accounts to listen to as an input
    let (websocket_sender, websocket_receiver) =
        async_channel::unbounded::<websocket_source::Message>();
    websocket_source::start(config.clone(), mango_pyth_oracles.clone(), websocket_sender);

    // Getting solana account snapshots via jsonrpc
    let (snapshot_sender, snapshot_receiver) =
        async_channel::unbounded::<snapshot_source::AccountSnapshot>();
    // FUTURE: of what to fetch a snapshot - should probably take as an input
    snapshot_source::start(config.clone(), mango_pyth_oracles, snapshot_sender);

    // The representation of current on-chain account data
    let mut chain_data = ChainData::new(&metrics);

    // Addresses of the MangoAccounts belonging to the mango program.
    // Needed to check health of them all when the cache updates.
    let mut mango_accounts = HashSet::<Pubkey>::new();

    let mut mint_infos = HashMap::<TokenIndex, Pubkey>::new();
    let mut oracles = HashSet::<Pubkey>::new();
    let mut perp_markets = HashMap::<PerpMarketIndex, Pubkey>::new();

    // List of accounts that are potentially liquidatable.
    //
    // Used to send a different message for newly liqudatable accounts and
    // accounts that are still liquidatable but not fresh anymore.
    //
    // This should actually be done per connected websocket client, and not globally.
    let _current_candidates = HashSet::<Pubkey>::new();

    // Is the first snapshot done? Only start checking account health when it is.
    let mut one_snapshot_done = false;

    let mut metric_websocket_queue_len = metrics.register_u64("websocket_queue_length".into());
    let mut metric_snapshot_queue_len = metrics.register_u64("snapshot_queue_length".into());
    let mut metric_mango_accounts = metrics.register_u64("mango_accouns".into());

    info!("main loop");
    loop {
        tokio::select! {
            message = websocket_receiver.recv() => {

                metric_websocket_queue_len.set(websocket_receiver.len() as u64);
                let message = message.expect("channel not closed");

                // build a model of slots and accounts in `chain_data`
                // this code should be generic so it can be reused in future projects
                chain_data.update_from_websocket(message.clone());

                // specific program logic using the mirrored data
                if let websocket_source::Message::Account(account_write) = message {

                    if is_mango_account(&account_write.account, &mango_program_id, &mango_group_id).is_some() {

                        // e.g. to render debug logs RUST_LOG="liquidator=debug"
                        log::debug!("change to mango account {}...", &account_write.pubkey.to_string()[0..3]);

                        // Track all MangoAccounts: we need to iterate over them later
                        mango_accounts.insert(account_write.pubkey);
                        metric_mango_accounts.set(mango_accounts.len() as u64);

                        if !one_snapshot_done {
                            continue;
                        }

                        if let Err(err) = liquidate::process_accounts(
                                &mango_client,
                                &chain_data,
                                std::iter::once(&account_write.pubkey),
                                &mint_infos,
                                &perp_markets,

                        ) {
                            warn!("could not process account {}: {:?}", account_write.pubkey, err);
                        }
                    }

                    if is_mango_bank(&account_write.account, &mango_program_id, &mango_group_id).is_some() || oracles.contains(&account_write.pubkey) {
                        if !one_snapshot_done {
                            continue;
                        }

                        if is_mango_bank(&account_write.account, &mango_program_id, &mango_group_id).is_some() {
                            log::debug!("change to bank {}", &account_write.pubkey);
                        }

                        if oracles.contains(&account_write.pubkey) {
                            log::debug!("change to oracle {}", &account_write.pubkey);
                        }

                        // check health of all accounts
                        //
                        // TODO: This could be done asynchronously by calling
                        // let accounts = chain_data.accounts_snapshot();
                        // and then working with the snapshot of the data
                        //
                        // However, this currently takes like 50ms for me in release builds,
                        // so optimizing much seems unnecessary.
                        if let Err(err) = liquidate::process_accounts(
                                &mango_client,
                                &chain_data,
                                mango_accounts.iter(),
                                &mint_infos,
                                &perp_markets,
                        ) {
                            warn!("could not process accounts: {:?}", err);
                        }
                    }
                }
            },

            message = snapshot_receiver.recv() => {
                metric_snapshot_queue_len.set(snapshot_receiver.len() as u64);
                let message = message.expect("channel not closed");

                // Track all mango account pubkeys
                for update in message.accounts.iter() {
                    if is_mango_account(&update.account, &mango_program_id, &mango_group_id).is_some() {
                        mango_accounts.insert(update.pubkey);
                    }
                    if let Some(mint_info) = is_mint_info(&update.account, &mango_program_id, &mango_group_id) {
                        mint_infos.insert(mint_info.token_index, update.pubkey);
                        oracles.insert(mint_info.oracle);
                    }
                    if let Some(perp_market) = is_perp_market(&update.account, &mango_program_id, &mango_group_id) {
                        perp_markets.insert(perp_market.perp_market_index, update.pubkey);
                    }
                }
                metric_mango_accounts.set(mango_accounts.len() as u64);

                chain_data.update_from_snapshot(message);
                one_snapshot_done = true;

                // trigger a full health check
                if let Err(err) = liquidate::process_accounts(
                        &mango_client,
                        &chain_data,
                        mango_accounts.iter(),
                        &mint_infos,
                        &perp_markets,
                ) {
                    warn!("could not process accounts: {:?}", err);
                }
            },
        }
    }
}
