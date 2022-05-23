use std::{sync::Arc, time::Duration};

use mango_v4::state::PerpMarket;

use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use tokio::time;

use crate::MangoClient;

pub async fn loop_blocking(mango_client: Arc<MangoClient>, pk: Pubkey, perp_market: PerpMarket) {
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        let client = mango_client.clone();
        tokio::task::spawn_blocking(move || {
            perform_operation(client, pk, perp_market).expect("Something went wrong here...");
        })
        .await
        .expect("Something went wrong here...");
    }
}

pub fn perform_operation(
    mango_client: Arc<MangoClient>,
    pk: Pubkey,
    perp_market: PerpMarket,
) -> anyhow::Result<()> {
    let sig_result = mango_client
        .program()
        .request()
        .instruction(Instruction {
            program_id: mango_v4::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &mango_v4::accounts::PerpUpdateFunding {
                    perp_market: pk,
                    asks: perp_market.asks,
                    bids: perp_market.bids,
                    oracle: perp_market.oracle,
                },
                None,
            ),
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::PerpUpdateFunding {}),
        })
        .send();
    if let Err(e) = sig_result {
        log::error!("{:?}", e)
    }

    Ok(())
}
