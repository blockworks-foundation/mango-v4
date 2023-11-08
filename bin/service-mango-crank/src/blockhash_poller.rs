use log::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{clock::DEFAULT_MS_PER_SLOT, commitment_config::CommitmentConfig, hash::Hash};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{spawn, time::sleep};

const RETRY_INTERVAL: Duration = Duration::from_millis(5 * DEFAULT_MS_PER_SLOT);

pub async fn poll_loop(blockhash: Arc<RwLock<Hash>>, client: Arc<RpcClient>) {
    let cfg = CommitmentConfig::processed();
    loop {
        let old_blockhash = *blockhash.read().unwrap();
        if let Ok((new_blockhash, _)) = client.get_latest_blockhash_with_commitment(cfg).await {
            if new_blockhash != old_blockhash {
                debug!("new blockhash ({:?})", blockhash);
                *blockhash.write().unwrap() = new_blockhash;
            }
        }

        // Retry every few slots
        sleep(RETRY_INTERVAL).await;
    }
}

pub async fn init(client: Arc<RpcClient>) -> Arc<RwLock<Hash>> {
    // get the first blockhash
    let blockhash = Arc::new(RwLock::new(
        client
            .get_latest_blockhash()
            .await
            .expect("fetch initial blockhash"),
    ));

    // launch task
    let _join_hdl = {
        // create a thread-local reference to blockhash
        let blockhash_c = blockhash.clone();
        spawn(async move { poll_loop(blockhash_c, client).await })
    };

    blockhash
}
