mod configuration;
mod processors;
mod utils;

use futures_util::StreamExt;
// use mango_feeds_connector::metrics;
use mango_v4_client::tracing_subscriber_init;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::Ordering;

use crate::configuration::Configuration;
use crate::processors::data::DataProcessor;
use crate::processors::exit::ExitProcessor;
use crate::processors::health::HealthProcessor;
use crate::processors::logger::LoggerProcessor;
use crate::processors::persister::PersisterProcessor;

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

    // TODO FAS Add metrics
    // let metrics_tx = metrics::start(configuration.metrics.clone(), "health".into());

    let exit_processor = ExitProcessor::init().await?;

    let data_processor: DataProcessor =
        DataProcessor::init(&configuration, exit_processor.exit.clone()).await?;

    let health_processor = HealthProcessor::init(
        &data_processor.channel,
        data_processor.chain_data.clone(),
        &configuration,
        exit_processor.exit.clone(),
    )
    .await?;

    let logger = LoggerProcessor::init(
        &health_processor.channel,
        &configuration,
        exit_processor.exit.clone(),
    )
    .await?;

    let persister = PersisterProcessor::init(
        &health_processor.channel,
        &configuration,
        exit_processor.exit.clone(),
    )
    .await?;

    let mut jobs: futures::stream::FuturesUnordered<_> = vec![
        exit_processor.job,
        data_processor.job,
        health_processor.job,
        logger.job,
        persister.job,
    ]
    .into_iter()
    .collect();

    while let Some(_) = jobs.next().await {
        exit_processor.exit.store(true, Ordering::Relaxed);
    }

    Ok(())
}
