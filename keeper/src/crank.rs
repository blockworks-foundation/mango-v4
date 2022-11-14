use std::{sync::Arc, time::Duration, time::Instant};

use crate::MangoClient;
use itertools::Itertools;

use anchor_lang::{__private::bytemuck::cast_ref, solana_program};
use client::prettify_client_error;
use futures::Future;
use mango_v4::state::{EventQueue, EventType, FillEvent, OutEvent, PerpMarket, TokenIndex};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tokio::time;

// TODO: move instructions into the client proper

pub async fn runner(
    mango_client: Arc<MangoClient>,
    debugging_handle: impl Future,
    interval_update_banks: u64,
    interval_consume_events: u64,
    interval_update_funding: u64,
) -> Result<(), anyhow::Error> {
    let handles1 = mango_client
        .context
        .tokens
        .keys()
        // TokenUpdateIndexAndRate is known to take max 71k cu
        // from cargo test-bpf local tests
        // chunk size of 8 seems to be max before encountering "VersionedTransaction too large" issues
        .chunks(8)
        .into_iter()
        .map(|chunk| {
            loop_update_index_and_rate(
                mango_client.clone(),
                chunk.copied().collect::<Vec<TokenIndex>>(),
                interval_update_banks,
            )
        })
        .collect::<Vec<_>>();

    let handles2 = mango_client
        .context
        .perp_markets
        .values()
        .map(|perp| {
            loop_consume_events(
                mango_client.clone(),
                perp.address,
                perp.market,
                interval_consume_events,
            )
        })
        .collect::<Vec<_>>();

    let handles3 = mango_client
        .context
        .perp_markets
        .values()
        .map(|perp| {
            loop_update_funding(
                mango_client.clone(),
                perp.address,
                perp.market,
                interval_update_funding,
            )
        })
        .collect::<Vec<_>>();

    futures::join!(
        futures::future::join_all(handles1),
        futures::future::join_all(handles2),
        futures::future::join_all(handles3),
        debugging_handle
    );

    Ok(())
}

pub async fn loop_update_index_and_rate(
    mango_client: Arc<MangoClient>,
    token_indices: Vec<TokenIndex>,
    interval: u64,
) {
    let mut interval = time::interval(Duration::from_secs(interval));
    loop {
        interval.tick().await;

        let client = mango_client.clone();

        let token_indices_clone = token_indices.clone();

        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let token_names = token_indices_clone
                .iter()
                .map(|token_index| client.context.token(*token_index).name.to_owned())
                .join(",");

            let program = client.program();
            let mut req = program.request();
            req = req.instruction(ComputeBudgetInstruction::set_compute_unit_price(1));
            for token_index in token_indices_clone.iter() {
                let token = client.context.token(*token_index);
                let banks_for_a_token = token.mint_info.banks();
                let oracle = token.mint_info.oracle;

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
                req = req.instruction(ix);
            }
            let pre = Instant::now();
            let sig_result = req.send().map_err(prettify_client_error);

            if let Err(e) = sig_result {
                log::info!(
                    "metricName=UpdateTokensV4Failure tokens={} durationMs={} error={}",
                    token_names,
                    pre.elapsed().as_millis(),
                    e
                );
                log::error!("{:?}", e)
            } else {
                log::info!(
                    "metricName=UpdateTokensV4Success tokens={} durationMs={}",
                    token_names,
                    pre.elapsed().as_millis(),
                );
                log::info!("{:?}", sig_result);
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
    interval: u64,
) {
    let mut interval = time::interval(Duration::from_secs(interval));
    loop {
        interval.tick().await;

        let client = mango_client.clone();
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut event_queue: EventQueue =
                client.program().account(perp_market.event_queue).unwrap();

            let mut ams_ = vec![];
            let mut num_of_events = 0;

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
                num_of_events+=1;
            }

            if num_of_events == 0 {
                return Ok(());
            }

            let pre = Instant::now();
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
                log::info!(
                    "metricName=ConsumeEventsV4Failure market={} durationMs={} consumed={} error={}",
                    perp_market.name(),
                    pre.elapsed().as_millis(),
                    num_of_events,
                    e.to_string()
                );
                log::error!("{:?}", e)
            } else {
                log::info!(
                    "metricName=ConsumeEventsV4Success market={} durationMs={} consumed={}",
                    perp_market.name(),
                    pre.elapsed().as_millis(),
                    num_of_events,
                );
                log::info!("{:?}", sig_result);
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
    interval: u64,
) {
    let mut interval = time::interval(Duration::from_secs(interval));
    loop {
        interval.tick().await;

        let client = mango_client.clone();
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let pre = Instant::now();
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
                log::error!(
                    "metricName=UpdateFundingV4Error market={} durationMs={} error={}",
                    perp_market.name(),
                    pre.elapsed().as_millis(),
                    e.to_string()
                );
                log::error!("{:?}", e)
            } else {
                log::info!(
                    "metricName=UpdateFundingV4Success market={} durationMs={}",
                    perp_market.name(),
                    pre.elapsed().as_millis(),
                );
                log::info!("{:?}", sig_result);
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
