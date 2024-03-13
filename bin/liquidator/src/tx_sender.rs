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
    Tcs((Pubkey, u64, u64)),
}

pub fn spawn_tx_senders_job(
    max_parallel_operations: u64,
    tx_trigger_receiver: Receiver<()>,
    rebalance_trigger_sender: Sender<()>,
    shared_state: Arc<RwLock<SharedState>>,
    liquidation: Box<LiquidationState>,
    tcs: Box<TcsState>,
) -> Vec<JoinHandle<()>> {
    if max_parallel_operations < 1 {
        error!("max_parallel_operations must be >= 1");
        std::process::exit(1)
    }

    let workers: Vec<JoinHandle<()>> = (0..max_parallel_operations)
        .map(|worker_id| {
            tokio::spawn({
                let shared_state = shared_state.clone();
                let receiver = tx_trigger_receiver.clone();
                let rebalance_trigger_sender = rebalance_trigger_sender.clone();
                let liquidation = liquidation.clone();
                let tcs = tcs.clone();
                async move {
                    worker_loop(
                        shared_state,
                        receiver,
                        rebalance_trigger_sender,
                        liquidation,
                        tcs,
                        worker_id,
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
    receiver: Receiver<()>,
    rebalance_trigger_sender: Sender<()>,
    mut liquidation: Box<LiquidationState>,
    mut tcs: Box<TcsState>,
    id: u64,
) {
    loop {
        debug!("Worker #{} waiting for task", id);
        let _ = receiver.recv().await.unwrap();

        // a task must be available to process
        // find it in global shared state, and mark it as processing
        let task =
            worker_pull_task(&shared_state, id).expect("Worker woke up but has nothing to do");

        // execute the task
        let done = match task {
            WorkerTask::Liquidation(l) => worker_execute_liquidation(&mut liquidation, l).await,
            WorkerTask::Tcs(t) => worker_execute_tcs(&mut tcs, t).await,
        };

        if done {
            rebalance_trigger_sender.send_unless_full(()).unwrap();
        }

        // remove from shared state
        worker_finalize_task(&shared_state, id, task, done);
    }
}

async fn worker_execute_tcs(tcs: &mut Box<TcsState>, candidate: (Pubkey, u64, u64)) -> bool {
    tcs.maybe_take_token_conditional_swap(vec![candidate])
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
) -> anyhow::Result<WorkerTask> {
    let mut writer = shared_state.write().unwrap();

    // print out list of all task for debugging
    for x in &writer.liquidation_candidates_accounts {
        if !writer.processing_liquidation.contains(x) {
            trace!(" - LIQ {:?}", x);
        }
    }

    for x in &writer.interesting_tcs {
        if !writer.processing_tcs.contains(x) {
            trace!("  - TCS {:?}", x);
        }
    }

    // next task to execute
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

    if let Some(tcs_candidate) = writer
        .interesting_tcs
        .iter()
        .find(|x| !writer.processing_tcs.contains(x))
        .copied()
    {
        debug!(
            "worker #{} got a tcs candidate -> {:?} (out of {})",
            id,
            tcs_candidate,
            writer.interesting_tcs.len()
        );
        writer.processing_tcs.insert(tcs_candidate);
        return Ok(WorkerTask::Tcs(tcs_candidate));
    }

    anyhow::bail!("Worker #{} - No task found", id);
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
        WorkerTask::Tcs(tcs) => {
            debug!(
                "worker #{} - checked tcs {:?} with success ? {}",
                id, tcs, done
            );
            writer.interesting_tcs.shift_remove(&tcs);
            writer.processing_tcs.remove(&tcs);
        }
    }
}
