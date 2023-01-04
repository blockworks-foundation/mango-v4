use clap::{Args, Parser, Subcommand};
use client::MangoClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Parser, Debug, Clone)]
#[clap()]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Args, Debug, Clone)]
struct Rpc {
    #[clap(short, long, default_value = "m")]
    url: String,

    #[clap(short, long, default_value = "")]
    fee_payer: String,
}

#[derive(Args, Debug, Clone)]
struct CreateAccount {
    #[clap(short, long)]
    group: String,

    /// also pays for everything
    #[clap(short, long)]
    owner: String,

    #[clap(short, long)]
    account_num: Option<u32>,

    #[clap(short, long, default_value = "")]
    name: String,

    #[clap(flatten)]
    rpc: Rpc,
}

#[derive(Args, Debug, Clone)]
struct Deposit {
    #[clap(long)]
    account: String,

    /// also pays for everything
    #[clap(short, long)]
    owner: String,

    #[clap(short, long)]
    mint: String,

    #[clap(short, long)]
    amount: u64,

    #[clap(flatten)]
    rpc: Rpc,
}

#[derive(Args, Debug, Clone)]
struct JupiterSwap {
    #[clap(long)]
    account: String,

    /// also pays for everything
    #[clap(short, long)]
    owner: String,

    #[clap(long)]
    input_mint: String,

    #[clap(long)]
    output_mint: String,

    #[clap(short, long)]
    amount: u64,

    #[clap(short, long)]
    slippage: f64,

    #[clap(flatten)]
    rpc: Rpc,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    CreateAccount(CreateAccount),
    Deposit(Deposit),
    JupiterSwap(JupiterSwap),
    GroupAddress {
        #[clap(short, long)]
        creator: String,

        #[clap(short, long, default_value = "0")]
        num: u32,
    },
    MangoAccountAddress {
        #[clap(short, long)]
        group: String,

        #[clap(short, long)]
        owner: String,

        #[clap(short, long, default_value = "0")]
        num: u32,
    },
}

impl Rpc {
    fn client(&self, override_fee_payer: Option<&str>) -> anyhow::Result<client::Client> {
        let fee_payer = client::keypair_from_cli(override_fee_payer.unwrap_or(&self.fee_payer));
        Ok(client::Client {
            cluster: anchor_client::Cluster::from_str(&self.url)?,
            commitment: solana_sdk::commitment_config::CommitmentConfig::confirmed(),
            fee_payer: Arc::new(fee_payer),
            timeout: None,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    dotenv::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Command::CreateAccount(cmd) => {
            let client = cmd.rpc.client(Some(&cmd.owner))?;
            let group = client::pubkey_from_cli(&cmd.group);
            let owner = client::keypair_from_cli(&cmd.owner);

            let account_num = if let Some(num) = cmd.account_num {
                num
            } else {
                // find free account_num
                let accounts = MangoClient::find_accounts(&client, group, &owner).await?;
                if accounts.is_empty() {
                    0
                } else {
                    accounts
                        .iter()
                        .map(|(_, account)| account.fixed.account_num)
                        .max()
                        .unwrap()
                        + 1
                }
            };
            let (account, txsig) =
                MangoClient::create_account(&client, group, &owner, &owner, account_num, &cmd.name)
                    .await?;
            println!("{}", account);
            println!("{}", txsig);
        }
        Command::Deposit(cmd) => {
            let client = cmd.rpc.client(Some(&cmd.owner))?;
            let account = client::pubkey_from_cli(&cmd.account);
            let owner = client::keypair_from_cli(&cmd.owner);
            let mint = client::pubkey_from_cli(&cmd.mint);
            let client = MangoClient::new_for_existing_account(client, account, owner).await?;
            let txsig = client.token_deposit(mint, cmd.amount, false).await?;
            println!("{}", txsig);
        }
        Command::JupiterSwap(cmd) => {
            let client = cmd.rpc.client(Some(&cmd.owner))?;
            let account = client::pubkey_from_cli(&cmd.account);
            let owner = client::keypair_from_cli(&cmd.owner);
            let input_mint = client::pubkey_from_cli(&cmd.input_mint);
            let output_mint = client::pubkey_from_cli(&cmd.output_mint);
            let client = MangoClient::new_for_existing_account(client, account, owner).await?;
            let txsig = client
                .jupiter_swap(
                    input_mint,
                    output_mint,
                    cmd.amount,
                    cmd.slippage,
                    client::JupiterSwapMode::ExactIn,
                )
                .await?;
            println!("{}", txsig);
        }
        Command::GroupAddress { creator, num } => {
            let creator = client::pubkey_from_cli(&creator);
            println!("{}", MangoClient::group_for_admin(creator, num));
        }
        Command::MangoAccountAddress { group, owner, num } => {
            let group = client::pubkey_from_cli(&group);
            let owner = client::pubkey_from_cli(&owner);
            let address = Pubkey::find_program_address(
                &[
                    group.as_ref(),
                    b"MangoAccount".as_ref(),
                    owner.as_ref(),
                    &num.to_le_bytes(),
                ],
                &mango_v4::ID,
            )
            .0;
            println!("{}", address);
        }
    };

    Ok(())
}
