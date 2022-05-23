mod crank;
mod mango_client;
mod taker;
mod util;

use std::env;
use std::sync::Arc;

use anchor_client::Cluster;

use clap::{Parser, Subcommand};

use solana_sdk::{commitment_config::CommitmentConfig, signer::keypair};
use tokio::time;

use crate::mango_client::MangoClient;

// TODO
// - may be nice to have one-shot cranking as well as the interval cranking
// - doing a gPA for all banks call every 10millis may be too often,
// might make sense that we maintain a service when users should query group for changes
// - I'm really annoyed about Keypair not being clonable. Seems everyone works around that manually. Should make a PR to solana to newtype it and provide that function.
// keypair_from_arg_or_env could be a function

#[derive(Parser)]
#[clap()]
struct Cli {
    #[clap(short, long, env = "RPC_URL")]
    rpc_url: Option<String>,

    #[clap(short, long, env = "PAYER_KEYPAIR")]
    payer: Option<std::path::PathBuf>,

    #[clap(short, long, env = "ADMIN_KEYPAIR")]
    admin: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Crank {},
    Taker {},
}
fn main() -> Result<(), anyhow::Error> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    dotenv::dotenv().ok();

    let Cli {
        rpc_url,
        payer,
        admin,
        command,
    } = Cli::parse();

    let payer = match payer {
        Some(p) => keypair::read_keypair_file(&p)
            .unwrap_or_else(|_| panic!("Failed to read keypair from {}", p.to_string_lossy())),
        None => match env::var("PAYER_KEYPAIR").ok() {
            Some(k) => {
                keypair::read_keypair(&mut k.as_bytes()).expect("Failed to parse $PAYER_KEYPAIR")
            }
            None => panic!("Payer keypair not provided..."),
        },
    };

    let admin = match admin {
        Some(p) => keypair::read_keypair_file(&p)
            .unwrap_or_else(|_| panic!("Failed to read keypair from {}", p.to_string_lossy())),
        None => match env::var("ADMIN_KEYPAIR").ok() {
            Some(k) => {
                keypair::read_keypair(&mut k.as_bytes()).expect("Failed to parse $ADMIN_KEYPAIR")
            }
            None => panic!("Admin keypair not provided..."),
        },
    };

    let rpc_url = match rpc_url {
        Some(rpc_url) => rpc_url,
        None => match env::var("RPC_URL").ok() {
            Some(rpc_url) => rpc_url,
            None => panic!("Rpc URL not provided..."),
        },
    };
    let ws_url = rpc_url.replace("https", "wss");

    let cluster = Cluster::Custom(rpc_url, ws_url);
    let commitment = match command {
        Command::Crank { .. } => CommitmentConfig::confirmed(),
        Command::Taker { .. } => CommitmentConfig::confirmed(),
    };

    let mango_client = Arc::new(MangoClient::new(cluster, commitment, payer, admin));

    log::info!("Program Id {}", &mango_client.program().id());
    log::info!("Admin {}", &mango_client.admin.to_base58_string());

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let debugging_handle = async {
        let mut interval = time::interval(time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let client = mango_client.clone();
            tokio::task::spawn_blocking(move || {
                log::info!(
                    "Arc<MangoClient>::strong_count() {}",
                    Arc::<MangoClient>::strong_count(&client)
                )
            });
        }
    };

    match command {
        Command::Crank { .. } => {
            let client = mango_client.clone();
            let x: Result<(), anyhow::Error> = rt.block_on(crank::runner(client, debugging_handle));
            x.expect("Something went wrong here...");
        }
        Command::Taker { .. } => {
            let client = mango_client.clone();
            let x: Result<(), anyhow::Error> = rt.block_on(taker::runner(client, debugging_handle));
            x.expect("Something went wrong here...");
        }
    }

    Ok(())
}
