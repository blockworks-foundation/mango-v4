use clap::{Parser, Subcommand};
use client::MangoClient;
use solana_sdk::pubkey::Pubkey;

#[derive(Parser, Debug, Clone)]
#[clap()]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
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
        num: u8,
    },
}
fn main() -> Result<(), anyhow::Error> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    dotenv::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
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
