use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anchor_client::Cluster;
use clap::Parser;
use client::{MangoClient, MangoGroupContext};
use log::*;
use mango_v4::state::{PerpMarketIndex, TokenIndex};

use once_cell::sync::OnceCell;

use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::{
    keypair::{self, Keypair},
    Signer,
};
use std::collections::HashSet;
use std::str::FromStr;

pub mod account_shared_data;
pub mod chain_data;
pub mod chain_data_fetcher;
pub mod liquidate;
pub mod metrics;
pub mod snapshot_source;
pub mod util;
pub mod websocket_source;

use crate::chain_data::*;
use crate::chain_data_fetcher::*;
use crate::util::{is_mango_account, is_mango_bank, is_mint_info, is_perp_market};

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

#[derive(Parser)]
#[clap()]
struct Cli {
    #[clap(short, long, env)]
    rpc_url: String,

    #[clap(long, env)]
    group: Option<Pubkey>,

    // These exist only as a shorthand to make testing easier. Normal users would provide the group.
    #[clap(long, env)]
    group_from_admin_keypair: Option<std::path::PathBuf>,
    #[clap(long, env, default_value = "0")]
    group_from_admin_num: u32,

    // TODO: maybe store this in the group, so it's easy to start without providing it?
    #[clap(long, env)]
    pyth_program: Pubkey,

    // TODO: different serum markets could use different serum programs, should come from registered markets
    #[clap(long, env)]
    serum_program: Pubkey,

    #[clap(long, env)]
    liqor_owner: std::path::PathBuf,

    #[clap(long, env)]
    liqor_mango_account_name: String,

    #[clap(long, env, default_value = "300")]
    snapshot_interval_secs: u64,

    // how many getMultipleAccounts requests to send in parallel
    #[clap(long, env, default_value = "10")]
    parallel_rpc_requests: usize,

    // typically 100 is the max number for getMultipleAccounts
    #[clap(long, env, default_value = "100")]
    get_multiple_accounts_count: usize,
}

fn keypair_from_path(p: &std::path::PathBuf) -> Keypair {
    let path = std::path::PathBuf::from_str(&*shellexpand::tilde(p.to_str().unwrap())).unwrap();
    keypair::read_keypair_file(path)
        .unwrap_or_else(|_| panic!("Failed to read keypair from {}", p.to_string_lossy()))
}

pub fn encode_address(addr: &Pubkey) -> String {
    bs58::encode(&addr.to_bytes()).into_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();

    let liqor_owner = keypair_from_path(&cli.liqor_owner);

    let mango_group = if let Some(group) = cli.group {
        group
    } else if let Some(p) = cli.group_from_admin_keypair {
        let admin = keypair_from_path(&p);
        MangoClient::group_for_admin(admin.pubkey(), cli.group_from_admin_num)
    } else {
        panic!("Must provide either group or group_from_admin_keypair");
    };

    let rpc_url = cli.rpc_url;
    let ws_url = rpc_url.replace("https", "wss");

    let rpc_timeout = Duration::from_secs(1);
    let cluster = Cluster::Custom(rpc_url.clone(), ws_url.clone());
    let commitment = CommitmentConfig::processed();
    let group_context = MangoGroupContext::new_from_rpc(mango_group, cluster.clone(), commitment)?;

    // TODO: this is all oracles, not just pyth!
    let mango_pyth_oracles = group_context
        .tokens
        .values()
        .map(|value| value.mint_info.oracle)
        .collect::<Vec<Pubkey>>();

    //
    // feed setup
    //
    // FUTURE: decouple feed setup and liquidator business logic
    // feed should send updates to a channel which liquidator can consume

    let mango_program = mango_v4::ID;

    solana_logger::setup_with_default("info");
    info!("startup");

    let metrics = metrics::start();

    // Sourcing account and slot data from solana via websockets
    // FUTURE: websocket feed should take which accounts to listen to as an input
    let (websocket_sender, websocket_receiver) =
        async_channel::unbounded::<websocket_source::Message>();
    websocket_source::start(
        websocket_source::Config {
            rpc_ws_url: ws_url.clone(),
            mango_program,
            serum_program: cli.serum_program,
            open_orders_authority: mango_group,
        },
        mango_pyth_oracles.clone(),
        websocket_sender,
    );

    // Getting solana account snapshots via jsonrpc
    let (snapshot_sender, snapshot_receiver) =
        async_channel::unbounded::<snapshot_source::AccountSnapshot>();
    // FUTURE: of what to fetch a snapshot - should probably take as an input
    snapshot_source::start(
        snapshot_source::Config {
            rpc_http_url: rpc_url.clone(),
            mango_program,
            mango_group,
            get_multiple_accounts_count: cli.get_multiple_accounts_count,
            parallel_rpc_requests: cli.parallel_rpc_requests,
            snapshot_interval: std::time::Duration::from_secs(cli.snapshot_interval_secs),
        },
        mango_pyth_oracles,
        snapshot_sender,
    );

    // The representation of current on-chain account data
    let chain_data = Arc::new(RwLock::new(ChainData::new(&metrics)));
    // Reading accounts from chain_data
    let account_fetcher = Arc::new(ChainDataAccountFetcher {
        chain_data: chain_data.clone(),
        rpc: RpcClient::new_with_timeout_and_commitment(
            cluster.url().to_string(),
            rpc_timeout,
            commitment,
        ),
    });

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

    //
    // mango client setup
    //
    let mango_client = {
        Arc::new(MangoClient::new_detail(
            cluster,
            commitment,
            liqor_owner,
            &cli.liqor_mango_account_name,
            group_context,
            account_fetcher.clone(),
        )?)
    };

    info!("main loop");
    loop {
        tokio::select! {
            message = websocket_receiver.recv() => {

                metric_websocket_queue_len.set(websocket_receiver.len() as u64);
                let message = message.expect("channel not closed");

                // build a model of slots and accounts in `chain_data`
                // this code should be generic so it can be reused in future projects
                chain_data.write().unwrap().update_from_websocket(message.clone());

                // specific program logic using the mirrored data
                if let websocket_source::Message::Account(account_write) = message {

                    if is_mango_account(&account_write.account, &mango_program, &mango_group).is_some() {

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
                                &account_fetcher,
                                std::iter::once(&account_write.pubkey),

                        ) {
                            warn!("could not process account {}: {:?}", account_write.pubkey, err);
                        }
                    }

                    if is_mango_bank(&account_write.account, &mango_program, &mango_group).is_some() || oracles.contains(&account_write.pubkey) {
                        if !one_snapshot_done {
                            continue;
                        }

                        if is_mango_bank(&account_write.account, &mango_program, &mango_group).is_some() {
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
                                &account_fetcher,
                                mango_accounts.iter(),
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
                    if is_mango_account(&update.account, &mango_program, &mango_group).is_some() {
                        mango_accounts.insert(update.pubkey);
                    }
                    if let Some(mint_info) = is_mint_info(&update.account, &mango_program, &mango_group) {
                        mint_infos.insert(mint_info.token_index, update.pubkey);
                        oracles.insert(mint_info.oracle);
                    }
                    if let Some(perp_market) = is_perp_market(&update.account, &mango_program, &mango_group) {
                        perp_markets.insert(perp_market.perp_market_index, update.pubkey);
                    }
                }
                metric_mango_accounts.set(mango_accounts.len() as u64);

                chain_data.write().unwrap().update_from_snapshot(message);
                one_snapshot_done = true;

                // trigger a full health check
                if let Err(err) = liquidate::process_accounts(
                        &mango_client,
                        &account_fetcher,
                        mango_accounts.iter(),
                ) {
                    warn!("could not process accounts: {:?}", err);
                }
            },
        }
    }
}
