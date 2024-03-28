mod configuration;
mod processors;

use futures_util::StreamExt;
// use mango_feeds_connector::metrics;
use mango_v4_client::tracing_subscriber_init;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::Ordering;

use crate::configuration::Configuration;
use crate::processors::data::{DataEventSource, DataProcessor};
use crate::processors::exit::ExitProcessor;
use crate::processors::logger::LoggerProcessor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Please enter a config file path argument.");
        return Ok(());
    }

    let configuration: Configuration = {
        let mut file = File::open(&args[1])?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        toml::from_str(&contents).unwrap()
    };

    tracing_subscriber_init();

    let exit_processor = ExitProcessor::init().await?;

    let ws_processor: DataProcessor = DataProcessor::init(
        &configuration,
        DataEventSource::Websocket,
        exit_processor.exit.clone(),
    )
    .await?;
    let grpc_processor: DataProcessor = DataProcessor::init(
        &configuration,
        DataEventSource::Grpc,
        exit_processor.exit.clone(),
    )
    .await?;

    let logger_processor = LoggerProcessor::init(
        &ws_processor.channel,
        &grpc_processor.channel,
        exit_processor.exit.clone(),
    )
    .await?;

    let jobs = vec![
        exit_processor.job,
        ws_processor.job,
        grpc_processor.job,
        logger_processor.job,
    ];
    let mut jobs: futures::stream::FuturesUnordered<_> = jobs.into_iter().collect();

    while let Some(_) = jobs.next().await {
        // if any job exit, stop the others threads & wait
        exit_processor.exit.store(true, Ordering::Relaxed);
    }

    // for now, we force exit here because websocket connection to RPC is not properly closed on exit
    tracing::warn!("killing process");
    std::process::exit(0x0100);
}
