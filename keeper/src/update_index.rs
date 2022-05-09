use std::{sync::Arc, time::Duration};

use mango_v4::state::Bank;

use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use tokio::time;

use crate::MangoClient;

pub async fn loop_blocking(mango_client: Arc<MangoClient>, pk: Pubkey, bank: Bank) {
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        let client = mango_client.clone();
        tokio::task::spawn_blocking(move || {
            perform_operation(client, pk, bank).expect("Something went wrong here...");
        });
    }
}

pub fn perform_operation(
    mango_client: Arc<MangoClient>,
    pk: Pubkey,
    bank: Bank,
) -> anyhow::Result<()> {
    let sig_result = mango_client
        .program()
        .request()
        .instruction(Instruction {
            program_id: mango_v4::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &mango_v4::accounts::UpdateIndex { bank: pk },
                None,
            ),
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::UpdateIndex {}),
        })
        .send();
    match sig_result {
        Ok(sig) => {
            log::info!(
                "Crank: update_index for bank {:?} ix signature: {:?}",
                format!("{: >6}", bank.name()),
                sig
            );
        }
        Err(e) => log::error!("Crank: {:?}", e),
    }

    Ok(())
}
