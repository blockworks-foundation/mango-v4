use std::{sync::Arc, time::Duration};

use anchor_lang::__private::bytemuck::cast_ref;

use mango_v4::state::{EventQueue, EventType, FillEvent, OutEvent, PerpMarket};

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
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
    let mut event_queue: EventQueue = mango_client
        .program()
        .account(perp_market.event_queue)
        .unwrap();

    let mut ams_ = vec![];

    // TODO: future, choose better constant of how many max events to pack
    // TODO: future, choose better constant of how many max mango accounts to pack
    for _ in 0..10 {
        let event = match event_queue.peek_front() {
            None => break,
            Some(e) => e,
        };
        match EventType::try_from(event.event_type)? {
            EventType::Fill => {
                let fill: &FillEvent = cast_ref(event);
                ams_.push(AccountMeta {
                    pubkey: fill.maker,
                    is_signer: false,
                    is_writable: true,
                });
                ams_.push(AccountMeta {
                    pubkey: fill.taker,
                    is_signer: false,
                    is_writable: true,
                });
            }
            EventType::Out => {
                let out: &OutEvent = cast_ref(event);
                ams_.push(AccountMeta {
                    pubkey: out.owner,
                    is_signer: false,
                    is_writable: true,
                });
            }
            EventType::Liquidate => {}
        }
        event_queue.pop_front()?;
    }

    let sig_result = mango_client
        .program()
        .request()
        .instruction(Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::PerpConsumeEvents {
                        group: perp_market.group,
                        perp_market: pk,
                        event_queue: perp_market.event_queue,
                    },
                    None,
                );
                ams.append(&mut ams_);
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::PerpConsumeEvents {
                limit: 10,
            }),
        })
        .send();

    if let Err(e) = sig_result {
        log::error!("{:?}", e)
    }

    Ok(())
}
