use chrono::*;
use clap::{Args, Parser, Subcommand};
use itertools::Itertools;
use mango_v4_client::{
    delay_interval, keypair_from_cli, pubkey_from_cli, Client, MangoClient, TransactionBuilder,
    TransactionBuilderConfig,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::TransactionConfirmationStatus;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod save_snapshot;
mod test_oracles;

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
    slippage_bps: u64,

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
    /// Regularly fetches all oracles and prints their prices
    TestOracles {
        #[clap(short, long)]
        group: String,

        #[clap(flatten)]
        rpc: Rpc,
    },
    SaveSnapshot {
        #[clap(short, long)]
        group: String,

        #[clap(flatten)]
        rpc: Rpc,

        #[clap(short, long)]
        output: String,
    },
    TxSendingTest {
        #[clap(flatten)]
        rpc: Rpc,
    },
}

impl Rpc {
    fn client(&self, override_fee_payer: Option<&str>) -> anyhow::Result<Client> {
        let fee_payer = keypair_from_cli(override_fee_payer.unwrap_or(&self.fee_payer));
        Ok(Client::builder()
            .cluster(anchor_client::Cluster::from_str(&self.url)?)
            .commitment(solana_sdk::commitment_config::CommitmentConfig::confirmed())
            .fee_payer(Some(Arc::new(fee_payer)))
            .transaction_builder_config(TransactionBuilderConfig {
                prioritization_micro_lamports: Some(5),
                compute_budget_per_instruction: Some(250_000),
            })
            .build()
            .unwrap())
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    mango_v4_client::tracing_subscriber_init();

    dotenv::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Command::CreateAccount(cmd) => {
            let client = cmd.rpc.client(Some(&cmd.owner))?;
            let group = pubkey_from_cli(&cmd.group);
            let owner = Arc::new(keypair_from_cli(&cmd.owner));

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
            let (account, txsig) = MangoClient::create_account(
                &client,
                group,
                owner.clone(),
                owner.clone(),
                account_num,
                &cmd.name,
            )
            .await?;
            println!("{}", account);
            println!("{}", txsig);
        }
        Command::Deposit(cmd) => {
            let client = cmd.rpc.client(Some(&cmd.owner))?;
            let account = pubkey_from_cli(&cmd.account);
            let owner = Arc::new(keypair_from_cli(&cmd.owner));
            let mint = pubkey_from_cli(&cmd.mint);
            let client = MangoClient::new_for_existing_account(client, account, owner).await?;
            let txsig = client.token_deposit(mint, cmd.amount, false).await?;
            println!("{}", txsig);
        }
        Command::JupiterSwap(cmd) => {
            let client = cmd.rpc.client(Some(&cmd.owner))?;
            let account = pubkey_from_cli(&cmd.account);
            let owner = Arc::new(keypair_from_cli(&cmd.owner));
            let input_mint = pubkey_from_cli(&cmd.input_mint);
            let output_mint = pubkey_from_cli(&cmd.output_mint);
            let client = MangoClient::new_for_existing_account(client, account, owner).await?;
            let txsig = client
                .jupiter_v4()
                .swap(
                    input_mint,
                    output_mint,
                    cmd.amount,
                    cmd.slippage_bps,
                    mango_v4_client::JupiterSwapMode::ExactIn,
                    false,
                )
                .await?;
            println!("{}", txsig);
        }
        Command::GroupAddress { creator, num } => {
            let creator = pubkey_from_cli(&creator);
            println!("{}", MangoClient::group_for_admin(creator, num));
        }
        Command::MangoAccountAddress { group, owner, num } => {
            let group = pubkey_from_cli(&group);
            let owner = pubkey_from_cli(&owner);
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
        Command::TestOracles { group, rpc } => {
            let client = rpc.client(None)?;
            let group = pubkey_from_cli(&group);
            test_oracles::run(&client, group).await?;
        }
        Command::SaveSnapshot { group, rpc, output } => {
            let mango_group = pubkey_from_cli(&group);
            let client = rpc.client(None)?;
            save_snapshot::save_snapshot(mango_group, client, output).await?
        }
        Command::TxSendingTest { rpc } => {
            let client = rpc.client(None)?;
            tx_sending_test(client).await?;
        }
    };

    Ok(())
}

#[derive(Clone)]
struct InFlightTx {
    sent_at: DateTime<Utc>,
    sent_at_slot: u64,
    kind: String,
}

async fn tx_sending_test(client: Client) -> anyhow::Result<()> {
    let client = Arc::new(client);
    let in_flight_tx = Arc::new(RwLock::new(HashMap::default()));

    tokio::spawn(confirm_tx(client.clone(), in_flight_tx.clone()));

    let mut interval = delay_interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        let _ = send_tx_inner(&client, &in_flight_tx).await?;
    }
}

async fn confirm_tx(
    client: Arc<Client>,
    in_flight_tx: Arc<RwLock<HashMap<Signature, InFlightTx>>>,
) -> anyhow::Result<()> {
    let rpc = client.rpc_async();
    let mut interval = delay_interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        let _ = confirm_tx_inner(rpc, &in_flight_tx).await?;
    }
}

async fn confirm_tx_inner(
    rpc: &RpcClient,
    in_flight_tx: &RwLock<HashMap<Signature, InFlightTx>>,
) -> anyhow::Result<()> {
    let max_confirmation_seconds = 120;

    let in_flight = in_flight_tx.read().unwrap().clone();
    let signatures = in_flight.keys().copied().collect_vec();

    let statuses = rpc.get_signature_statuses(&signatures).await?;

    let mut results = vec![];

    let now = Utc::now();
    for (status, signature) in statuses.value.iter().zip(signatures.iter()) {
        let data = in_flight.get(signature).unwrap();

        if let Some(status) = status {
            if status.confirmation_status() == TransactionConfirmationStatus::Finalized {
                let bt = Utc
                    .timestamp_opt(
                        rpc.get_block_time(status.slot).await?.try_into().unwrap(),
                        0,
                    )
                    .unwrap();
                results.push((
                    *signature,
                    Some((bt.signed_duration_since(data.sent_at), status.slot)),
                ));
            }
            continue;
        }

        if now.signed_duration_since(data.sent_at).num_seconds() > max_confirmation_seconds {
            results.push((*signature, None));
        }
    }

    // log if confirmed, then
    for (signature, result) in results.iter() {
        let data = in_flight.get(signature).unwrap();
        let sent_at = data.sent_at;
        let sent_at_slot = data.sent_at_slot;
        let kind = &data.kind;

        let (conf_time, slot) = result
            .map(|(conf_time, slot)| (conf_time.num_milliseconds(), slot as i64))
            .unwrap_or((max_confirmation_seconds * 1000, -1));
        let slot_duration = if slot != -1 {
            slot - sent_at_slot as i64
        } else {
            -1
        };
        println!(
            "{sent_at},{sent_at_slot},{kind},{signature},true,{slot},{slot_duration},{conf_time}"
        );
    }

    // remove
    let mut lock = in_flight_tx.write().unwrap();
    for (signature, _) in results {
        lock.remove(&signature);
    }

    Ok(())
}

async fn send_tx_inner(
    client: &Client,
    in_flight_tx: &RwLock<HashMap<Signature, InFlightTx>>,
) -> anyhow::Result<()> {
    let rpc = client.rpc_async();
    let blockhash = rpc
        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
        .await?
        .0;

    let fee_payer = client.fee_payer().pubkey();

    // simple transfer only touching the fee payer
    let ix = solana_sdk::system_instruction::transfer(&fee_payer, &fee_payer, 0);
    let builder = TransactionBuilder {
        instructions: vec![ix],
        address_lookup_tables: vec![],
        signers: vec![client.fee_payer()],
        payer: fee_payer,
        config: TransactionBuilderConfig {
            prioritization_micro_lamports: None,
            compute_budget_per_instruction: None,
        },
    };
    let tx = builder.transaction_with_blockhash(blockhash)?;
    send_one_tx(client, tx, "transfer".into(), in_flight_tx).await?;

    // transfer that write locks a bunch of mango banks
    let strpk = |s| Pubkey::from_str(s).unwrap();

    let tx = make_tx_with_locks(
        client,
        blockhash,
        &[
            strpk("J6MsZiJUU6bjKSCkbfQsiHkd8gvJoddG2hsdSFsZQEZV"), // usdc bank
            strpk("FqEhSJSP3ao8RwRSekaAQ9sNQBSANhfb6EPtxQBByyh5"), // sol bank
            strpk("3k87hyqCaFR2G4SVwsLNMyPmR1mFN6uo7dUytzKQYu9d"), // usdt bank
        ],
        Some(600000),
    )?;
    send_one_tx(client, tx, "major-banks".into(), in_flight_tx).await?;

    let tx = make_tx_with_locks(
        client,
        blockhash,
        &[
            strpk("Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD"), // usdc oracle
            strpk("H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG"), // sol oracle
            strpk("3vxLXJqLqF3JG5TCbYycbKWRBbCJQLxQmBGCkyqEEefL"), // usdt oracle
        ],
        Some(600000),
    )?;
    send_one_tx(client, tx, "major-oracles".into(), in_flight_tx).await?;

    Ok(())
}

fn make_tx_with_locks(
    client: &Client,
    blockhash: solana_sdk::hash::Hash,
    locks: &[Pubkey],
    cu: Option<u32>,
) -> anyhow::Result<VersionedTransaction> {
    let fee_payer = client.fee_payer().pubkey();
    let mut instructions = locks
        .iter()
        .map(|acc| solana_sdk::system_instruction::transfer(&fee_payer, acc, 0))
        .collect_vec();
    if let Some(cu) = cu {
        instructions.insert(0, ComputeBudgetInstruction::set_compute_unit_limit(cu));
    }
    let builder = TransactionBuilder {
        instructions,
        address_lookup_tables: vec![],
        signers: vec![client.fee_payer()],
        payer: fee_payer,
        config: TransactionBuilderConfig {
            prioritization_micro_lamports: None,
            compute_budget_per_instruction: None,
        },
    };
    let tx = builder.transaction_with_blockhash(blockhash)?;
    Ok(tx)
}

async fn send_one_tx(
    client: &Client,
    tx: VersionedTransaction,
    kind: String,
    in_flight_tx: &RwLock<HashMap<Signature, InFlightTx>>,
) -> anyhow::Result<()> {
    let signature = client.send_transaction(&tx).await?;
    let now = Utc::now();

    let slot = client
        .rpc_async()
        .get_slot_with_commitment(CommitmentConfig::processed())
        .await?;

    let mut lock = in_flight_tx.write().unwrap();
    lock.insert(
        signature,
        InFlightTx {
            sent_at: now,
            sent_at_slot: slot,
            kind,
        },
    );

    Ok(())
}
