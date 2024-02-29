use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

pub struct ExitProcessor {
    pub job: JoinHandle<()>,
    pub exit: Arc<AtomicBool>,
}

impl ExitProcessor {
    pub async fn init() -> anyhow::Result<ExitProcessor> {
        let exit: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let exit_clone = exit.clone();

        let job = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(1000));
            loop {
                if exit_clone.load(Ordering::Relaxed) {
                    warn!("shutting down logger processor...");
                    break;
                }
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = tokio::signal::ctrl_c()=> {
                        info!("Received SIGINT, shutting down...");
                        exit_clone.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            }

            warn!("shutting down exit processor...");
        });

        let result = ExitProcessor { job, exit };
        Ok(result)
    }
}
