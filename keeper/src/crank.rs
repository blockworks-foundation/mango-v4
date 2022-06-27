use std::{sync::Arc, time::Duration};

use crate::MangoClient;

use anchor_lang::__private::bytemuck::cast_ref;
use futures::Future;
use mango_v4::state::{EventQueue, EventType, FillEvent, OutEvent, PerpMarket, TokenIndex};
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
        .map(|banks_for_a_token| {
            loop_update_index(
                mango_client.clone(),
                banks_for_a_token.get(0).unwrap().1.token_index,
            )
        })
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

pub async fn loop_update_index(mango_client: Arc<MangoClient>, token_index: TokenIndex) {
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        let client = mango_client.clone();

        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mint_info = client.get_mint_info(&token_index);
            let banks_for_a_token = client.banks_cache_by_token_index.get(&token_index).unwrap();
            let token_name = banks_for_a_token.get(0).unwrap().1.name();

            let bank_pubkeys_for_a_token = banks_for_a_token
                .into_iter()
                .map(|bank| bank.0)
                .collect::<Vec<Pubkey>>();

            let sig_result = client
                .program()
                .request()
                .instruction({
                    let mut ix = Instruction {
                        program_id: mango_v4::id(),
                        accounts: anchor_lang::ToAccountMetas::to_account_metas(
                            &mango_v4::accounts::UpdateIndex { mint_info },
                            None,
                        ),
                        data: anchor_lang::InstructionData::data(
                            &mango_v4::instruction::UpdateIndex {},
                        ),
                    };
                    let mut foo = bank_pubkeys_for_a_token
                        .iter()
                        .map(|bank_pubkey| AccountMeta {
                            pubkey: *bank_pubkey,
                            is_signer: false,
                            is_writable: false,
                        })
                        .collect::<Vec<_>>();
                    ix.accounts.append(&mut foo);
                    ix
                })
                .send();

            if let Err(e) = sig_result {
                log::error!("{:?}", e)
            } else {
                log::info!("update_index {} {:?}", token_name, sig_result.unwrap())
            }

            Ok(())
        })
        .await;

        match res {
            Ok(inner_res) => {
                if inner_res.is_err() {
                    log::error!("{}", inner_res.unwrap_err());
                }
            }
            Err(join_error) => {
                log::error!("{}", join_error);
            }
        }
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
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
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
        })
        .await;

        match res {
            Ok(inner_res) => {
                if inner_res.is_err() {
                    log::error!("{}", inner_res.unwrap_err());
                }
            }
            Err(join_error) => {
                log::error!("{}", join_error);
            }
        }
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
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
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
        })
        .await;

        match res {
            Ok(inner_res) => {
                if inner_res.is_err() {
                    log::error!("{}", inner_res.unwrap_err());
                }
            }
            Err(join_error) => {
                log::error!("{}", join_error);
            }
        }
    }
}
