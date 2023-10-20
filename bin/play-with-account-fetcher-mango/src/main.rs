
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use anchor_lang::Key;
use chrono::{TimeZone, Utc};
use futures_util::TryFutureExt;
use mango_feeds_connector::{account_fetcher, chain_data, chain_data_fetcher};
use mango_feeds_connector::account_fetcher::{CachedAccountFetcher, RpcAccountFetcher};
use mango_feeds_connector::account_fetcher_trait::AccountFetcher;
use tracing::{info, trace};
use solana_client::nonblocking::rpc_client::{RpcClient as RpcClientAsync, RpcClient};
use solana_sdk::account::{AccountSharedData, ReadableAccount};
use solana_sdk::clock;
use solana_sdk::clock::UnixTimestamp;
use solana_sdk::epoch_info::EpochInfo;
use solana_sdk::pubkey::Pubkey;
use mango_v4::state::{MangoAccountValue, PerpMarket};
use mango_v4_client::mango_account_fetcher;


#[tokio::main]
async fn main() {
    tracing_subscriber_init();

    let rpc_url: String = "https://api.mainnet-beta.solana.com/".to_string();
    // let rpc_url: String = "https://api.devnet.solana.com/".to_string();
    let mango_account_pk: Pubkey = Pubkey::from_str("7v8bovqsYfFfEeiXnGLiGTg2VJAn62hSoSCPidKjKL8w").unwrap();

    // https://app.mango.markets/dashboard
    // PERP-SOL
    let perp_account_pk: Pubkey = Pubkey::from_str("ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2").unwrap();

    chain_data_fetcher(rpc_url.clone()).await;

    chain_data_fetcher_bank(rpc_url.clone()).await;

    load_mango_account_cached(rpc_url.clone(), mango_account_pk).await;

    load_mango_account(rpc_url.clone(), mango_account_pk).await;

    load_anchor_account(rpc_url.clone(), perp_account_pk).await;

    call_cache_with_mock(mango_account_pk).await;

}

async fn chain_data_fetcher(rpc_url: String) {
    let rpc_client = RpcClientAsync::new(rpc_url);

    let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
    let account_fetcher = Arc::new(chain_data_fetcher::ChainDataFetcher {
        chain_data: chain_data.clone(),
        rpc: rpc_client,
    });

    let account_key = Pubkey::from_str("J6MsZiJUU6bjKSCkbfQsiHkd8gvJoddG2hsdSFsZQEZV").unwrap();
    let price: anyhow::Result<AccountSharedData> = account_fetcher.fetch_raw_account(&account_key).await;
    println!("price: {:?}", price);
}


/// note: sometime the call is flakey
async fn chain_data_fetcher_bank(rpc_url: String) {
    let rpc_client = RpcClientAsync::new(rpc_url);

    let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
    let account_fetcher = Arc::new(chain_data_fetcher::ChainDataFetcher {
        chain_data: chain_data.clone(),
        rpc: rpc_client,
    });
    let bank = Pubkey::from_str("J6MsZiJUU6bjKSCkbfQsiHkd8gvJoddG2hsdSFsZQEZV").unwrap();

    let current_slot = account_fetcher.refresh_account_via_rpc(&bank).await.unwrap();
    info!("current_slot: {:?}", current_slot);

    let account_data: AccountSharedData = account_fetcher.fetch_raw_account(&bank).await.unwrap();
    info!("owner: {:?}", account_data.owner().key());
    info!("lamports: {:?}", account_data.lamports());
}

struct MockExampleFetcher {
    pub fetched_mango_calls: AtomicU32,
}

impl MockExampleFetcher {

    pub fn new() -> Self {
        Self {
            fetched_mango_calls: AtomicU32::new(0),
        }
    }

    pub fn assert_call_count(&self, expected: u32) {
        assert_eq!(self.fetched_mango_calls.load(Ordering::SeqCst), expected);
    }

}

#[async_trait::async_trait]
impl AccountFetcher for MockExampleFetcher {
    async fn fetch_raw_account(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        panic!()
    }

    async fn fetch_raw_account_lookup_table(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        panic!()
    }

    async fn fetch_program_accounts(&self, program: &Pubkey, discriminator: [u8; 8]) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>> {
        let call_count = self.fetched_mango_calls.fetch_add(1, Ordering::SeqCst) + 1;
        info!("Call to mocked fetch_program_accounts... {}", call_count);

        Ok(vec![])
    }
}




async fn call_cache_with_mock(account: Pubkey,) {

    let mut mock = Arc::new(MockExampleFetcher::new());

    let mock_fetcher = CachedAccountFetcher::new(mock.clone());
    mock.assert_call_count(0);

    let first_call = mock_fetcher.fetch_program_accounts(&account, [0; 8]).await.unwrap();
    mock.assert_call_count(1);

    let second_call_cached = mock_fetcher.fetch_program_accounts(&account, [0; 8]).await.unwrap();
    mock.assert_call_count(1);

    mock_fetcher.clear_cache();
    let third_call_cached = mock_fetcher.fetch_program_accounts(&account, [0; 8]).await.unwrap();
    mock.assert_call_count(2);
}


pub async fn load_mango_account_cached(
    rpc_url: String,
    account: Pubkey,
) {
    let rpc_client = RpcClientAsync::new(rpc_url);

    let cachedaccount_fetcher = Arc::new(CachedAccountFetcher::new(Arc::new(RpcAccountFetcher {
        rpc: rpc_client,
    })));
    let _mango_account: MangoAccountValue =
        mango_account_fetcher::account_fetcher_fetch_mango_account(&*cachedaccount_fetcher, &account).await.unwrap();
    info!("mango account loaded cached");
}


pub async fn load_mango_account(
    rpc_url: String,
    account: Pubkey,
) {
    let rpc_client = RpcClientAsync::new(rpc_url);

    let account_fetcher = Arc::new(RpcAccountFetcher {
        rpc: rpc_client,
    });
    let _mango_account: MangoAccountValue =
        mango_account_fetcher::account_fetcher_fetch_mango_account(&*account_fetcher, &account).await.unwrap();
    info!("mango account loaded");
}

pub async fn load_anchor_account(
    rpc_url: String,
    account: Pubkey,
) {
    let rpc_client = RpcClientAsync::new(rpc_url);

    let account_fetcher = Arc::new(CachedAccountFetcher::new(Arc::new(RpcAccountFetcher {
        rpc: rpc_client,
    })));
    let perp_market: PerpMarket =
        account_fetcher::account_fetcher_fetch_anchor_account::<PerpMarket>(&*account_fetcher, &account).await.unwrap();
    info!("perp account loaded: base_decimals={}", perp_market.base_decimals);
}

fn instances(rpc1: RpcClientAsync, rpc2: RpcClientAsync, rpc3: RpcClientAsync) {

    let account_fetcher = Arc::new(CachedAccountFetcher::new(Arc::new(RpcAccountFetcher {
        rpc: rpc1,
    })));

    let _ = Arc::new(CachedAccountFetcher::new(Arc::new(RpcAccountFetcher {
        rpc: rpc2,
    })));

    let _ = RpcAccountFetcher {
        rpc: rpc3,
    };


}

pub fn tracing_subscriber_init() {
    let format = tracing_subscriber::fmt::format().with_ansi(atty::is(atty::Stream::Stdout));

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .event_format(format)
        .init();
}

#[derive(Debug)]
pub struct CliEpochInfo {
    pub epoch_info: EpochInfo,
    pub epoch_completed_percent: f64,
    pub average_slot_time_ms: u64,
    pub start_block_time: Option<UnixTimestamp>,
    pub current_block_time: Option<UnixTimestamp>,
}
