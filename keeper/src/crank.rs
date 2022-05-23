use std::{sync::Arc, time::Duration};

use crate::MangoClient;

use anchor_lang::__private::bytemuck::cast_ref;
use futures::Future;
use mango_v4::state::{Bank, EventQueue, EventType, FillEvent, OutEvent, PerpMarket};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tokio::time;

pub async fn runner(
    mango_client: Arc<MangoClient>,
    debugging_handle: impl Future,
) -> Result<(), anyhow::Error> {
    let handles1 = mango_client
        .banks_cache
        .values()
        .map(|(pk, bank)| loop_update_index(mango_client.clone(), *pk, *bank))
        .collect::<Vec<_>>();

    let handles2 = mango_client
        .perp_markets_cache
        .values()
        .map(|(pk, perp_market)| loop_consume_events(mango_client.clone(), *pk, *perp_market))
        .collect::<Vec<_>>();

    let handles3 = mango_client
        .perp_markets_cache
        .values()
        .map(|(pk, perp_market)| loop_update_funding(mango_client.clone(), *pk, *perp_market))
        .collect::<Vec<_>>();

    futures::join!(
        futures::future::join_all(handles1),
        futures::future::join_all(handles2),
        futures::future::join_all(handles3),
        debugging_handle
    );

    Ok(())
}

pub async fn loop_update_index(mango_client: Arc<MangoClient>, pk: Pubkey, bank: Bank) {
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        let client = mango_client.clone();
        tokio::task::spawn_blocking(move || {
            || -> anyhow::Result<()> {
                let sig_result = client
                    .program()
                    .request()
                    .instruction(Instruction {
                        program_id: mango_v4::id(),
                        accounts: anchor_lang::ToAccountMetas::to_account_metas(
                            &mango_v4::accounts::UpdateIndex { bank: pk },
                            None,
                        ),
                        data: anchor_lang::InstructionData::data(
                            &mango_v4::instruction::UpdateIndex {},
                        ),
                    })
                    .send();
                if let Err(e) = sig_result {
                    log::error!("{:?}", e)
                } else {
                    log::info!("update_index {} {:?}", bank.name(), sig_result.unwrap())
                }

                Ok(())
            }()
            .expect("Something went wrong here...")
        })
        .await
        .expect("Something went wrong here...");
    }
}

pub async fn loop_consume_events(
    mango_client: Arc<MangoClient>,
    pk: Pubkey,
    perp_market: PerpMarket,
) {
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        let client = mango_client.clone();
        tokio::task::spawn_blocking(move || {
            || -> anyhow::Result<()> {
                let mut event_queue: EventQueue =
                    client.program().account(perp_market.event_queue).unwrap();

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

                let sig_result = client
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
                        data: anchor_lang::InstructionData::data(
                            &mango_v4::instruction::PerpConsumeEvents { limit: 10 },
                        ),
                    })
                    .send();

                if let Err(e) = sig_result {
                    log::error!("{:?}", e)
                } else {
                    log::info!(
                        "consume_event {} {:?}",
                        perp_market.name(),
                        sig_result.unwrap()
                    )
                }

                Ok(())
            }()
            .expect("Something went wrong here...");
        })
        .await
        .expect("Something went wrong here...");
    }
}

pub async fn loop_update_funding(
    mango_client: Arc<MangoClient>,
    pk: Pubkey,
    perp_market: PerpMarket,
) {
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        let client = mango_client.clone();
        tokio::task::spawn_blocking(move || {
            || -> anyhow::Result<()> {
                let sig_result = client
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
                        data: anchor_lang::InstructionData::data(
                            &mango_v4::instruction::PerpUpdateFunding {},
                        ),
                    })
                    .send();
                if let Err(e) = sig_result {
                    log::error!("{:?}", e)
                } else {
                    log::info!(
                        "update_funding {} {:?}",
                        perp_market.name(),
                        sig_result.unwrap()
                    )
                }

                Ok(())
            }()
            .expect("Something went wrong here...");
        })
        .await
        .expect("Something went wrong here...");
    }
}
