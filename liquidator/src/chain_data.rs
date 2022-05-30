use {
    log::*, solana_sdk::account::AccountSharedData, solana_sdk::pubkey::Pubkey,
    std::collections::HashMap,
};

use {
    // TODO: None of these should be here
    crate::metrics,
    crate::snapshot_source,
    crate::websocket_source,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SlotStatus {
    Rooted,
    Confirmed,
    Processed,
}

#[derive(Clone, Debug)]
pub struct SlotData {
    pub slot: u64,
    pub parent: Option<u64>,
    pub status: SlotStatus,
    pub chain: u64, // the top slot that this is in a chain with. uncles will have values < tip
}

#[derive(Clone, Debug)]
pub struct AccountData {
    pub slot: u64,
    pub account: AccountSharedData,
}

/// Track slots and account writes
///
/// - use account() to retrieve the current best data for an account.
/// - update_from_snapshot() and update_from_websocket() update the state for new messages
pub struct ChainData {
    /// only slots >= newest_rooted_slot are retained
    slots: HashMap<u64, SlotData>,
    /// writes to accounts, only the latest rooted write an newer are retained
    accounts: HashMap<Pubkey, Vec<AccountData>>,
    newest_rooted_slot: u64,
    newest_processed_slot: u64,

    // storing global metrics here is not good style
    metric_slots_count: metrics::MetricU64,
    metric_accounts_count: metrics::MetricU64,
    metric_account_write_count: metrics::MetricU64,
}

impl ChainData {
    pub fn new(metrics: &metrics::Metrics) -> Self {
        Self {
            slots: HashMap::new(),
            accounts: HashMap::new(),
            newest_rooted_slot: 0,
            newest_processed_slot: 0,
            metric_slots_count: metrics.register_u64("chain_data_slots_count".into()),
            metric_accounts_count: metrics.register_u64("chain_data_accounts_count".into()),
            metric_account_write_count: metrics
                .register_u64("chain_data_account_write_count".into()),
        }
    }

    fn update_slot(&mut self, new_slot: SlotData) {
        let new_processed_head = new_slot.slot > self.newest_processed_slot;
        if new_processed_head {
            self.newest_processed_slot = new_slot.slot;
        }

        let new_rooted_head =
            new_slot.slot > self.newest_rooted_slot && new_slot.status == SlotStatus::Rooted;
        if new_rooted_head {
            self.newest_rooted_slot = new_slot.slot;
        }

        let mut parent_update = false;

        use std::collections::hash_map::Entry;
        match self.slots.entry(new_slot.slot) {
            Entry::Vacant(v) => {
                v.insert(new_slot);
            }
            Entry::Occupied(o) => {
                let v = o.into_mut();
                parent_update = v.parent != new_slot.parent && new_slot.parent.is_some();
                v.parent = v.parent.or(new_slot.parent);
                v.status = new_slot.status;
            }
        };

        if new_processed_head || parent_update {
            // update the "chain" field down to the first rooted slot
            let mut slot = self.newest_processed_slot;
            loop {
                if let Some(data) = self.slots.get_mut(&slot) {
                    data.chain = self.newest_processed_slot;
                    if data.status == SlotStatus::Rooted {
                        break;
                    }
                    if let Some(parent) = data.parent {
                        slot = parent;
                        continue;
                    }
                }
                break;
            }
        }

        if new_rooted_head {
            // for each account, preserve only writes > newest_rooted_slot, or the newest
            // rooted write
            for (_, writes) in self.accounts.iter_mut() {
                let newest_rooted_write = writes
                    .iter()
                    .rev()
                    .find(|w| {
                        w.slot <= self.newest_rooted_slot
                            && self
                                .slots
                                .get(&w.slot)
                                .map(|s| {
                                    // sometimes we seem not to get notifications about slots
                                    // getting rooted, hence assume non-uncle slots < newest_rooted_slot
                                    // are rooted too
                                    s.status == SlotStatus::Rooted
                                        || s.chain == self.newest_processed_slot
                                })
                                // preserved account writes for deleted slots <= newest_rooted_slot
                                // are expected to be rooted
                                .unwrap_or(true)
                    })
                    .map(|w| w.slot)
                    // no rooted write found: produce no effect, since writes > newest_rooted_slot are retained anyway
                    .unwrap_or(self.newest_rooted_slot + 1);
                writes
                    .retain(|w| w.slot == newest_rooted_write || w.slot > self.newest_rooted_slot);
            }

            // now it's fine to drop any slots before the new rooted head
            // as account writes for non-rooted slots before it have been dropped
            self.slots.retain(|s, _| *s >= self.newest_rooted_slot);

            self.metric_slots_count.set(self.slots.len() as u64);
            self.metric_accounts_count.set(self.accounts.len() as u64);
            self.metric_account_write_count.set(
                self.accounts
                    .iter()
                    .map(|(_key, writes)| writes.len() as u64)
                    .sum(),
            );
        }
    }

    fn update_account(&mut self, pubkey: Pubkey, account: AccountData) {
        use std::collections::hash_map::Entry;
        match self.accounts.entry(pubkey) {
            Entry::Vacant(v) => {
                v.insert(vec![account]);
            }
            Entry::Occupied(o) => {
                let v = o.into_mut();
                // v is ordered by slot ascending. find the right position
                // overwrite if an entry for the slot already exists, otherwise insert
                let rev_pos = v
                    .iter()
                    .rev()
                    .position(|d| d.slot <= account.slot)
                    .unwrap_or(v.len());
                let pos = v.len() - rev_pos;
                if pos < v.len() && v[pos].slot == account.slot {
                    v[pos] = account;
                } else {
                    v.insert(pos, account);
                }
            }
        };
    }

    pub fn update_from_snapshot(&mut self, snapshot: snapshot_source::AccountSnapshot) {
        for account_write in snapshot.accounts {
            self.update_account(
                account_write.pubkey,
                AccountData {
                    slot: account_write.slot,
                    account: account_write.account,
                },
            );
        }
    }

    pub fn update_from_websocket(&mut self, message: websocket_source::Message) {
        match message {
            websocket_source::Message::Account(account_write) => {
                trace!("websocket account message");
                self.update_account(
                    account_write.pubkey,
                    AccountData {
                        slot: account_write.slot,
                        account: account_write.account,
                    },
                );
            }
            websocket_source::Message::Slot(slot_update) => {
                trace!("websocket slot message");
                let slot_update = match *slot_update {
                    solana_client::rpc_response::SlotUpdate::CreatedBank {
                        slot, parent, ..
                    } => Some(SlotData {
                        slot,
                        parent: Some(parent),
                        status: SlotStatus::Processed,
                        chain: 0,
                    }),
                    solana_client::rpc_response::SlotUpdate::OptimisticConfirmation {
                        slot,
                        ..
                    } => Some(SlotData {
                        slot,
                        parent: None,
                        status: SlotStatus::Confirmed,
                        chain: 0,
                    }),
                    solana_client::rpc_response::SlotUpdate::Root { slot, .. } => Some(SlotData {
                        slot,
                        parent: None,
                        status: SlotStatus::Rooted,
                        chain: 0,
                    }),
                    _ => None,
                };
                if let Some(update) = slot_update {
                    self.update_slot(update);
                }
            }
        }
    }

    fn is_account_write_live(&self, write: &AccountData) -> bool {
        self.slots
            .get(&write.slot)
            // either the slot is rooted or in the current chain
            .map(|s| s.status == SlotStatus::Rooted || s.chain == self.newest_processed_slot)
            // if the slot can't be found but preceeds newest rooted, use it too (old rooted slots are removed)
            .unwrap_or(write.slot <= self.newest_rooted_slot)
    }

    /// Cloned snapshot of all the most recent live writes per pubkey
    pub fn accounts_snapshot(&self) -> HashMap<Pubkey, AccountData> {
        self.accounts
            .iter()
            .filter_map(|(pubkey, writes)| {
                let latest_good_write = writes
                    .iter()
                    .rev()
                    .find(|w| self.is_account_write_live(w))?;
                Some((pubkey.clone(), latest_good_write.clone()))
            })
            .collect()
    }

    /// Ref to the most recent live write of the pubkey
    pub fn account<'a>(&'a self, pubkey: &Pubkey) -> anyhow::Result<&'a AccountSharedData> {
        self.accounts
            .get(pubkey)
            .ok_or(anyhow::anyhow!("account {} not found", pubkey))?
            .iter()
            .rev()
            .find(|w| self.is_account_write_live(w))
            .ok_or(anyhow::anyhow!("account {} has no live data", pubkey))
            .map(|w| &w.account)
    }
}
