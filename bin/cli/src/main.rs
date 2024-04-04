use clap::clap_derive::ArgEnum;
use clap::{Args, Parser, Subcommand};
use mango_v4::accounts_ix::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};
use mango_v4::state::{PlaceOrderType, SelfTradeBehavior, Side};
use mango_v4_client::{
    keypair_from_cli, pubkey_from_cli, Client, MangoClient, TransactionBuilderConfig,
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(ArgEnum, Clone, Debug)]
#[repr(u8)]
pub enum CliSide {
    Bid = 0,
    Ask = 1,
}

#[derive(Args, Debug, Clone)]
struct PerpPlaceOrder {
    #[clap(long)]
    account: String,

    /// also pays for everything
    #[clap(short, long)]
    owner: String,

    #[clap(long)]
    market_name: String,

    #[clap(long, value_enum)]
    side: CliSide,

    #[clap(short, long)]
    price: f64,

    #[clap(long)]
    quantity: f64,

    #[clap(long)]
    expiry: u64,

    #[clap(flatten)]
    rpc: Rpc,
}

#[derive(Args, Debug, Clone)]
struct Serum3CreateOpenOrders {
    #[clap(long)]
    account: String,

    /// also pays for everything
    #[clap(short, long)]
    owner: String,

    #[clap(long)]
    market_name: String,

    #[clap(flatten)]
    rpc: Rpc,
}

#[derive(Args, Debug, Clone)]
struct Serum3CloseOpenOrders {
    #[clap(long)]
    account: String,

    /// also pays for everything
    #[clap(short, long)]
    owner: String,

    #[clap(long)]
    market_name: String,

    #[clap(flatten)]
    rpc: Rpc,
}

#[derive(Args, Debug, Clone)]
struct Serum3PlaceOrder {
    #[clap(long)]
    account: String,

    /// also pays for everything
    #[clap(short, long)]
    owner: String,

    #[clap(long)]
    market_name: String,

    #[clap(long, value_enum)]
    side: CliSide,

    #[clap(short, long)]
    price: f64,

    #[clap(long)]
    quantity: f64,

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
    PerpPlaceOrder(PerpPlaceOrder),
    Serum3CloseOpenOrders(Serum3CloseOpenOrders),
    Serum3CreateOpenOrders(Serum3CreateOpenOrders),
    Serum3PlaceOrder(Serum3PlaceOrder),
}

impl Rpc {
    fn client(&self, override_fee_payer: Option<&str>) -> anyhow::Result<Client> {
        let fee_payer = keypair_from_cli(override_fee_payer.unwrap_or(&self.fee_payer));
        Ok(Client::builder()
            .cluster(anchor_client::Cluster::from_str(&self.url)?)
            .commitment(solana_sdk::commitment_config::CommitmentConfig::confirmed())
            .fee_payer(Some(Arc::new(fee_payer)))
            .transaction_builder_config(
                TransactionBuilderConfig::builder()
                    .prioritization_micro_lamports(Some(5))
                    .compute_budget_per_instruction(Some(250_000))
                    .build()
                    .unwrap(),
            )
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
                .jupiter_v6()
                .swap(input_mint, output_mint, cmd.amount, cmd.slippage_bps, false)
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
        Command::PerpPlaceOrder(cmd) => {
            let client = cmd.rpc.client(Some(&cmd.owner))?;
            let account = pubkey_from_cli(&cmd.account);
            let owner = Arc::new(keypair_from_cli(&cmd.owner));
            let client = MangoClient::new_for_existing_account(client, account, owner).await?;
            let market = client
                .context
                .perp_markets
                .iter()
                .find(|p| p.1.name == cmd.market_name)
                .unwrap()
                .1;

            fn native(x: f64, b: u32) -> i64 {
                (x * (10_i64.pow(b)) as f64) as i64
            }

            let price_lots = native(cmd.price, 6) * market.base_lot_size
                / (market.quote_lot_size * 10_i64.pow(market.base_decimals.into()));
            let max_base_lots =
                native(cmd.quantity, market.base_decimals.into()) / market.base_lot_size;

            let txsig = client
                .perp_place_order(
                    market.perp_market_index,
                    match cmd.side {
                        CliSide::Bid => Side::Bid,
                        CliSide::Ask => Side::Ask,
                    },
                    price_lots,
                    max_base_lots,
                    i64::max_value(),
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                    PlaceOrderType::Limit,
                    false,
                    if cmd.expiry > 0 {
                        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + cmd.expiry
                    } else {
                        0
                    },
                    10,
                    SelfTradeBehavior::AbortTransaction,
                )
                .await?;
            println!("{}", txsig);
        }
        Command::Serum3CreateOpenOrders(cmd) => {
            let client = cmd.rpc.client(Some(&cmd.owner))?;
            let account = pubkey_from_cli(&cmd.account);
            let owner = Arc::new(keypair_from_cli(&cmd.owner));
            let client = MangoClient::new_for_existing_account(client, account, owner).await?;

            let txsig = client.serum3_create_open_orders(&cmd.market_name).await?;
            println!("{}", txsig);
        }
        Command::Serum3CloseOpenOrders(cmd) => {
            let client = cmd.rpc.client(Some(&cmd.owner))?;
            let account = pubkey_from_cli(&cmd.account);
            let owner = Arc::new(keypair_from_cli(&cmd.owner));
            let client = MangoClient::new_for_existing_account(client, account, owner).await?;

            let txsig = client.serum3_close_open_orders(&cmd.market_name).await?;
            println!("{}", txsig);
        }
        Command::Serum3PlaceOrder(cmd) => {
            let client = cmd.rpc.client(Some(&cmd.owner))?;
            let account = pubkey_from_cli(&cmd.account);
            let owner = Arc::new(keypair_from_cli(&cmd.owner));
            let client = MangoClient::new_for_existing_account(client, account, owner).await?;
            let market_index = client.context.serum3_market_index(&cmd.market_name);
            let market = client.context.serum3(market_index);
            let base_token = client.context.token(market.base_token_index);
            let quote_token = client.context.token(market.quote_token_index);

            fn native(x: f64, b: u32) -> u64 {
                (x * (10_i64.pow(b)) as f64) as u64
            }

            // coin_lot_size = base lot size ?
            // cf priceNumberToLots
            let price_lots = native(cmd.price, quote_token.decimals as u32) * market.coin_lot_size
                / (native(1.0, base_token.decimals as u32) * market.pc_lot_size);

            // cf baseSizeNumberToLots
            let max_base_lots =
                native(cmd.quantity, base_token.decimals as u32) / market.coin_lot_size;

            let txsig = client
                .serum3_place_order(
                    &cmd.market_name,
                    match cmd.side {
                        CliSide::Bid => Serum3Side::Bid,
                        CliSide::Ask => Serum3Side::Ask,
                    },
                    price_lots,
                    max_base_lots as u64,
                    ((price_lots * max_base_lots) as f64 * 1.01) as u64,
                    Serum3SelfTradeBehavior::AbortTransaction,
                    Serum3OrderType::Limit,
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                    10,
                )
                .await?;
            println!("{}", txsig);
        }
    };

    Ok(())
}
