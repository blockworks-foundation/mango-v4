use mango_feeds_connector::{
    account_write_filter::{self, AccountWriteRoute},
    metrics::Metrics,
    AccountWrite, SlotUpdate,
};

use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use std::{sync::Arc, time::Duration};

use crate::{
    mango_v4_perp_crank_sink::MangoV4PerpCrankSink, openbook_crank_sink::OpenbookCrankSink,
};

#[allow(clippy::type_complexity)]
pub fn init(
    perp_queue_pks: Vec<(Pubkey, Pubkey)>,
    serum_queue_pks: Vec<(Pubkey, Pubkey)>,
    group_pk: Pubkey,
    metrics_sender: Metrics,
) -> anyhow::Result<(
    async_channel::Sender<AccountWrite>,
    async_channel::Sender<SlotUpdate>,
    async_channel::Receiver<Vec<Instruction>>,
)> {
    // Event queue updates can be consumed by client connections
    let (instruction_sender, instruction_receiver) = async_channel::unbounded::<Vec<Instruction>>();

    let routes = vec![
        AccountWriteRoute {
            matched_pubkeys: serum_queue_pks.iter().map(|(_, evq_pk)| *evq_pk).collect(),
            sink: Arc::new(OpenbookCrankSink::new(
                serum_queue_pks,
                instruction_sender.clone(),
            )),
            timeout_interval: Duration::default(),
        },
        AccountWriteRoute {
            matched_pubkeys: perp_queue_pks.iter().map(|(_, evq_pk)| *evq_pk).collect(),
            sink: Arc::new(MangoV4PerpCrankSink::new(
                perp_queue_pks,
                group_pk,
                instruction_sender,
            )),
            timeout_interval: Duration::default(),
        },
    ];

    let (account_write_queue_sender, slot_queue_sender) =
        account_write_filter::init(routes, metrics_sender)?;

    Ok((
        account_write_queue_sender,
        slot_queue_sender,
        instruction_receiver,
    ))
}
