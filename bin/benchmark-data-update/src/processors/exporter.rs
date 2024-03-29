use csv::Writer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::warn;

use crate::configuration::Configuration;

use super::data::{AccountUpdateEvent, DataEvent, DataEventSource};

pub struct ExporterProcessor {
    pub job: JoinHandle<()>,
}

impl ExporterProcessor {
    pub async fn init(
        configuration: &Configuration,
        data_sender_1: &tokio::sync::broadcast::Sender<DataEvent>,
        data_sender_2: &tokio::sync::broadcast::Sender<DataEvent>,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<ExporterProcessor> {
        let export_csv_path = configuration.export_csv_path.clone();
        let mut data_1 = data_sender_1.subscribe();
        let mut data_2: tokio::sync::broadcast::Receiver<DataEvent> = data_sender_2.subscribe();

        let job = tokio::spawn(async move {
            let mut wtr = Writer::from_path(export_csv_path).expect("could not create csv file");
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(5 * 1000));

            wtr.write_record(&["slot", "time", "source", "account", "snap"])
                .expect("failed to write header");

            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down logger processor...");
                    break;
                }

                tokio::select! {
                    _ = interval.tick() => {
                        wtr.flush().expect("flushing csv file failed");
                    },
                    Ok(msg) = data_1.recv() => Self::handle(msg, &mut wtr),
                    Ok(msg) = data_2.recv() => Self::handle(msg, &mut wtr),
                }
            }
        });

        let result = ExporterProcessor { job };

        Ok(result)
    }

    fn handle_account<T: std::io::Write>(
        upd: AccountUpdateEvent,
        writer: &mut Writer<T>,
        is_snapshot: bool,
    ) {
        let source = match upd.source {
            DataEventSource::Websocket => "ws".to_string(),
            DataEventSource::Grpc => "grpc".to_string(),
        };
        let snap = match is_snapshot {
            true => "snapshot".to_string(),
            false => "single".to_string(),
        };
        writer
            .write_record(&[
                upd.slot.to_string(),
                upd.received_at.to_string(),
                source,
                upd.account.to_string(),
                snap,
            ])
            .expect("failed to write account update");
    }

    fn handle<T: std::io::Write>(msg: DataEvent, writer: &mut Writer<T>) {
        match msg {
            DataEvent::Other => {}
            DataEvent::Snapshot(upd) => {
                for acc in upd.accounts {
                    Self::handle_account(
                        AccountUpdateEvent {
                            received_at: upd.received_at,
                            account: acc,
                            source: upd.source,
                            slot: upd.slot,
                        },
                        writer,
                        true,
                    );
                }
            }
            DataEvent::AccountUpdate(upd) => {
                Self::handle_account(upd, writer, false);
            }
        }
    }
}
