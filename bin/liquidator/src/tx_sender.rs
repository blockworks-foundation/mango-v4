use crate::liquidation_state::LiquidationState;
use crate::tcs_state::TcsState;
use crate::{SharedState, TxTrigger};
use mango_v4_client::AsyncChannelSendUnlessFull;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tracing::debug;

pub fn spawn_tx_senders_job(
    max_parallel_operations: u64,
    mut tx_trigger_receiver: Receiver<TxTrigger>,
    rebalance_trigger_sender: Sender<()>,
    shared_state: Arc<RwLock<SharedState>>,
    liquidation: Box<LiquidationState>,
    tcs: Box<TcsState>,
) -> Vec<JoinHandle<()>> {
    if max_parallel_operations < 1 {
        tracing::error!("max_parallel_operations must be >= 1");
        std::process::exit(1)
    }

    let semaphore = Arc::new(Semaphore::new(0));
    let mut workers: Vec<JoinHandle<()>> = (0..max_parallel_operations)
        .map(|i| {
            tokio::spawn({
                let semaphore = semaphore.clone();
                let shared_state = shared_state.clone();
                let rebalance_trigger_sender = rebalance_trigger_sender.clone();
                let mut liquidation = liquidation.clone();
                let mut tcs = tcs.clone();
                let id = i;
                async move {
                    loop {
                        debug!("Worker #{} waiting for task", id);
                        let permit = semaphore.acquire().await.unwrap();

                        // a task is supposed to be available to process
                        // find it in global shared state, and mark it as processing
                        // (it's also possible we don't find anything in case liq/tcs job enqueued the same task multiple time)
                        let (l, t) = {
                            let mut writer = shared_state.write().unwrap();

                            // print out list of all task for debugging
                            for x in &writer.liquidation_candidates_accounts {
                                if !writer.processing_liquidation.contains(x) {
                                    tracing::trace!(" - LIQ {:?}", x);
                                }
                            }
                            for x in &writer.interesting_tcs {
                                if !writer.processing_tcs.contains(x) {
                                    tracing::trace!("  - TCS {:?}", x);
                                }
                            }

                            // next task to execute
                            let liq_candidate = writer
                                .liquidation_candidates_accounts
                                .iter()
                                .find(|x| !writer.processing_liquidation.contains(x))
                                .map(|x| *x);
                            let tcs_candidate = writer
                                .interesting_tcs
                                .iter()
                                .find(|x| !writer.processing_tcs.contains(x))
                                .map(|x| *x);

                            if let Some(l) = liq_candidate {
                                debug!("worker #{} got a liq candidate -> {}", id, l);
                                writer.processing_liquidation.insert(l);
                                (Some(l), None)
                            } else if let Some(t) = tcs_candidate {
                                debug!(
                                    "worker #{} got a tcs candidate -> {:?} (out of {})",
                                    id,
                                    t,
                                    writer.interesting_tcs.len()
                                );
                                writer.processing_tcs.insert(t);
                                (None, Some(t))
                            } else {
                                debug!("worker #{} got nothing", id);
                                (None, None)
                            }
                        };

                        // execute the task
                        let done = if let Some(l) = l {
                            liquidation
                                .maybe_liquidate_and_log_error(&l)
                                .await
                                .unwrap_or(false)
                        } else if let Some(t) = t {
                            tcs.maybe_take_token_conditional_swap(vec![t])
                                .await
                                .unwrap_or(false)
                        } else {
                            false
                        };

                        if done {
                            rebalance_trigger_sender.send_unless_full(()).unwrap();
                        }

                        // remove from shared state
                        {
                            let mut writer = shared_state.write().unwrap();
                            if let Some(l) = l {
                                debug!(
                                    "worker #{} - checked liq {:?} with success ? {}",
                                    id, l, done
                                );
                                writer.liquidation_candidates_accounts.shift_remove(&l);
                                writer.processing_liquidation.remove(&l);
                            }
                            if let Some(t) = t {
                                debug!(
                                    "worker #{} - checked tcs {:?} with success ? {}",
                                    id, t, done
                                );
                                writer.interesting_tcs.shift_remove(&t);
                                writer.processing_tcs.remove(&t);
                            }
                        }

                        // worker is available for next task
                        permit.forget();
                    }
                }
            })
        })
        .collect();

    workers.push(tokio::spawn({
        async move {
            loop {
                let trigger_opt = tx_trigger_receiver.recv().await;
                if let Some(trigger) = trigger_opt {
                    let n = match trigger {
                        TxTrigger::Liquidation() => 1,
                        TxTrigger::TokenConditionalSwap(n) => n,
                    };

                    debug!("Queuing {} new task(s) for workers..", n);
                    semaphore.add_permits(n);
                }
            }
        }
    }));

    workers
}
