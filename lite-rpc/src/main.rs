use std::time::Duration;

use anyhow::bail;
use clap::Parser;
use dotenv::dotenv;
use lite_rpc::{bridge::LiteBridge, cli::Args};
use log::info;
use solana_sdk::signature::Keypair;
use std::env;
use solana_lite_rpc_quic_forward_proxy::proxy::QuicForwardProxy;
use solana_lite_rpc_quic_forward_proxy::SelfSignedTlsConfigProvider;
use solana_lite_rpc_quic_forward_proxy::test_client::quic_test_client::QuicTestClient;
// use lite_rpc_quic_forward_proxy::tls_config::SelfSignedTlsConfigProvider;

async fn get_identity_keypair(identity_from_cli: &String) -> Keypair {
    if let Ok(identity_env_var) = env::var("IDENTITY") {
        if let Ok(identity_bytes) = serde_json::from_str::<Vec<u8>>(identity_env_var.as_str()) {
            Keypair::from_bytes(identity_bytes.as_slice()).unwrap()
        } else {
            // must be a file
            let identity_file = tokio::fs::read_to_string(identity_env_var.as_str())
                .await
                .expect("Cannot find the identity file provided");
            let identity_bytes: Vec<u8> = serde_json::from_str(&identity_file).unwrap();
            Keypair::from_bytes(identity_bytes.as_slice()).unwrap()
        }
    } else if identity_from_cli.is_empty() {
        Keypair::new()
    } else {
        let identity_file = tokio::fs::read_to_string(identity_from_cli.as_str())
            .await
            .expect("Cannot find the identity file provided");
        let identity_bytes: Vec<u8> = serde_json::from_str(&identity_file).unwrap();
        Keypair::from_bytes(identity_bytes.as_slice()).unwrap()
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
pub async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let Args {
        rpc_addr,
        ws_addr,
        lite_rpc_ws_addr,
        lite_rpc_http_addr,
        clean_interval_ms,
        fanout_size,
        enable_postgres,
        prometheus_addr,
        identity_keypair,
        maximum_retries_per_tx,
        transaction_retry_after_secs,
    } = Args::parse();

    dotenv().ok();

    let identity = get_identity_keypair(&identity_keypair).await;

    let clean_interval_ms = Duration::from_millis(clean_interval_ms);

    let enable_postgres = enable_postgres
        || if let Ok(enable_postgres_env_var) = env::var("PG_ENABLED") {
            enable_postgres_env_var != "false"
        } else {
            false
        };

    let retry_after = Duration::from_secs(transaction_retry_after_secs);


    let proxy_listener_addr = "127.0.0.1:11111".parse().unwrap();
    let tls_configuration = SelfSignedTlsConfigProvider::new_singleton_self_signed_localhost();
    let quicproxy_service = QuicForwardProxy::new(proxy_listener_addr, &tls_configuration)
        .await?
        .start_services();


    let services = LiteBridge::new(
        rpc_addr,
        ws_addr,
        fanout_size,
        identity,
        retry_after,
        maximum_retries_per_tx,
    )
    .await?
    .start_services(
        lite_rpc_http_addr,
        lite_rpc_ws_addr,
        clean_interval_ms,
        enable_postgres,
        prometheus_addr,
    );

    let proxy_addr = "127.0.0.1:11111".parse().unwrap();
    let test_client = QuicTestClient::new_with_endpoint(
        proxy_addr, &tls_configuration)
        .await?
        .start_services();

    let ctrl_c_signal = tokio::signal::ctrl_c();

    tokio::select! {
        res = services => {
            bail!("Services quit unexpectedly {res:?}");
        },
        res = quicproxy_service => {
            bail!("Quic Proxy quit unexpectedly {res:?}");
        },
        res = test_client => {
            bail!("Test Client quit unexpectedly {res:?}");
        },
        _ = ctrl_c_signal => {
            info!("Received ctrl+c signal");

            Ok(())
        }
    }
}
