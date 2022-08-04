use std::{sync::Arc, time::Duration};

use crate::MangoClient;

use anchor_lang::{__private::bytemuck::cast_ref, solana_program};
use client::prettify_client_error;
use futures::Future;
use mango_v4::state::{EventQueue, EventType, FillEvent, OutEvent, PerpMarket, TokenIndex};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tokio::time;

// TODO: move instructions into the client proper

pub async fn runner(
    mango_client: Arc<MangoClient>,
    debugging_handle: impl Future,
) -> Result<(), anyhow::Error> {
    let handles1 = mango_client
        .context
        .tokens
        .keys()
        .map(|&token_index| loop_update_index_and_rate(mango_client.clone(), token_index))
        .collect::<Vec<_>>();

    let handles2 = mango_client
        .context
        .perp_markets
        .values()
        .map(|perp| loop_consume_events(mango_client.clone(), perp.address, perp.market))
        .collect::<Vec<_>>();

    let handles3 = mango_client
        .context
        .perp_markets
        .values()
        .map(|perp| loop_update_funding(mango_client.clone(), perp.address, perp.market))
        .collect::<Vec<_>>();

    futures::join!(
        futures::future::join_all(handles1),
        futures::future::join_all(handles2),
        futures::future::join_all(handles3),
        debugging_handle
    );

    Ok(())
}

pub async fn loop_update_index_and_rate(mango_client: Arc<MangoClient>, token_index: TokenIndex) {
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        let client = mango_client.clone();

        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let token = client.context.token(token_index);
            let banks_for_a_token = token.mint_info.banks();
            let token_name = &token.name;
            let oracle = token.mint_info.oracle;

            let sig_result = client
                .program()
                .request()
                .instruction({
                    let mut ix = Instruction {
                        program_id: mango_v4::id(),
                        accounts: anchor_lang::ToAccountMetas::to_account_metas(
                            &mango_v4::accounts::TokenUpdateIndexAndRate {
                                group: token.mint_info.group,
                                mint_info: token.mint_info_address,
                                oracle,
                                instructions: solana_program::sysvar::instructions::id(),
                            },
                            None,
                        ),
                        data: anchor_lang::InstructionData::data(
                            &mango_v4::instruction::TokenUpdateIndexAndRate {},
                        ),
                    };
                    let mut banks = banks_for_a_token
                        .iter()
                        .map(|bank_pubkey| AccountMeta {
                            pubkey: *bank_pubkey,
                            is_signer: false,
                            is_writable: true,
                        })
                        .collect::<Vec<_>>();
                    ix.accounts.append(&mut banks);
                    ix
                })
                .send()
                .map_err(prettify_client_error);

            if let Err(e) = sig_result {
                log::error!("{:?}", e)
            } else {
                log::info!(
                    "update_index_and_rate {} {:?}",
                    token_name,
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
                .send()
                .map_err(prettify_client_error);

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
                            group: perp_market.group,
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
                .send()
                .map_err(prettify_client_error);
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
