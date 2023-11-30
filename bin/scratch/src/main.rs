use std::str::FromStr;
use std::sync::Arc;
use mango_v4_client::{Client, MangoClient, MangoGroupContext, chain_data, JupiterSwapMode};
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::pubkey::Pubkey;
use mango_v4_client::chain_data::ChainData;
use std::sync::RwLock;
use anchor_client::Cluster;


#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let kp = Arc::new(Keypair::new());

    let client = Client::builder().cluster(Cluster::Mainnet).fee_payer(Some(kp.clone())).jupiter_url(.to_string()).build().unwrap();

        // The representation of current on-chain account data
        let chain_data = Arc::new(RwLock::new(chain_data::ChainData::new()));
        // Reading accounts from chain_data
        let account_fetcher = Arc::new(chain_data::AccountFetcher {
            chain_data: chain_data.clone(),
            rpc: client.rpc_async(),
        });
        
    let mango_key = Pubkey::from_str("").unwrap();
    let mango_account = account_fetcher
        .fetch_fresh_mango_account(&mango_key)
        .await.unwrap();
    let mango_group = mango_account.fixed.group;

    let group_context = MangoGroupContext::new_from_rpc(&client.rpc_async(), mango_group).await.unwrap();
    let mango_client = MangoClient::new_detail(client, mango_key, kp, group_context, account_fetcher).unwrap();

    let quote = mango_client.jupiter_v6().quote(Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(), Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(), 100, 50, false).await.unwrap();
    println!("{:?}", quote);


    // let q2 = mango_client.jupiter_v4().quote(Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(), Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(), 100, 50, JupiterSwapMode::ExactIn, false).await.unwrap();
    // println!("{:?}", q2);
}
