use mango_feeds_connector::chain_data::*;
use mango_feeds_connector::*;
use solana_sdk::{account::WritableAccount, clock::Epoch};
use std::sync::{Arc, RwLock};

pub async fn init(
    chain_data: Arc<RwLock<ChainData>>,
) -> anyhow::Result<(
    async_channel::Sender<AccountWrite>,
    async_channel::Sender<SlotUpdate>,
)> {
    let (account_write_queue_sender, account_write_queue_receiver) =
        async_channel::unbounded::<AccountWrite>();

    let (slot_queue_sender, slot_queue_receiver) = async_channel::unbounded::<SlotUpdate>();

    // update handling thread, reads both slots and account updates
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(account_write) = account_write_queue_receiver.recv() => {
                    let mut chain = chain_data.write().unwrap();
                    chain.update_account(
                        account_write.pubkey,
                        AccountData {
                            slot: account_write.slot,
                            write_version: account_write.write_version,
                            account: WritableAccount::create(
                                account_write.lamports,
                                account_write.data.clone(),
                                account_write.owner,
                                account_write.executable,
                                account_write.rent_epoch as Epoch,
                            ),
                        },
                    );
                }
                Ok(slot_update) = slot_queue_receiver.recv() => {
                    let mut chain = chain_data.write().unwrap();
                    chain.update_slot(SlotData {
                        slot: slot_update.slot,
                        parent: slot_update.parent,
                        status: slot_update.status,
                        chain: 0,
                    });
                }
            }
        }
    });

    Ok((account_write_queue_sender, slot_queue_sender))
}
