use std::{time::Duration};


use anyhow::ensure;

use log::{error, info};
use mango_v4::state::Bank;

use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};

use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signer::{Signer},
};
use tokio::time;

use crate::MangoClient;

pub async fn runner(mango_client: &'static MangoClient) -> Result<(), anyhow::Error> {
    // Collect all banks for a group belonging to an admin
    let banks = mango_client
        .program()
        .accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp {
            offset: 24,
            bytes: MemcmpEncodedBytes::Base58({
                // find group belonging to admin
                Pubkey::find_program_address(
                    &["Group".as_ref(), mango_client.admin.pubkey().as_ref()],
                    &mango_client.program().id(),
                )
                .0
                .to_string()
            }),
            encoding: None,
        })])?;

    ensure!(!banks.is_empty());

    let handles = banks
        .iter()
        .map(|(pk, bank)| loop_blocking(mango_client, *pk, *bank))
        .collect::<Vec<_>>();

    futures::join!(futures::future::join_all(handles));

    Ok(())
}

async fn loop_blocking(mango_client: &'static MangoClient, pk: Pubkey, bank: Bank) {
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        tokio::task::spawn_blocking(move || {
            perform_operation(mango_client, pk, bank).expect("Something went wrong here...");
        });
    }
}

pub fn perform_operation(
    mango_client: &'static MangoClient,
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
            info!(
                "Crank: update_index for bank {:?} ix signature: {:?}",
                format!("{: >6}", bank.name()),
                sig
            );
        }
        Err(e) => error!("Crank: {:?}", e),
    }

    Ok(())
}
