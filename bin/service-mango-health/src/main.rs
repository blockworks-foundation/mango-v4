mod configuration;
mod processors;

use futures_util::StreamExt;
use log::info;
use mango_feeds_connector::metrics;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::configuration::Configuration;
use crate::processors::data::DataProcessor;
use crate::processors::exit::ExitProcessor;
use crate::processors::health::HealthProcessor;
use crate::processors::persister::PersisterProcessor;
use crate::processors::publisher::PublisherProcessor;

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

    solana_logger::setup_with_default("info");

    // TODO FAS Add metrics
    let metrics_tx = metrics::start(configuration.metrics.clone(), "health".into());

    let exit_processor = ExitProcessor::init().await?;

    let data_processor: DataProcessor =
        DataProcessor::init(&configuration, exit_processor.exit.clone()).await?;

    let health_processor = HealthProcessor::init(
        data_processor.receiver,
        data_processor.chain_data.clone(),
        &configuration,
        exit_processor.exit.clone(),
    )
    .await?;


    let publisher = PublisherProcessor::init(
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
        publisher.job,
        persister.job,
    ]
    .into_iter()
    .collect();

    while let Some(_) = jobs.next().await {
        exit_processor.exit.store(true, Ordering::Relaxed);
    }

    Ok(())
}
