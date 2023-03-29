use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::chain_data::*;

use anchor_lang::Discriminator;

use mango_v4::accounts_zerocopy::LoadZeroCopy;
use mango_v4::state::{MangoAccount, MangoAccountValue};

use anyhow::Context;

use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_sdk::account::{AccountSharedData, ReadableAccount};
use solana_sdk::clock::Slot;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;

pub struct AccountFetcher {
    pub chain_data: Arc<RwLock<ChainData>>,
    pub rpc: RpcClientAsync,
}

impl AccountFetcher {
    // loads from ChainData
    pub fn fetch<T: anchor_lang::ZeroCopy + anchor_lang::Owner>(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<T> {
        Ok(*self
            .fetch_raw(address)?
            .load::<T>()
            .with_context(|| format!("loading account {}", address))?)
    }

    pub fn fetch_mango_account(&self, address: &Pubkey) -> anyhow::Result<MangoAccountValue> {
        let acc = self.fetch_raw(address)?;

        let data = acc.data();
        if data.len() < 8 {
            anyhow::bail!(
                "account at {} has only {} bytes of data",
                address,
                data.len()
            );
        }
        let disc_bytes = &data[0..8];
        if disc_bytes != MangoAccount::discriminator() {
            anyhow::bail!("not a mango account at {}", address);
        }

        MangoAccountValue::from_bytes(&data[8..])
            .with_context(|| format!("loading mango account {}", address))
    }

    // fetches via RPC, stores in ChainData, returns new version
    pub async fn fetch_fresh<T: anchor_lang::ZeroCopy + anchor_lang::Owner>(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<T> {
        self.refresh_account_via_rpc(address).await?;
        self.fetch(address)
    }

    pub async fn fetch_fresh_mango_account(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<MangoAccountValue> {
        self.refresh_account_via_rpc(address).await?;
        self.fetch_mango_account(address)
    }

    pub fn fetch_raw(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        let chain_data = self.chain_data.read().unwrap();
        Ok(chain_data
            .account(address)
            .map(|d| d.account.clone())
            .with_context(|| format!("fetch account {} via chain_data", address))?)
    }

    pub async fn refresh_account_via_rpc(&self, address: &Pubkey) -> anyhow::Result<Slot> {
        let response = self
            .rpc
            .get_account_with_commitment(address, self.rpc.commitment())
            .await
            .with_context(|| format!("refresh account {} via rpc", address))?;
        let slot = response.context.slot;
        let account = response
            .value
            .ok_or(anchor_client::ClientError::AccountNotFound)
            .with_context(|| format!("refresh account {} via rpc", address))?;

        let mut chain_data = self.chain_data.write().unwrap();
        let best_chain_slot = chain_data.best_chain_slot();

        // The RPC can get information for slots that haven't been seen yet on chaindata. That means
        // that the rpc thinks that slot is valid. Make it so by telling chain data about it.
        if best_chain_slot < slot {
            chain_data.update_slot(SlotData {
                slot,
                parent: Some(best_chain_slot),
                status: SlotStatus::Processed,
                chain: 0,
            });
        }

        chain_data.update_account(
            *address,
            AccountData {
                slot,
                account: account.into(),
                write_version: 1,
            },
        );

        Ok(slot)
    }

    /// Return the maximum slot reported for the processing of the signatures
    pub async fn transaction_max_slot(&self, signatures: &[Signature]) -> anyhow::Result<Slot> {
        let statuses = self.rpc.get_signature_statuses(signatures).await?.value;
        Ok(statuses
            .iter()
            .map(|status_opt| status_opt.as_ref().map(|status| status.slot).unwrap_or(0))
            .max()
            .unwrap_or(0))
    }

    /// Return success once all addresses have data >= min_slot
    pub async fn refresh_accounts_via_rpc_until_slot(
        &self,
        addresses: &[Pubkey],
        min_slot: Slot,
        timeout: Duration,
    ) -> anyhow::Result<()> {
        let start = Instant::now();
        for address in addresses {
            loop {
                if start.elapsed() > timeout {
                    anyhow::bail!(
                        "timeout while waiting for data for {} that's newer than slot {}",
                        address,
                        min_slot
                    );
                }
                let data_slot = self.refresh_account_via_rpc(address).await?;
                if data_slot >= min_slot {
                    break;
                }
                thread::sleep(Duration::from_millis(500));
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::AccountFetcher for AccountFetcher {
    async fn fetch_raw_account(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<solana_sdk::account::AccountSharedData> {
        self.fetch_raw(address)
    }

    async fn fetch_raw_account_lookup_table(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<AccountSharedData> {
        // Fetch data via RPC if missing: the chain data updater doesn't know about all the
        // lookup talbes we may need.
        if let Ok(alt) = self.fetch_raw(address) {
            return Ok(alt);
        }
        self.refresh_account_via_rpc(address).await?;
        self.fetch_raw(address)
    }

    async fn fetch_program_accounts(
        &self,
        program: &Pubkey,
        discriminator: [u8; 8],
    ) -> anyhow::Result<Vec<(Pubkey, AccountSharedData)>> {
        let chain_data = self.chain_data.read().unwrap();
        Ok(chain_data
            .iter_accounts()
            .filter_map(|(pk, data)| {
                if data.account.owner() != program {
                    return None;
                }
                let acc_data = data.account.data();
                if acc_data.len() < 8 || acc_data[..8] != discriminator {
                    return None;
                }
                Some((*pk, data.account.clone()))
            })
            .collect::<Vec<_>>())
    }
}
