mod blockhash_poller;
mod mango_v4_perp_crank_sink;
mod openbook_crank_sink;
mod transaction_builder;
mod transaction_sender;

use anchor_client::Cluster;
use bytemuck::bytes_of;
use log::*;
use mango_v4_client::{Client, MangoGroupContext, TransactionBuilderConfig};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::iter::FromIterator;
use std::{
    collections::HashSet,
    convert::TryFrom,
    fs::File,
    io::Read,
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use mango_feeds_connector::EntityFilter::FilterByAccountIds;
use mango_feeds_connector::FilterConfig;
use mango_feeds_connector::{
    grpc_plugin_source, metrics, websocket_source, MetricsConfig, SourceConfig,
};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub source: SourceConfig,
    pub metrics: MetricsConfig,
    pub bind_ws_addr: String,
    pub rpc_http_url: String,
    pub mango_group: String,
    pub keypair: Vec<u8>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    solana_logger::setup_with_default("info");

    let exit: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        error!("Please enter a config file path argument.");
        return Ok(());
    }

    let config: Config = {
        let mut file = File::open(&args[1])?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        toml::from_str(&contents).unwrap()
    };

    let rpc_client = Arc::new(RpcClient::new(config.rpc_http_url.clone()));

    let blockhash = blockhash_poller::init(rpc_client.clone()).await;

    let metrics_tx = metrics::start(config.metrics, "crank".into());

    let rpc_url = config.rpc_http_url;
    let ws_url = rpc_url.replace("https", "wss");
    let rpc_timeout = Duration::from_secs(10);
    let cluster = Cluster::Custom(rpc_url.clone(), ws_url.clone());
    let client = Client::new(
        cluster.clone(),
        CommitmentConfig::processed(),
        Arc::new(Keypair::new()),
        Some(rpc_timeout),
        TransactionBuilderConfig::default(),
    );
    let group_pk = Pubkey::from_str(&config.mango_group).unwrap();
    let group_context =
        Arc::new(MangoGroupContext::new_from_rpc(client.rpc_async(), group_pk).await?);

    let perp_queue_pks: Vec<_> = group_context
        .perp_markets
        .values()
        .map(|context| (context.address, context.event_queue))
        .collect();

    // fetch all serum/openbook markets to find their event queues
    let serum_market_pks: Vec<_> = group_context
        .serum3_markets
        .values()
        .map(|context| context.serum_market_external)
        .collect();

    let serum_market_ais = client
        .rpc_async()
        .get_multiple_accounts(serum_market_pks.as_slice())
        .await?;

    let serum_market_ais: Vec<_> = serum_market_ais
        .iter()
        .filter_map(|maybe_ai| match maybe_ai {
            Some(ai) => Some(ai),
            None => None,
        })
        .collect();

    let serum_queue_pks: Vec<_> = serum_market_ais
        .iter()
        .enumerate()
        .map(|pair| {
            let market_state: serum_dex::state::MarketState = *bytemuck::from_bytes(
                &pair.1.data[5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
            );
            let event_q = market_state.event_q;
            (
                serum_market_pks[pair.0],
                Pubkey::try_from(bytes_of(&event_q)).unwrap(),
            )
        })
        .collect();

    let (account_write_queue_sender, slot_queue_sender, instruction_receiver) =
        transaction_builder::init(
            perp_queue_pks.clone(),
            serum_queue_pks.clone(),
            group_pk,
            metrics_tx.clone(),
        )
        .expect("init transaction builder");

    transaction_sender::init(
        instruction_receiver,
        blockhash,
        rpc_client,
        Keypair::from_bytes(&config.keypair).expect("valid keyair in config"),
    );

    info!(
        "connect: {}",
        config
            .source
            .grpc_sources
            .iter()
            .map(|c| c.connection_string.clone())
            .collect::<String>()
    );
    let use_geyser = true;
    let all_queue_pks: HashSet<Pubkey> = perp_queue_pks
        .iter()
        .chain(serum_queue_pks.iter())
        .map(|mkt| mkt.1)
        .collect();

    let filter_config = FilterConfig {
        entity_filter: FilterByAccountIds(Vec::from_iter(all_queue_pks)),
    };
    if use_geyser {
        grpc_plugin_source::process_events(
            &config.source,
            &filter_config,
            account_write_queue_sender,
            slot_queue_sender,
            metrics_tx.clone(),
            exit.clone(),
        )
        .await;
    } else {
        websocket_source::process_events(
            &config.source,
            &filter_config,
            account_write_queue_sender,
            slot_queue_sender,
        )
        .await;
    }

    Ok(())
}
