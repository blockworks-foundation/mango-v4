use std::sync::{Arc, RwLock};
use anchor_client::Cluster;
use anyhow::Context;
use std::time::Duration;
use mango_v4_client::{chain_data, Client, MangoClient};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::read_keypair_file};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ws_url = rpc_url.replace("https", "wss");
    let rpc_timeout = Duration::from_secs(10);
    let cluster = Cluster::Custom(rpc_url.clone(), ws_url.clone());
    let commitment = CommitmentConfig::processed();
    let client = Client::builder()
        .cluster(cluster.clone())
        .commitment(commitment)
        .fee_payer(Some(keypair.clone()))
        .timeout(rpc_timeout)
        .build()
        .unwrap();

    // The representation of current on-chain account data
    let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
    // Reading accounts from chain_data
    let account_fetcher = Arc::new(chain_data::AccountFetcher {
        chain_data: chain_data.clone(),
        rpc: client.new_rpc_async(),
    });

    let mango_client = MangoClient::new_for_existing_account(client, account, owner)

    // let mango_account_keys = client.


    // loop {

    // }
    

    Ok(())
}

pub async fn health_cache(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    pubkey: &Pubkey
) -> anyhow::Result<()> {
    let account = account_fetcher.fetch_mango_account(pubkey)?;
    let health_cache = mango_client
        .health_cache(&account)
        .await
        .context("creating health cache 1")?;

    Ok(())
}
