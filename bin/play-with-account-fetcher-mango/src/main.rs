
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU32, Ordering};
use anchor_lang::Key;
use anchor_lang::solana_program::clock::Slot;
use mango_feeds_connector::chain_data;
use mango_feeds_connector::account_fetcher::{AccountFetcherFeeds};
use mango_feeds_connector::account_fetchers::{CachedAccountFetcher, RpcAccountFetcher};
use mango_feeds_connector::feeds_chain_data_fetcher::FeedsAccountFetcher;
use tracing::info;
use solana_client::nonblocking::rpc_client::{RpcClient as RpcClientAsync, RpcClient};
use solana_sdk::account::{AccountSharedData, ReadableAccount};
use solana_sdk::clock::UnixTimestamp;
use solana_sdk::epoch_info::EpochInfo;
use solana_sdk::pubkey::Pubkey;
use mango_v4::state::{MangoAccountValue, PerpMarket};
use mango_v4_client::account_fetcher_utils::{account_fetcher_fetch_anchor_account, account_fetcher_fetch_mango_account};
use mango_v4_client::{AccountFetcher, chain_data_fetcher};


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

    call_cache_with_mock_error(mango_account_pk).await;

}

async fn chain_data_fetcher(rpc_url: String) {
    let rpc_client = RpcClientAsync::new(rpc_url);

    let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
    let account_fetcher = Arc::new(chain_data_fetcher::ClientChainDataAccountFetcher {
        base_fetcher: FeedsAccountFetcher {
            chain_data: chain_data.clone()
        },
        rpc: rpc_client,
    });

    let account_key = Pubkey::from_str("phxBcughCYKiYJxx9kYEkyqoAUL2RD3vyxSaL1gZRNG").unwrap();

    account_fetcher.refresh_account_via_rpc(&account_key).await.unwrap();

    let price: anyhow::Result<AccountSharedData> = account_fetcher.fetch_raw_account(&account_key).await;
    println!("price: {:?}", price);
}


/// note: sometime the call is flakey
async fn chain_data_fetcher_bank(rpc_url: String) {
    let rpc_client = RpcClientAsync::new(rpc_url);

    let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
    let account_fetcher = Arc::new(chain_data_fetcher::ClientChainDataAccountFetcher {
        base_fetcher: FeedsAccountFetcher {
            chain_data: chain_data.clone()
        },
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
    fetched_mango_calls: AtomicU32,
    scenario: Scenario,
}

#[derive(Clone, Copy)]
enum Scenario {
    Happy,
    Error,
}

impl MockExampleFetcher {


    pub fn new(scenario: Scenario) -> Self {
            Self {
            fetched_mango_calls: AtomicU32::new(0),
            scenario,
        }
    }

    pub fn assert_call_count(&self, expected: u32) {
        assert_eq!(self.fetched_mango_calls.load(Ordering::Acquire), expected);
    }

    fn inc_call(&self) {
        self.fetched_mango_calls.fetch_add(1, Ordering::Release);
    }
}

#[async_trait::async_trait]
impl AccountFetcherFeeds for MockExampleFetcher {
    async fn feeds_fetch_raw_account(
        &self, _address: &Pubkey,
    ) -> anyhow::Result<(AccountSharedData, Slot)> {

        self.inc_call();

        match self.scenario {

            Scenario::Happy => {
                let account_owner =
                    Pubkey::from_str("66fEFnKyCPUWzxKeB9GngcvZDakjvFCVnYLRtcBk9t5D").unwrap();
                let acc = AccountSharedData::new(420000, 0, &account_owner);
                return Ok((acc, 2409999333));
            }

            Scenario::Error => {
                return Err(anyhow::anyhow!("simulated error in mock fetcher"));
            }

        }

    }

    async fn feeds_fetch_program_accounts(
        &self,
        _program: &Pubkey,
        _discriminator: [u8; 8],
    ) -> anyhow::Result<(Vec<(Pubkey, AccountSharedData)>, Slot)> {
        unreachable!("program accounts not mocked")
    }

}


async fn call_cache_with_mock(account: Pubkey,) {

    let mut mock = Arc::new(MockExampleFetcher::new(Scenario::Happy));

    let cache = CachedAccountFetcher::new(mock.clone());
    mock.assert_call_count(0);

    let _first_call = cache.fetch_raw_account(&account).await;
    mock.assert_call_count(1);

    // must load from cache
    let _second_call_cached = cache.fetch_raw_account(&account).await;
    mock.assert_call_count(1);

    cache.clear_cache();
    let _third_call_cached = cache.fetch_raw_account(&account).await;;
    mock.assert_call_count(2);
}


async fn call_cache_with_mock_error(account: Pubkey,) {

    let mut mock = Arc::new(MockExampleFetcher::new(Scenario::Error));

    let cache = CachedAccountFetcher::new(mock.clone());
    mock.assert_call_count(0);

    let _first_call = cache.fetch_raw_account(&account).await;
    mock.assert_call_count(1);

    // must hit wrapped fetcher again on error
    let _second_call_cached = cache.fetch_raw_account(&account).await;
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
        account_fetcher_fetch_mango_account(&*cachedaccount_fetcher, &account).await.unwrap();
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
        account_fetcher_fetch_mango_account(&*account_fetcher, &account).await.unwrap();
    info!("mango account loaded");
}

pub async fn load_anchor_account(
    rpc_url: String,
    account: Pubkey,
) {
    let rpc_client = RpcClientAsync::new(rpc_url);

    let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
    let account_fetcher = Arc::new(chain_data_fetcher::ClientChainDataAccountFetcher {
        base_fetcher: FeedsAccountFetcher {
            chain_data: chain_data.clone()
        },
        rpc: rpc_client,
    });

    account_fetcher.refresh_account_via_rpc(&account).await.unwrap();

    let perp_market: PerpMarket =
        account_fetcher_fetch_anchor_account::<PerpMarket>(&*account_fetcher, &account).await.unwrap();
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
