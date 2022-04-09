use std::{rc::Rc, str::FromStr, time::Duration};

use solana_sdk::{instruction::Instruction, signature::Keypair};
use tokio::time;

// TODO:
// cmd line args
// expand to various tasks e.g. crank event queue, crank banks, run liquidators
// support multiple workers
// logging facility
// robust error handling
fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(update_index_runner())
        .expect("Something went wrong here...");
}

pub async fn update_index_runner() -> anyhow::Result<()> {
    let mut interval = time::interval(Duration::from_millis(10));

    loop {
        interval.tick().await;
        update_index().await?;
    }
}

pub async fn update_index() -> anyhow::Result<()> {
    let keypair = load_default_keypair()?;
    let rpc = "https://mango.devnet.rpcpool.com".to_owned();
    let wss = rpc.replace("https", "wss");
    let connection =
        anchor_client::Client::new(anchor_client::Cluster::Custom(rpc, wss), Rc::new(keypair));
    let client = connection.program(mango_v4::ID);

    let update_index_ix = Instruction {
        program_id: mango_v4::id(),
        accounts: anchor_lang::ToAccountMetas::to_account_metas(
            &mango_v4::accounts::UpdateIndex {
                bank: anchor_lang::prelude::Pubkey::from_str(
                    "9xmZdkWbYNYsBshr7PwjhU8c7mmrvzmocu8dSQeNCKTG",
                )?,
            },
            None,
        ),
        data: anchor_lang::InstructionData::data(&mango_v4::instruction::UpdateIndex {}),
    };

    let sig = client.request().instruction(update_index_ix).send()?;
    println!("update_index: {:?}", sig);

    Ok(())
}

fn load_default_keypair() -> anyhow::Result<Keypair> {
    let keypair_path = shellexpand::tilde("~/.config/solana/mango-devnet.json");
    let keypair_data = std::fs::read_to_string(keypair_path.to_string())?;
    let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)?;
    let keypair = Keypair::from_bytes(&keypair_bytes)?;

    Ok(keypair)
}
