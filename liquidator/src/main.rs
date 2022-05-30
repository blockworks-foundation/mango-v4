pub mod chain_data;
pub mod healthcheck;
pub mod metrics;
pub mod snapshot_source;
pub mod websocket_sink;
pub mod websocket_source;

use {
    crate::chain_data::*,
    log::*,
    mango::state::{DataType, MangoAccount},
    mango_common::Loadable,
    serde_derive::Deserialize,
    solana_sdk::account::{AccountSharedData, ReadableAccount},
    solana_sdk::pubkey::Pubkey,
    std::collections::HashSet,
    std::fs::File,
    std::io::Read,
    std::str::FromStr,
};

// jemalloc seems to be better at keeping the memory footprint reasonable over
// longer periods of time
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

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
    pub mango_group_id: String,
    pub mango_cache_id: String,
    pub mango_signer_id: String,
    pub serum_program_id: String,
    pub snapshot_interval_secs: u64,
    pub websocket_server_bind_address: String,
    // how many getMultipleAccounts requests to send in parallel
    pub parallel_rpc_requests: usize,
    // typically 100 is the max number for getMultipleAccounts
    pub get_multiple_accounts_count: usize,
    pub early_candidate_percentage: f64,
}

pub fn encode_address(addr: &Pubkey) -> String {
    bs58::encode(&addr.to_bytes()).into_string()
}

fn is_mango_account<'a>(
    account: &'a AccountSharedData,
    program_id: &Pubkey,
    group_id: &Pubkey,
) -> Option<&'a MangoAccount> {
    let data = account.data();
    if account.owner() != program_id || data.len() == 0 {
        return None;
    }
    let kind = DataType::try_from(data[0]).ok()?;
    if !matches!(kind, DataType::MangoAccount) {
        return None;
    }
    if data.len() != std::mem::size_of::<MangoAccount>() {
        return None;
    }
    let mango_account = MangoAccount::load_from_bytes(&data).expect("always Ok");
    if mango_account.mango_group != *group_id {
        return None;
    }
    Some(mango_account)
}

fn is_mango_cache<'a>(account: &'a AccountSharedData, program_id: &Pubkey) -> bool {
    let data = account.data();
    if account.owner() != program_id || data.len() == 0 {
        return false;
    }
    let kind = DataType::try_from(data[0]).unwrap();
    matches!(kind, DataType::MangoCache)
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

    let mango_program_id = Pubkey::from_str(&config.mango_program_id)?;
    let mango_group_id = Pubkey::from_str(&config.mango_group_id)?;
    let mango_cache_id = Pubkey::from_str(&config.mango_cache_id)?;

    solana_logger::setup_with_default("info");
    info!("startup");

    let metrics = metrics::start();

    // Information about potentially liquidatable accounts is sent through this
    // channel and then forwarded to all connected websocket clients
    let liquidation_candidate_sender = websocket_sink::start(config.clone()).await?;

    // Sourcing account and slot data from solana via websockets
    let (websocket_sender, websocket_receiver) =
        async_channel::unbounded::<websocket_source::Message>();
    websocket_source::start(config.clone(), websocket_sender);

    // Wait for some websocket data to accumulate before requesting snapshots,
    // to make it more likely that there's no gap between the slot the snapshot
    // was for and the slot of the first websocket messages.
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    // Getting solana account snapshots via jsonrpc
    let (snapshot_sender, snapshot_receiver) =
        async_channel::unbounded::<snapshot_source::AccountSnapshot>();
    snapshot_source::start(config.clone(), snapshot_sender);

    // The representation of current on-chain account data
    let mut chain_data = ChainData::new(&metrics);

    // Addresses of the MangoAccounts belonging to the mango program.
    // Needed to check health of them all when the cache updates.
    let mut mango_accounts = HashSet::<Pubkey>::new();

    // List of accounts that are potentially liquidatable.
    //
    // Used to send a different message for newly liqudatable accounts and
    // accounts that are still liquidatable but not fresh anymore.
    //
    // This should actually be done per connected websocket client, and not globally.
    let mut current_candidates = HashSet::<Pubkey>::new();

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
                match message {
                    websocket_source::Message::Account(account_write) => {
                        if let Some(_mango_account) = is_mango_account(&account_write.account, &mango_program_id, &mango_group_id) {
                            // Track all MangoAccounts: we need to iterate over them later
                            mango_accounts.insert(account_write.pubkey);
                            metric_mango_accounts.set(mango_accounts.len() as u64);

                            if !one_snapshot_done {
                                continue;
                            }
                            if let Err(err) = healthcheck::process_accounts(
                                    &config,
                                    &chain_data,
                                    &mango_group_id,
                                    &mango_cache_id,
                                    std::iter::once(&account_write.pubkey),
                                    &mut current_candidates,
                                    &liquidation_candidate_sender,
                            ) {
                                warn!("could not process account {}: {:?}", account_write.pubkey, err);
                            }
                        }

                        if account_write.pubkey == mango_cache_id && is_mango_cache(&account_write.account, &mango_program_id) {
                            if !one_snapshot_done {
                                continue;
                            }

                            // check health of all accounts
                            //
                            // TODO: This could be done asynchronously by calling
                            // let accounts = chain_data.accounts_snapshot();
                            // and then working with the snapshot of the data
                            //
                            // However, this currently takes like 50ms for me in release builds,
                            // so optimizing much seems unnecessary.
                            if let Err(err) = healthcheck::process_accounts(
                                    &config,
                                    &chain_data,
                                    &mango_group_id,
                                    &mango_cache_id,
                                    mango_accounts.iter(),
                                    &mut current_candidates,
                                    &liquidation_candidate_sender,
                            ) {
                                warn!("could not process accounts: {:?}", err);
                            }
                        }
                    }
                    _ => {}
                }
            },
            message = snapshot_receiver.recv() => {
                metric_snapshot_queue_len.set(snapshot_receiver.len() as u64);
                let message = message.expect("channel not closed");

                // Track all mango account pubkeys
                for update in message.accounts.iter() {
                    if let Some(_mango_account) = is_mango_account(&update.account, &mango_program_id, &mango_group_id) {
                        mango_accounts.insert(update.pubkey);
                    }
                }
                metric_mango_accounts.set(mango_accounts.len() as u64);

                chain_data.update_from_snapshot(message);
                one_snapshot_done = true;

                // TODO: trigger a full health check
            },
        }
    }
}
