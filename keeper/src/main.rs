mod crank;
mod taker;

use std::sync::Arc;
use std::time::Duration;

use anchor_client::Cluster;

use clap::{Parser, Subcommand};
use client::{keypair_from_cli, Client, MangoClient};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use tokio::time;

// TODO
// - may be nice to have one-shot cranking as well as the interval cranking
// - doing a gPA for all banks call every 10millis may be too often,
// might make sense that we maintain a service when users should query group for changes
// - I'm really annoyed about Keypair not being clonable. Seems everyone works around that manually. Should make a PR to solana to newtype it and provide that function.
// keypair_from_arg_or_env could be a function

#[derive(Parser, Debug)]
#[clap()]
struct CliDotenv {
    // When --dotenv <file> is passed, read the specified dotenv file before parsing args
    #[clap(long)]
    dotenv: std::path::PathBuf,

    remaining_args: Vec<std::ffi::OsString>,
}

#[derive(Parser, Debug, Clone)]
#[clap()]
struct Cli {
    #[clap(short, long, env)]
    rpc_url: String,

    #[clap(short, long, env)]
    mango_account: Pubkey,

    #[clap(short, long, env)]
    owner: String,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    Crank {},
    Taker {},
}
fn main() -> Result<(), anyhow::Error> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = if let Ok(cli_dotenv) = CliDotenv::try_parse() {
        dotenv::from_path(cli_dotenv.dotenv)?;
        cli_dotenv.remaining_args
    } else {
        dotenv::dotenv().ok();
        std::env::args_os().collect()
    };
    let cli = Cli::parse_from(args);

    let owner = keypair_from_cli(&cli.owner);

    let rpc_url = cli.rpc_url;
    let ws_url = rpc_url.replace("https", "wss");

    let cluster = Cluster::Custom(rpc_url, ws_url);
    let commitment = match cli.command {
        Command::Crank { .. } => CommitmentConfig::confirmed(),
        Command::Taker { .. } => CommitmentConfig::confirmed(),
    };

    let mango_client = Arc::new(MangoClient::new_for_existing_account(
        Client::new(cluster, commitment, &owner, Some(Duration::from_secs(1))),
        cli.mango_account,
        owner,
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
