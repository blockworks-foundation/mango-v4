use log::*;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_sdk::{
    hash::Hash, instruction::Instruction, signature::Keypair, signature::Signer,
    transaction::Transaction,
};
use std::sync::{Arc, RwLock};
use tokio::spawn;

pub async fn send_loop(
    ixs_rx: async_channel::Receiver<Vec<Instruction>>,
    blockhash: Arc<RwLock<Hash>>,
    client: Arc<RpcClient>,
    keypair: Keypair,
) {
    info!("signing with keypair pk={:?}", keypair.pubkey());
    let cfg = RpcSendTransactionConfig {
        skip_preflight: true,
        ..RpcSendTransactionConfig::default()
    };
    loop {
        if let Ok(ixs) = ixs_rx.recv().await {
            // TODO add priority fee
            let tx = Transaction::new_signed_with_payer(
                &ixs,
                Some(&keypair.pubkey()),
                &[&keypair],
                *blockhash.read().unwrap(),
            );
            // TODO: collect metrics
            info!(
                "send tx={:?} ok={:?}",
                tx.signatures[0],
                client.send_transaction_with_config(&tx, cfg).await
            );
        }
    }
}

pub fn init(
    ixs_rx: async_channel::Receiver<Vec<Instruction>>,
    blockhash: Arc<RwLock<Hash>>,
    client: Arc<RpcClient>,
    keypair: Keypair,
) {
    spawn(async move { send_loop(ixs_rx, blockhash, client, keypair).await });
}
