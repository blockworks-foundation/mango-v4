mod consume_events;
mod crank;
mod update_index;

use std::env;
use std::sync::Arc;

use anchor_client::{Client, Cluster, Program};

use clap::{Parser, Subcommand};

use solana_client::rpc_client::RpcClient;

use solana_sdk::signature::Keypair;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signer::{keypair, Signer},
};
use tokio::time;

// TODO
// - may be nice to have one-shot cranking as well as the interval cranking
// - doing a gPA for all banks call every 10millis may be too often,
// might make sense that we maintain a service when users should query group for changes
// - I'm really annoyed about Keypair not being clonable. Seems everyone works around that manually. Should make a PR to solana to newtype it and provide that function.
// keypair_from_arg_or_env could be a function

/// Wrapper around anchor client with some mango specific useful things
pub struct MangoClient {
    pub rpc: RpcClient,
    pub cluster: Cluster,
    pub commitment: CommitmentConfig,
    pub payer: Keypair,
    pub admin: Keypair,
}

impl MangoClient {
    pub fn new(
        cluster: Cluster,
        commitment: CommitmentConfig,
        payer: Keypair,
        admin: Keypair,
    ) -> Self {
        let program = Client::new_with_options(
            cluster.clone(),
            std::rc::Rc::new(Keypair::from_bytes(&payer.to_bytes()).unwrap()),
            commitment,
        )
        .program(mango_v4::ID);

        let rpc = program.rpc();
        Self {
            rpc,
            cluster,
            commitment,
            admin,
            payer,
        }
    }

    pub fn client(&self) -> Client {
        Client::new_with_options(
            self.cluster.clone(),
            std::rc::Rc::new(Keypair::from_bytes(&self.payer.to_bytes()).unwrap()),
            self.commitment,
        )
    }

    pub fn program(&self) -> Program {
        self.client().program(mango_v4::ID)
    }

    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }

    pub fn admin(&self) -> Pubkey {
        self.payer.pubkey()
    }
}

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
    Liquidator {},
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
        Command::Liquidator {} => todo!(),
    };

    let mango_client = Arc::new(MangoClient::new(cluster, commitment, payer, admin));

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
                    "std::sync::Arc<MangoClient>::strong_count() {}",
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
        Command::Liquidator { .. } => {
            todo!()
        }
    }

    Ok(())
}
