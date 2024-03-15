use crate::liquidation_state::LiquidationState;
use crate::tcs_state::TcsState;
use crate::SharedState;
use anchor_lang::prelude::Pubkey;
use async_channel::{Receiver, Sender};
use mango_v4_client::AsyncChannelSendUnlessFull;
use std::sync::{Arc, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, trace};

enum WorkerTask {
    Liquidation(Pubkey),
    Tcs(Vec<(Pubkey, u64, u64)>),
    GiveUpTcs,
}

pub fn spawn_tx_senders_job(
    max_parallel_operations: u64,
    enable_liquidation: bool,
    tx_liq_trigger_receiver: Receiver<()>,
    tx_tcs_trigger_receiver: Receiver<()>,
    tx_tcs_trigger_sender: Sender<()>,
    rebalance_trigger_sender: Sender<()>,
    shared_state: Arc<RwLock<SharedState>>,
    liquidation: Box<LiquidationState>,
    tcs: Box<TcsState>,
) -> Vec<JoinHandle<()>> {
    if max_parallel_operations < 1 {
        error!("max_parallel_operations must be >= 1");
        std::process::exit(1)
    }

    let reserve_one_worker_for_liquidation = max_parallel_operations > 1 && enable_liquidation;
    let tcs_capable_worker_count = (max_parallel_operations as usize)
        - if reserve_one_worker_for_liquidation {
            1
        } else {
            0
        };

    let workers: Vec<JoinHandle<()>> = (0..max_parallel_operations)
        .map(|worker_id| {
            tokio::spawn({
                let shared_state = shared_state.clone();
                let receiver_liq = tx_liq_trigger_receiver.clone();
                let receiver_tcs = tx_tcs_trigger_receiver.clone();
                let sender_tcs = tx_tcs_trigger_sender.clone();
                let rebalance_trigger_sender = rebalance_trigger_sender.clone();
                let liquidation = liquidation.clone();
                let tcs = tcs.clone();
                async move {
                    worker_loop(
                        shared_state,
                        receiver_liq,
                        receiver_tcs,
                        sender_tcs,
                        rebalance_trigger_sender,
                        liquidation,
                        tcs,
                        worker_id,
                        tcs_capable_worker_count,
                        reserve_one_worker_for_liquidation && worker_id == 0,
                    )
                    .await;
                }
            })
        })
        .collect();

    workers
}

async fn worker_loop(
    shared_state: Arc<RwLock<SharedState>>,
    liq_receiver: Receiver<()>,
    tcs_receiver: Receiver<()>,
    tcs_sender: Sender<()>,
    rebalance_trigger_sender: Sender<()>,
    mut liquidation: Box<LiquidationState>,
    mut tcs: Box<TcsState>,
    id: u64,
    tcs_capable_workers: usize,
    only_liquidation: bool,
) {
    loop {
        debug!("Worker #{} waiting for task", id);

        let _ = if only_liquidation {
            liq_receiver.recv().await.expect("receive failed")
        } else {
            tokio::select!(
                _ = liq_receiver.recv() => {},
                _ = tcs_receiver.recv() => {},
            )
        };

        // a task must be available to process
        // find it in global shared state, and mark it as processing
        let task = worker_pull_task(&shared_state, id, tcs_capable_workers, only_liquidation)
            .expect("Worker woke up but has nothing to do");

        // execute the task
        let need_rebalancing = match &task {
            WorkerTask::Liquidation(l) => worker_execute_liquidation(&mut liquidation, *l).await,
            WorkerTask::Tcs(t) => worker_execute_tcs(&mut tcs, t.clone()).await,
            WorkerTask::GiveUpTcs => worker_give_up_tcs(&tcs_sender).await,
        };

        if need_rebalancing {
            rebalance_trigger_sender.send_unless_full(()).unwrap();
        }

        // remove from shared state
        worker_finalize_task(&shared_state, id, task, need_rebalancing);
    }
}

async fn worker_give_up_tcs(sender: &Sender<()>) -> bool {
    sender.send(()).await.expect("sending task failed");
    false
}

async fn worker_execute_tcs(tcs: &mut Box<TcsState>, candidates: Vec<(Pubkey, u64, u64)>) -> bool {
    tcs.maybe_take_token_conditional_swap(candidates)
        .await
        .unwrap_or(false)
}

async fn worker_execute_liquidation(
    liquidation: &mut Box<LiquidationState>,
    candidate: Pubkey,
) -> bool {
    liquidation
        .maybe_liquidate_and_log_error(&candidate)
        .await
        .unwrap_or(false)
}

fn worker_pull_task(
    shared_state: &Arc<RwLock<SharedState>>,
    id: u64,
    tcs_capable_workers: usize,
    only_liquidation: bool,
) -> anyhow::Result<WorkerTask> {
    let mut writer = shared_state.write().unwrap();

    // print out list of all task for debugging
    for x in &writer.liquidation_candidates_accounts {
        if !writer.processing_liquidation.contains(x) {
            trace!(" - LIQ {:?}", x);
        }
    }

    // next liq task to execute
    if let Some(liq_candidate) = writer
        .liquidation_candidates_accounts
        .iter()
        .find(|x| !writer.processing_liquidation.contains(x))
        .copied()
    {
        debug!("worker #{} got a liq candidate -> {}", id, liq_candidate);
        writer.processing_liquidation.insert(liq_candidate);
        return Ok(WorkerTask::Liquidation(liq_candidate));
    }

    if only_liquidation {
        return Ok(WorkerTask::GiveUpTcs);
    }

    for x in &writer.interesting_tcs {
        if !writer.processing_tcs.contains(x) {
            trace!("  - TCS {:?}", x);
        }
    }

    // next tcs task to execute
    let tcs_todo = writer.interesting_tcs.len() - writer.processing_tcs.len();
    let max_tcs_batch_size = tcs_todo / tcs_capable_workers;
    let tcs_candidates: Vec<(Pubkey, u64, u64)> = writer
        .interesting_tcs
        .iter()
        .filter(|x| !writer.processing_tcs.contains(x))
        .take(max_tcs_batch_size)
        .copied()
        .collect();

    for tcs_candidate in &tcs_candidates {
        debug!(
            "worker #{} got a tcs candidate -> {:?} (out of {})",
            id,
            tcs_candidate,
            writer.interesting_tcs.len()
        );
        writer.processing_tcs.insert(tcs_candidate.clone());
    }

    Ok(WorkerTask::Tcs(tcs_candidates))
}

fn worker_finalize_task(
    shared_state: &Arc<RwLock<SharedState>>,
    id: u64,
    task: WorkerTask,
    done: bool,
) {
    let mut writer = shared_state.write().unwrap();
    match task {
        WorkerTask::Liquidation(liq) => {
            debug!(
                "worker #{} - checked liq {:?} with success ? {}",
                id, liq, done
            );
            writer.liquidation_candidates_accounts.shift_remove(&liq);
            writer.processing_liquidation.remove(&liq);
        }
        WorkerTask::Tcs(tcs_list) => {
            for tcs in tcs_list {
                debug!(
                    "worker #{} - checked tcs {:?} with success ? {}",
                    id, tcs, done
                );
                writer.interesting_tcs.shift_remove(&tcs);
                writer.processing_tcs.remove(&tcs);
            }
        }
        WorkerTask::GiveUpTcs => {}
    }
}
