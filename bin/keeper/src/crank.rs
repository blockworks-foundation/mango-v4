use std::{collections::HashSet, sync::Arc, time::Duration, time::Instant};

use crate::MangoClient;
use itertools::Itertools;

use anchor_lang::{__private::bytemuck::cast_ref, solana_program};
use futures::Future;
use mango_v4::state::{EventQueue, EventType, FillEvent, OutEvent, TokenIndex};
use mango_v4_client::PerpMarketContext;
use prometheus::{register_histogram, Encoder, Histogram, IntCounter, Registry};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tracing::*;
use warp::Filter;

lazy_static::lazy_static! {
    pub static ref METRICS_REGISTRY: Registry = Registry::new_custom(Some("keeper".to_string()), None).unwrap();
    pub static ref METRIC_UPDATE_TOKENS_SUCCESS: IntCounter =
        IntCounter::new("update_tokens_success", "Successful update token transactions").unwrap();
    pub static ref METRIC_UPDATE_TOKENS_FAILURE: IntCounter =
        IntCounter::new("update_tokens_failure", "Failed update token transactions").unwrap();
    pub static ref METRIC_CONSUME_EVENTS_SUCCESS: IntCounter =
        IntCounter::new("consume_events_success", "Successful consume events transactions").unwrap();
    pub static ref METRIC_CONSUME_EVENTS_FAILURE: IntCounter =
        IntCounter::new("consume_events_failure", "Failed consume events transactions").unwrap();
    pub static ref METRIC_UPDATE_FUNDING_SUCCESS: IntCounter =
        IntCounter::new("update_funding_success", "Successful update funding transactions").unwrap();
    pub static ref METRIC_UPDATE_FUNDING_FAILURE: IntCounter =
        IntCounter::new("update_funding_failure", "Failed update funding transactions").unwrap();
    pub static ref METRIC_CONFIRMATION_TIMES: Histogram = register_histogram!(
        "confirmation_times", "Transaction confirmation times",
        vec![1000.0, 3000.0, 5000.0, 7000.0, 10000.0, 15000.0, 20000.0, 30000.0, 40000.0, 50000.0, 60000.0]
    ).unwrap();
}

// TODO: move instructions into the client proper

async fn serve_metrics() {
    METRICS_REGISTRY
        .register(Box::new(METRIC_UPDATE_TOKENS_SUCCESS.clone()))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(METRIC_UPDATE_TOKENS_FAILURE.clone()))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(METRIC_CONSUME_EVENTS_SUCCESS.clone()))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(METRIC_CONSUME_EVENTS_FAILURE.clone()))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(METRIC_UPDATE_FUNDING_SUCCESS.clone()))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(METRIC_UPDATE_FUNDING_FAILURE.clone()))
        .unwrap();
    METRICS_REGISTRY
        .register(Box::new(METRIC_CONFIRMATION_TIMES.clone()))
        .unwrap();

    let metrics_route = warp::path!("metrics").map(|| {
        let mut buffer = Vec::<u8>::new();
        let encoder = prometheus::TextEncoder::new();
        encoder
            .encode(&METRICS_REGISTRY.gather(), &mut buffer)
            .unwrap();

        String::from_utf8(buffer.clone()).unwrap()
    });
    println!("Metrics server starting on port 9091");
    warp::serve(metrics_route).run(([0, 0, 0, 0], 9091)).await;
}

pub async fn runner(
    mango_client: Arc<MangoClient>,
    debugging_handle: impl Future,
    interval_update_banks: u64,
    interval_consume_events: u64,
    interval_update_funding: u64,
    interval_check_for_changes_and_abort: u64,
) -> Result<(), anyhow::Error> {
    let handles1 = mango_client
        .context
        .tokens
        .keys()
        // TODO: grouping tokens whose oracle might have less confidencen e.g. ORCA with the rest, fails whole ix
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
        .filter(|perp|
            // MNGO-PERP-OLD
            perp.perp_market_index != 1)
        .map(|perp| {
            loop_consume_events(
                mango_client.clone(),
                perp.address,
                perp,
                interval_consume_events,
            )
        })
        .collect::<Vec<_>>();

    let handles3 = mango_client
        .context
        .perp_markets
        .values()
        .filter(|perp|
            // MNGO-PERP-OLD
            perp.perp_market_index != 1)
        .map(|perp| {
            loop_update_funding(
                mango_client.clone(),
                perp.address,
                perp,
                interval_update_funding,
            )
        })
        .collect::<Vec<_>>();

    futures::join!(
        futures::future::join_all(handles1),
        futures::future::join_all(handles2),
        futures::future::join_all(handles3),
        MangoClient::loop_check_for_context_changes_and_abort(
            mango_client.clone(),
            Duration::from_secs(interval_check_for_changes_and_abort),
        ),
        serve_metrics(),
        debugging_handle,
    );

    Ok(())
}

pub async fn loop_update_index_and_rate(
    mango_client: Arc<MangoClient>,
    token_indices: Vec<TokenIndex>,
    interval: u64,
) {
    let mut interval = mango_v4_client::delay_interval(Duration::from_secs(interval));
    loop {
        interval.tick().await;

        let client = mango_client.clone();

        let token_indices_clone = token_indices.clone();

        let token_names = token_indices_clone
            .iter()
            .map(|token_index| client.context.token(*token_index).name.to_owned())
            .join(",");

        let mut instructions = vec![];
        for token_index in token_indices_clone.iter() {
            let token = client.context.token(*token_index);
            let banks_for_a_token = token.banks();
            let oracle = token.oracle;

            let mut ix = Instruction {
                program_id: mango_v4::id(),
                accounts: anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::TokenUpdateIndexAndRate {
                        group: token.group,
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

            let sim_result = match client.simulate(vec![ix.clone()]).await {
                Ok(response) => response.value,
                Err(e) => {
                    error!(token.name, "simulation request error: {e:?}");
                    continue;
                }
            };

            if let Some(e) = sim_result.err {
                error!(token.name, "simulation error: {e:?} {:?}", sim_result.logs);
                continue;
            }

            instructions.push(ix);
        }
        let pre = Instant::now();
        let sig_result = client
            .send_and_confirm_permissionless_tx(instructions)
            .await;

        let confirmation_time = pre.elapsed().as_millis();
        METRIC_CONFIRMATION_TIMES.observe(confirmation_time as f64);

        if let Err(e) = sig_result {
            METRIC_UPDATE_TOKENS_FAILURE.inc();
            info!(
                "metricName=UpdateTokensV4Failure tokens={} durationMs={} error={}",
                token_names, confirmation_time, e
            );
            error!("{:?}", e)
        } else {
            METRIC_UPDATE_TOKENS_SUCCESS.inc();
            info!(
                "metricName=UpdateTokensV4Success tokens={} durationMs={}",
                token_names, confirmation_time,
            );
            info!("{:?}", sig_result);
        }
    }
}

pub async fn loop_consume_events(
    mango_client: Arc<MangoClient>,
    pk: Pubkey,
    perp_market: &PerpMarketContext,
    interval: u64,
) {
    let mut interval = mango_v4_client::delay_interval(Duration::from_secs(interval));
    loop {
        interval.tick().await;

        let client = mango_client.clone();

        let find_accounts = || async {
            let mut num_of_events = 0;
            let mut event_queue: EventQueue = client
                .client
                .rpc_anchor_account(&perp_market.event_queue)
                .await?;

            // TODO: future, choose better constant of how many max events to pack
            // TODO: future, choose better constant of how many max mango accounts to pack
            let mut set = HashSet::new();
            for _ in 0..10 {
                let event = match event_queue.peek_front() {
                    None => break,
                    Some(e) => e,
                };
                match EventType::try_from(event.event_type)? {
                    EventType::Fill => {
                        let fill: &FillEvent = cast_ref(event);
                        set.insert(fill.maker);
                        set.insert(fill.taker);
                    }
                    EventType::Out => {
                        let out: &OutEvent = cast_ref(event);
                        set.insert(out.owner);
                    }
                    EventType::Liquidate => {}
                }
                event_queue.pop_front()?;
                num_of_events += 1;
            }

            if num_of_events == 0 {
                return Ok(None);
            }

            Ok(Some((set, num_of_events)))
        };

        let event_info: anyhow::Result<Option<(HashSet<Pubkey>, u32)>> = find_accounts().await;

        let (event_accounts, num_of_events) = match event_info {
            Ok(Some(x)) => x,
            Ok(None) => continue,
            Err(err) => {
                error!("preparing consume_events ams: {err:?}");
                continue;
            }
        };

        let mut event_ams = event_accounts
            .iter()
            .map(|key| -> AccountMeta {
                AccountMeta {
                    pubkey: *key,
                    is_signer: false,
                    is_writable: true,
                }
            })
            .collect::<Vec<_>>();

        let pre = Instant::now();
        let ix = Instruction {
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
                ams.append(&mut event_ams);
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::PerpConsumeEvents {
                limit: 10,
            }),
        };

        let sig_result = client.send_and_confirm_permissionless_tx(vec![ix]).await;

        let confirmation_time = pre.elapsed().as_millis();
        METRIC_CONFIRMATION_TIMES.observe(confirmation_time as f64);

        if let Err(e) = sig_result {
            METRIC_CONSUME_EVENTS_FAILURE.inc();
            info!(
                "metricName=ConsumeEventsV4Failure market={} durationMs={} consumed={} error={}",
                perp_market.name,
                confirmation_time,
                num_of_events,
                e.to_string()
            );
            error!("{:?}", e)
        } else {
            METRIC_CONSUME_EVENTS_SUCCESS.inc();
            info!(
                "metricName=ConsumeEventsV4Success market={} durationMs={} consumed={}",
                perp_market.name, confirmation_time, num_of_events,
            );
            info!("{:?}", sig_result);
        }
    }
}

pub async fn loop_update_funding(
    mango_client: Arc<MangoClient>,
    pk: Pubkey,
    perp_market: &PerpMarketContext,
    interval: u64,
) {
    let mut interval = mango_v4_client::delay_interval(Duration::from_secs(interval));
    loop {
        interval.tick().await;

        let client = mango_client.clone();

        let pre = Instant::now();
        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &mango_v4::accounts::PerpUpdateFunding {
                    group: perp_market.group,
                    perp_market: pk,
                    bids: perp_market.bids,
                    asks: perp_market.asks,
                    oracle: perp_market.oracle,
                },
                None,
            ),
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::PerpUpdateFunding {}),
        };
        let sig_result = client.send_and_confirm_permissionless_tx(vec![ix]).await;

        let confirmation_time = pre.elapsed().as_millis();
        METRIC_CONFIRMATION_TIMES.observe(confirmation_time as f64);

        if let Err(e) = sig_result {
            METRIC_UPDATE_FUNDING_FAILURE.inc();
            error!(
                "metricName=UpdateFundingV4Error market={} durationMs={} error={}",
                perp_market.name,
                confirmation_time,
                e.to_string()
            );
            error!("{:?}", e)
        } else {
            METRIC_UPDATE_FUNDING_SUCCESS.inc();
            info!(
                "metricName=UpdateFundingV4Success market={} durationMs={}",
                perp_market.name, confirmation_time,
            );
            info!("{:?}", sig_result);
        }
    }
}
