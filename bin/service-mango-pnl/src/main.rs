mod memory_target;

use {
    log::*,
    mango_feeds_connector::chain_data::ChainData,
    serde_derive::{Deserialize, Serialize},
    solana_sdk::pubkey::Pubkey,
    std::str::FromStr,
    std::{
        fs::File,
        io::Read,
        mem::size_of,
        sync::{atomic::AtomicBool, Arc, RwLock},
        time::Duration,
    },
};

use anchor_client::Cluster;
use anchor_lang::Discriminator;
use fixed::types::I80F48;
use mango_feeds_connector::metrics::*;
use mango_v4::state::{MangoAccount, MangoAccountValue, PerpMarketIndex};
use mango_v4_client::{
    chain_data, health_cache, AccountFetcher, Client, FallbackOracleConfig, MangoGroupContext,
    TransactionBuilderConfig,
};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::{account::ReadableAccount, signature::Keypair};
#[derive(Clone, Debug, Deserialize)]
pub struct PnlConfig {
    pub update_interval_millis: u64,
    pub mango_program: String,
    pub mango_group: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct JsonRpcConfig {
    pub bind_address: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub source: SourceConfig,
    pub metrics: MetricsConfig,
    pub pnl: PnlConfig,
    pub jsonrpc_server: JsonRpcConfig,
}

type PnlData = Vec<(Pubkey, Vec<(PerpMarketIndex, I80F48)>)>;

async fn compute_pnl(
    context: Arc<MangoGroupContext>,
    account_fetcher: Arc<impl AccountFetcher>,
    account: &MangoAccountValue,
) -> anyhow::Result<Vec<(PerpMarketIndex, I80F48)>> {
    let health_cache = health_cache::new(
        &context,
        &FallbackOracleConfig::Dynamic,
        account_fetcher.as_ref(),
        account,
    )
    .await?;

    let pnls = account
        .active_perp_positions()
        .filter_map(|pp| {
            if pp.base_position_lots() != 0 {
                return None;
            }
            let pnl = pp.quote_position_native();
            let settle_token_index = context
                .perp_markets
                .get(&pp.market_index)
                .unwrap()
                .settle_token_index;
            let perp_settle_health = health_cache.perp_max_settle(settle_token_index).unwrap();
            let settleable_pnl = if pnl > 0 {
                pnl
            } else if pnl < 0 && perp_settle_health > 0 {
                pnl.max(-perp_settle_health)
            } else {
                return None;
            };
            Some((pp.market_index, I80F48::from_bits(settleable_pnl.to_bits())))
        })
        .collect::<Vec<(PerpMarketIndex, I80F48)>>();

    Ok(pnls)
}

// regularly updates pnl_data from chain_data
fn start_pnl_updater(
    config: PnlConfig,
    context: Arc<MangoGroupContext>,
    account_fetcher: Arc<impl AccountFetcher + 'static>,
    chain_data: Arc<RwLock<ChainData>>,
    pnl_data: Arc<RwLock<PnlData>>,
    metrics_pnls_tracked: MetricU64,
) {
    let program_pk = Pubkey::from_str(&config.mango_program).unwrap();
    let group_pk = Pubkey::from_str(&config.mango_group).unwrap();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(
                config.update_interval_millis,
            ))
            .await;

            let snapshot = chain_data.read().unwrap().accounts_snapshot();

            // get the group and cache now
            let group = snapshot.get(&group_pk);
            if group.is_none() {
                continue;
            }

            let mut pnls = Vec::with_capacity(snapshot.len());
            for (pubkey, account) in snapshot.iter() {
                let owner = account.account.owner();
                let data = account.account.data();

                if data.len() != size_of::<MangoAccount>()
                    || data[0..8] != MangoAccount::discriminator()
                    || owner != &program_pk
                {
                    continue;
                }

                let mango_account = MangoAccountValue::from_bytes(&data[8..]).unwrap();
                if mango_account.fixed.group != group_pk {
                    continue;
                }

                let pnl_vals =
                    compute_pnl(context.clone(), account_fetcher.clone(), &mango_account)
                        .await
                        .unwrap();

                // Alternatively, we could prepare the sorted and limited lists for each
                // market here. That would be faster and cause less contention on the pnl_data
                // lock, but it looks like it's very far from being an issue.
                pnls.push((*pubkey, pnl_vals));
            }

            *pnl_data.write().unwrap() = pnls;
            metrics_pnls_tracked
                .clone()
                .set(pnl_data.read().unwrap().len() as u64)
        }
    });
}

#[derive(Serialize, Deserialize, Debug)]
struct UnsettledPnlRankedRequest {
    market_index: u8,
    limit: u8,
    order: String,
}

#[derive(Serialize, Deserialize)]
struct PnlResponseItem {
    pnl: f64,
    pubkey: String,
}

use jsonrpsee::http_server::HttpServerHandle;
use mango_feeds_connector::{
    grpc_plugin_source, metrics, EntityFilter, FilterConfig, MetricsConfig, SourceConfig,
};

fn start_jsonrpc_server(
    config: JsonRpcConfig,
    pnl_data: Arc<RwLock<PnlData>>,
    metrics_reqs: MetricU64,
    metrics_invalid_reqs: MetricU64,
) -> anyhow::Result<HttpServerHandle> {
    use jsonrpsee::core::Error;
    use jsonrpsee::http_server::{HttpServerBuilder, RpcModule};
    use jsonrpsee::types::error::CallError;
    use std::net::SocketAddr;

    let server = HttpServerBuilder::default().build(config.bind_address.parse::<SocketAddr>()?)?;
    let mut module = RpcModule::new(());
    module.register_method("unsettledPnlRanked", move |params, _| {
        let req = params.parse::<UnsettledPnlRankedRequest>()?;
        metrics_reqs.clone().increment();
        let invalid =
            |s: &'static str| Err(Error::Call(CallError::InvalidParams(anyhow::anyhow!(s))));
        let limit = req.limit as usize;
        if limit > 20 {
            metrics_invalid_reqs.clone().increment();
            return invalid("'limit' must be <= 20");
        }
        let market_index = req.market_index as u16;
        // if market_index >= MAX_PAIRS {
        //     metrics_invalid_reqs.clone().increment();
        //     return invalid("'market_index' must be < MAX_PAIRS");
        // }
        if req.order != "ASC" && req.order != "DESC" {
            metrics_invalid_reqs.clone().increment();
            return invalid("'order' must be ASC or DESC");
        }

        // write lock, because we sort in-place...
        let mut pnls = pnl_data.write().unwrap();
        if req.order == "ASC" {
            pnls.sort_unstable_by(|a, b| {
                a.1.iter()
                    .find(|x| x.0 == market_index)
                    .cmp(&b.1.iter().find(|x| x.0 == market_index))
            });
        } else {
            pnls.sort_unstable_by(|a, b| {
                b.1.iter()
                    .find(|x| x.0 == market_index)
                    .cmp(&a.1.iter().find(|x| x.0 == market_index))
            });
        }
        let response = pnls
            .iter()
            .take(limit)
            .map(|p| PnlResponseItem {
                pnl: p
                    .1
                    .iter()
                    .find(|x| x.0 == market_index)
                    .unwrap()
                    .1
                    .to_num::<f64>(),
                pubkey: p.0.to_string(),
            })
            .collect::<Vec<_>>();

        Ok(response)
    })?;

    Ok(server.start(module)?)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let exit: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

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

    solana_logger::setup_with_default("info");
    info!("startup");

    let rpc_url = &config.source.snapshot.rpc_http_url;
    let ws_url = rpc_url.replace("https", "wss");
    let rpc_timeout = Duration::from_secs(10);
    let cluster = Cluster::Custom(rpc_url.clone(), ws_url.clone());
    let commitment = CommitmentConfig::processed();
    let client = Client::new(
        cluster.clone(),
        commitment,
        Arc::new(Keypair::new()),
        Some(rpc_timeout),
        TransactionBuilderConfig::default(),
    );
    let group_context = Arc::new(
        MangoGroupContext::new_from_rpc(
            client.rpc_async(),
            Pubkey::from_str(&config.pnl.mango_group).unwrap(),
        )
        .await?,
    );
    let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
    let account_fetcher = Arc::new(chain_data::AccountFetcher {
        chain_data: chain_data.clone(),
        rpc: client.new_rpc_async(),
    });

    let metrics_tx = metrics::start(config.metrics, "pnl".into());

    let metrics_reqs =
        metrics_tx.register_u64("pnl_jsonrpc_reqs_total".into(), MetricType::Counter);
    let metrics_invalid_reqs =
        metrics_tx.register_u64("pnl_jsonrpc_reqs_invalid_total".into(), MetricType::Counter);
    let metrics_pnls_tracked = metrics_tx.register_u64("pnl_num_tracked".into(), MetricType::Gauge);

    // BUG: This shadows the previous chain_data and means this can't actually get data!
    let chain_data = Arc::new(RwLock::new(ChainData::new()));
    let pnl_data = Arc::new(RwLock::new(PnlData::new()));

    start_pnl_updater(
        config.pnl.clone(),
        group_context.clone(),
        account_fetcher.clone(),
        chain_data.clone(),
        pnl_data.clone(),
        metrics_pnls_tracked,
    );

    // dropping the handle would exit the server
    let _http_server_handle = start_jsonrpc_server(
        config.jsonrpc_server.clone(),
        pnl_data,
        metrics_reqs,
        metrics_invalid_reqs,
    )?;

    // start filling chain_data from the grpc plugin source
    let (account_write_queue_sender, slot_queue_sender) = memory_target::init(chain_data).await?;
    let filter_config = FilterConfig {
        entity_filter: EntityFilter::filter_by_program_id(
            "4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg",
        ),
    };
    grpc_plugin_source::process_events(
        &config.source,
        &filter_config,
        account_write_queue_sender,
        slot_queue_sender,
        metrics_tx.clone(),
        exit.clone(),
    )
    .await;

    Ok(())
}
