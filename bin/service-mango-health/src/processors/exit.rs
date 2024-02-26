use tracing::info;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct ExitProcessor {
    pub job: JoinHandle<()>,
    pub exit: Arc<AtomicBool>,
}

impl ExitProcessor {
    pub async fn init() -> anyhow::Result<ExitProcessor> {
        let exit: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let exit_clone = exit.clone();

        let job = tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            info!("Received SIGINT, shutting down...");
            exit_clone.store(true, Ordering::Relaxed);
        });

        let result = ExitProcessor { job, exit };
        Ok(result)
    }
}
