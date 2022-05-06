use std::time::Duration;

use anchor_lang::{AccountDeserialize, __private::bytemuck::cast_ref};

use log::{error, info, warn};
use mango_v4::state::{EventQueue, EventType, FillEvent, OutEvent, PerpMarket};

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tokio::time;

use crate::MangoClient;

pub async fn loop_blocking(
    mango_client: &'static MangoClient,
    pk: Pubkey,
    perp_market: PerpMarket,
) {
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        tokio::task::spawn_blocking(move || {
            perform_operation(mango_client, pk, perp_market).expect("Something went wrong here...");
        });
    }
}

pub fn perform_operation(
    mango_client: &'static MangoClient,
    pk: Pubkey,
    perp_market: PerpMarket,
) -> anyhow::Result<()> {
    let mut event_queue = match get_event_queue(mango_client, &perp_market) {
        Ok(value) => value,
        Err(value) => return value,
    };

    let mut ams_ = vec![];

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
    match sig_result {
        Ok(sig) => {
            info!(
                "Crank: consume event for perp_market {:?} ix signature: {:?}",
                format!("{: >6}", perp_market.name()),
                sig
            );
        }
        Err(e) => error!("Crank: {:?}", e),
    }

    Ok(())
}

fn get_event_queue(
    mango_client: &MangoClient,
    perp_market: &PerpMarket,
) -> Result<mango_v4::state::EventQueue, Result<(), anyhow::Error>> {
    let event_queue_opt: Option<EventQueue> = {
        let res = mango_client
            .rpc
            .get_account_with_commitment(&perp_market.event_queue, mango_client.commitment);

        let res = match res {
            Ok(x) => x,
            Err(e) => {
                warn!("{}", e);
                return Err(Ok(()));
            }
        };

        let data = res.value.unwrap().data;
        let mut data_slice: &[u8] = &data;
        AccountDeserialize::try_deserialize(&mut data_slice).ok()
    };
    let mut event_queue = event_queue_opt.unwrap();
    Ok(event_queue)
}
