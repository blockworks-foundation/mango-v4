mod crank;
mod taker;

use std::sync::Arc;
use std::time::Duration;

use anchor_client::Cluster;

use clap::{Parser, Subcommand};
use mango_v4_client::{
    keypair_from_cli, priority_fees_cli, Client, FallbackOracleConfig, MangoClient,
    TransactionBuilderConfig,
};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use tokio::time;
use tracing::*;

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

    #[clap(long, env, default_value_t = 60)]
    // TODO: use duration type from rust instead of u64 for all these below intervals
    interval_update_banks: u64,

    #[clap(long, env, default_value_t = 5)]
    interval_consume_events: u64,

    #[clap(long, env, default_value_t = 5)]
    interval_update_funding: u64,

    #[clap(long, env, default_value_t = 120)]
    interval_check_new_listings_and_abort: u64,

    #[clap(long, env, default_value_t = 300)]
    interval_charge_collateral_fees: u64,

    #[clap(long, env, default_value_t = 10)]
    timeout: u64,

    #[clap(flatten)]
    prioritization_fee_cli: priority_fees_cli::PriorityFeeArgs,

    /// url to the lite-rpc websocket, optional
    #[clap(long, env, default_value = "")]
    lite_rpc_url: String,

    /// When batching multiple instructions into a transaction, don't exceed
    /// this compute unit limit.
    #[clap(long, env, default_value_t = 1_000_000)]
    max_cu_when_batching: u32,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    Crank {},
    Taker {},
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    mango_v4_client::tracing_subscriber_init();

    let args = if let Ok(cli_dotenv) = CliDotenv::try_parse() {
        dotenv::from_path(cli_dotenv.dotenv)?;
        cli_dotenv.remaining_args
    } else {
        dotenv::dotenv().ok();
        std::env::args_os().collect()
    };
    let cli = Cli::parse_from(args);

    let (prio_provider, prio_jobs) = cli
        .prioritization_fee_cli
        .make_prio_provider(cli.lite_rpc_url.clone())?;

    let owner = Arc::new(keypair_from_cli(&cli.owner));

    let rpc_url = cli.rpc_url;
    let ws_url = rpc_url.replace("https", "wss");

    let cluster = Cluster::Custom(rpc_url, ws_url);
    let commitment = match cli.command {
        Command::Crank { .. } => CommitmentConfig::confirmed(),
        Command::Taker { .. } => CommitmentConfig::confirmed(),
    };

    let mango_client = Arc::new(
        MangoClient::new_for_existing_account(
            Client::builder()
                .cluster(cluster)
                .commitment(commitment)
                .fee_payer(Some(owner.clone()))
                .timeout(Duration::from_secs(cli.timeout))
                .transaction_builder_config(
                    TransactionBuilderConfig::builder()
                        .priority_fee_provider(prio_provider)
                        .compute_budget_per_instruction(None)
                        .build()
                        .unwrap(),
                )
                .fallback_oracle_config(FallbackOracleConfig::Never)
                .build()
                .unwrap(),
            cli.mango_account,
            owner,
        )
        .await?,
    );

    let debugging_handle = async {
        let mut interval = mango_v4_client::delay_interval(time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let client = mango_client.clone();
            tokio::task::spawn_blocking(move || {
                info!(
                    "Arc<MangoClient>::strong_count() {}",
                    Arc::<MangoClient>::strong_count(&client)
                )
            });
        }
    };

    match cli.command {
        Command::Crank { .. } => {
            let client = mango_client.clone();
            crank::runner(
                client,
                debugging_handle,
                cli.interval_update_banks,
                cli.interval_consume_events,
                cli.interval_update_funding,
                cli.interval_check_new_listings_and_abort,
                cli.interval_charge_collateral_fees,
                cli.max_cu_when_batching,
                prio_jobs,
            )
            .await
        }
        Command::Taker { .. } => {
            let client = mango_client.clone();
            taker::runner(client, debugging_handle, prio_jobs).await
        }
    }
}
