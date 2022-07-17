mod crank;
mod taker;

use std::env;
use std::str::FromStr;
use std::sync::Arc;

use anchor_client::Cluster;

use clap::{Parser, Subcommand};
use client::MangoClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Signer,
    signer::{keypair, keypair::Keypair},
};
use tokio::time;

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

    #[clap(short, long, env = "GROUP")]
    group: Option<Pubkey>,

    // These exist only as a shorthand to make testing easier. Normal users would provide the group.
    #[clap(long, env = "GROUP_FROM_ADMIN_KEYPAIR")]
    group_from_admin_keypair: Option<std::path::PathBuf>,
    #[clap(long, env = "GROUP_FROM_ADMIN_NUM", default_value = "0")]
    group_from_admin_num: u32,

    #[clap(short, long, env = "MANGO_ACCOUNT_NAME")]
    mango_account_name: String,

    #[clap(subcommand)]
    command: Command,
}

fn keypair_from_path(p: &std::path::PathBuf) -> Keypair {
    let path = std::path::PathBuf::from_str(&*shellexpand::tilde(p.to_str().unwrap())).unwrap();
    keypair::read_keypair_file(path)
        .unwrap_or_else(|_| panic!("Failed to read keypair from {}", p.to_string_lossy()))
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

    let cli = Cli::parse();

    let payer = match cli.payer {
        Some(p) => keypair_from_path(&p),
        None => panic!("Payer keypair not provided..."),
    };

    let rpc_url = match cli.rpc_url {
        Some(rpc_url) => rpc_url,
        None => match env::var("RPC_URL").ok() {
            Some(rpc_url) => rpc_url,
            None => panic!("Rpc URL not provided..."),
        },
    };
    let ws_url = rpc_url.replace("https", "wss");

    let cluster = Cluster::Custom(rpc_url, ws_url);
    let commitment = match cli.command {
        Command::Crank { .. } => CommitmentConfig::confirmed(),
        Command::Taker { .. } => CommitmentConfig::confirmed(),
    };

    let group = if let Some(group) = cli.group {
        group
    } else if let Some(p) = cli.group_from_admin_keypair {
        let admin = keypair_from_path(&p);
        MangoClient::group_for_admin(admin.pubkey(), cli.group_from_admin_num)
    } else {
        panic!("Must provide either group or group_from_admin_keypair");
    };

    let mango_client = Arc::new(MangoClient::new(
        cluster,
        commitment,
        group,
        payer,
        &cli.mango_account_name,
    )?);

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

    match cli.command {
        Command::Crank { .. } => {
            let client = mango_client.clone();
            rt.block_on(crank::runner(client, debugging_handle))
        }
        Command::Taker { .. } => {
            let client = mango_client.clone();
            rt.block_on(taker::runner(client, debugging_handle))
        }
    }
}
