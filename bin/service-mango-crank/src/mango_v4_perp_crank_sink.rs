use std::{
    borrow::BorrowMut,
    collections::{BTreeMap, BTreeSet},
    convert::TryFrom,
};

use async_channel::Sender;
use async_trait::async_trait;
use log::*;
use mango_feeds_connector::{account_write_filter::AccountWriteSink, chain_data::AccountData};
use solana_sdk::{
    account::ReadableAccount,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use bytemuck::cast_ref;

use anchor_lang::AccountDeserialize;

const MAX_BACKLOG: usize = 2;

pub struct MangoV4PerpCrankSink {
    pks: BTreeMap<Pubkey, Pubkey>,
    group_pk: Pubkey,
    instruction_sender: Sender<Vec<Instruction>>,
}

impl MangoV4PerpCrankSink {
    pub fn new(
        pks: Vec<(Pubkey, Pubkey)>,
        group_pk: Pubkey,
        instruction_sender: Sender<Vec<Instruction>>,
    ) -> Self {
        Self {
            pks: pks.iter().copied().collect(),
            group_pk,
            instruction_sender,
        }
    }
}

#[async_trait]
impl AccountWriteSink for MangoV4PerpCrankSink {
    async fn process(&self, pk: &Pubkey, account: &AccountData) -> Result<(), String> {
        let account = &account.account;
        let event_queue: mango_v4::state::EventQueue =
            mango_v4::state::EventQueue::try_deserialize(account.data().borrow_mut()).unwrap();

        // only crank if at least 1 fill or a sufficient events of other categories are buffered
        let contains_fill_events = event_queue
            .iter()
            .any(|e| e.event_type == mango_v4::state::EventType::Fill as u8);
        let has_backlog = event_queue.iter().count() > MAX_BACKLOG;
        if !contains_fill_events && !has_backlog {
            return Err("throttled".into());
        }

        let mango_accounts: BTreeSet<_> = event_queue
            .iter()
            .take(10)
            .flat_map(|e| {
                match mango_v4::state::EventType::try_from(e.event_type).expect("mango v4 event") {
                    mango_v4::state::EventType::Fill => {
                        let fill: &mango_v4::state::FillEvent = cast_ref(e);
                        vec![fill.maker, fill.taker]
                    }
                    mango_v4::state::EventType::Out => {
                        let out: &mango_v4::state::OutEvent = cast_ref(e);
                        vec![out.owner]
                    }
                    mango_v4::state::EventType::Liquidate => vec![],
                }
            })
            .collect();

        let mkt_pk = self
            .pks
            .get(pk)
            .unwrap_or_else(|| panic!("{:?} is a known public key", pk));
        let mut ams: Vec<_> = anchor_lang::ToAccountMetas::to_account_metas(
            &mango_v4::accounts::PerpConsumeEvents {
                group: self.group_pk,
                perp_market: *mkt_pk,
                event_queue: *pk,
            },
            None,
        );
        ams.append(
            &mut mango_accounts
                .iter()
                .map(|pk| AccountMeta::new(*pk, false))
                .collect(),
        );

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: ams,
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::PerpConsumeEvents {
                limit: 10,
            }),
        };

        info!("evq={pk:?} count={} limit=10", event_queue.iter().count());

        if let Err(e) = self.instruction_sender.send(vec![ix]).await {
            return Err(e.to_string());
        }

        Ok(())
    }
}
