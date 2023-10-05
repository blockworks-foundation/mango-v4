use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

use async_channel::Sender;
use async_trait::async_trait;
use log::*;
use mango_feeds_connector::{account_write_filter::AccountWriteSink, chain_data::AccountData};
use mango_feeds_lib::serum::SerumEventQueueHeader;
use serum_dex::{instruction::MarketInstruction, state::EventView};
use solana_sdk::{
    account::ReadableAccount,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

const MAX_BACKLOG: usize = 2;

pub struct OpenbookCrankSink {
    pks: BTreeMap<Pubkey, Pubkey>,
    instruction_sender: Sender<Vec<Instruction>>,
}

impl OpenbookCrankSink {
    pub fn new(pks: Vec<(Pubkey, Pubkey)>, instruction_sender: Sender<Vec<Instruction>>) -> Self {
        Self {
            pks: pks.iter().copied().collect(),
            instruction_sender,
        }
    }
}

#[async_trait]
impl AccountWriteSink for OpenbookCrankSink {
    async fn process(&self, pk: &Pubkey, account: &AccountData) -> Result<(), String> {
        let account = &account.account;

        let inner_data = &account.data()[5..&account.data().len() - 7];
        let header_span = std::mem::size_of::<SerumEventQueueHeader>();
        let header: SerumEventQueueHeader = *bytemuck::from_bytes(&inner_data[..header_span]);
        let count = header.count;

        let rest = &inner_data[header_span..];
        let event_size = std::mem::size_of::<serum_dex::state::Event>();
        let slop = rest.len() % event_size;
        let end = rest.len() - slop;
        let events = bytemuck::cast_slice::<u8, serum_dex::state::Event>(&rest[..end]);
        let seq_num = header.seq_num;

        let events: Vec<_> = (0..count)
            .map(|i| {
                let offset = (seq_num - count + i) % events.len() as u64;
                let event: serum_dex::state::Event = events[offset as usize];
                event.as_view().unwrap()
            })
            .collect();

        // only crank if at least 1 fill or a sufficient events of other categories are buffered
        let contains_fill_events = events
            .iter()
            .any(|e| matches!(e, serum_dex::state::EventView::Fill { .. }));

        let has_backlog = events.len() > MAX_BACKLOG;
        if !contains_fill_events && !has_backlog {
            return Err("throttled".into());
        }

        let oo_pks: BTreeSet<_> = events
            .iter()
            .map(|e| match e {
                EventView::Fill { owner, .. } | EventView::Out { owner, .. } => {
                    bytemuck::cast_slice::<u64, Pubkey>(owner)[0]
                }
            })
            .collect();

        let mut ams: Vec<_> = oo_pks
            .iter()
            .map(|pk| AccountMeta::new(*pk, false))
            .collect();

        // pass two times evq_pk instead of deprecated fee receivers to reduce encoded tx size
        let mkt_pk = self
            .pks
            .get(pk)
            .unwrap_or_else(|| panic!("{:?} is a known public key", pk));
        ams.append(
            &mut [mkt_pk, pk, /*coin_pk*/ pk, /*pc_pk*/ pk]
                .iter()
                .map(|pk| AccountMeta::new(**pk, false))
                .collect(),
        );

        let ix = Instruction {
            program_id: Pubkey::from_str("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX").unwrap(),
            accounts: ams,
            data: MarketInstruction::ConsumeEvents(count as u16).pack(),
        };

        info!("evq={pk:?} count={count}");
        if let Err(e) = self.instruction_sender.send(vec![ix]).await {
            return Err(e.to_string());
        }

        Ok(())
    }
}
