use std::{env, time::Duration};

use anchor_client::{Client, Cluster, Program};
use clap::{Parser, Subcommand};
use log::info;
use mango_v4::state::Bank;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_sdk::signature::Keypair;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signer::{keypair, Signer},
};
use tokio::time;

/// Wrapper around anchor client with some mango specific useful things
pub struct MangoClient {
    pub program: Program,
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
            program,
            rpc,
            cluster,
            commitment,
            admin,
            payer,
        }
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

// future: more subcommands e.g. Liquidator
#[derive(Subcommand)]
enum Command {
    Crank {},
}
fn main() {
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
        Command::Crank { .. } => CommitmentConfig::processed(),
    };

    let mango_client = MangoClient::new(cluster, commitment, payer, admin);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // future: match on various subcommands
    rt.block_on(update_index_runner(&mango_client))
        .expect("Something went wrong here...");
}

pub async fn update_index_runner(mango_client: &MangoClient) -> anyhow::Result<()> {
    // future: make configurable
    let mut interval = time::interval(Duration::from_millis(10));

    loop {
        interval.tick().await;
        update_index(mango_client)
            .await
            .expect("Something went wrong here...");
    }
}

pub async fn update_index(mango_client: &MangoClient) -> anyhow::Result<()> {
    // Collect all banks for a group belonging to an admin
    let banks = mango_client
        .program
        .accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp {
            offset: 24,
            bytes: MemcmpEncodedBytes::Base58({
                // find group belonging to admin
                Pubkey::find_program_address(
                    &["Group".as_ref(), mango_client.admin.pubkey().as_ref()],
                    &mango_client.program.id(),
                )
                .0
                .to_string()
            }),
            encoding: None,
        })])?;

    assert!(
        !banks.is_empty(),
        "Found 0 banks, some part of the configuration might be wrong..."
    );

    // Call update index ix
    for bank in banks {
        let sig = mango_client
            .program
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::UpdateIndex { bank: bank.0 },
                    None,
                ),
                data: anchor_lang::InstructionData::data(&mango_v4::instruction::UpdateIndex {}),
            })
            .send()?;

        info!("update_index ix signature: {:?}", sig);
    }

    Ok(())
}
