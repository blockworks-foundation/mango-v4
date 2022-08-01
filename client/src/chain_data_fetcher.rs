use std::sync::{Arc, RwLock};

use crate::chain_data::*;

use anchor_lang::Discriminator;

use mango_v4::accounts_zerocopy::LoadZeroCopy;
use mango_v4::state::{MangoAccount, MangoAccountValue};

use anyhow::Context;

use solana_client::rpc_client::RpcClient;
use solana_sdk::account::{AccountSharedData, ReadableAccount};
use solana_sdk::pubkey::Pubkey;

pub struct AccountFetcher {
    pub chain_data: Arc<RwLock<ChainData>>,
    pub rpc: RpcClient,
}

impl AccountFetcher {
    // loads from ChainData
    pub fn fetch<T: anchor_lang::ZeroCopy + anchor_lang::Owner>(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<T> {
        Ok(self
            .fetch_raw(address)?
            .load::<T>()
            .with_context(|| format!("loading account {}", address))?
            .clone())
    }

    pub fn fetch_mango_account(&self, address: &Pubkey) -> anyhow::Result<MangoAccountValue> {
        let acc = self.fetch_raw(address)?;

        let data = acc.data();
        let disc_bytes = &data[0..8];
        if disc_bytes != &MangoAccount::discriminator() {
            anyhow::bail!("not a mango account at {}", address);
        }

        Ok(MangoAccountValue::from_bytes(&data[8..])
            .with_context(|| format!("loading mango account {}", address))?)
    }

    // fetches via RPC, stores in ChainData, returns new version
    pub fn fetch_fresh<T: anchor_lang::ZeroCopy + anchor_lang::Owner>(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<T> {
        self.refresh_account_via_rpc(address)?;
        self.fetch(address)
    }

    pub fn fetch_fresh_mango_account(&self, address: &Pubkey) -> anyhow::Result<MangoAccountValue> {
        self.refresh_account_via_rpc(address)?;
        self.fetch_mango_account(address)
    }

    pub fn fetch_raw(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        let chain_data = self.chain_data.read().unwrap();
        Ok(chain_data
            .account(address)
            .with_context(|| format!("fetch account {} via chain_data", address))?
            .clone())
    }

    pub fn refresh_account_via_rpc(&self, address: &Pubkey) -> anyhow::Result<()> {
        let response = self
            .rpc
            .get_account_with_commitment(&address, self.rpc.commitment())
            .with_context(|| format!("refresh account {} via rpc", address))?;
        let account = response
            .value
            .ok_or(anchor_client::ClientError::AccountNotFound)
            .with_context(|| format!("refresh account {} via rpc", address))?;

        let mut chain_data = self.chain_data.write().unwrap();
        chain_data.update_from_rpc(
            address,
            AccountAndSlot {
                slot: response.context.slot,
                account: account.into(),
            },
        );

        Ok(())
    }
}

impl crate::AccountFetcher for AccountFetcher {
    fn fetch_raw_account(&self, address: Pubkey) -> anyhow::Result<solana_sdk::account::Account> {
        self.fetch_raw(&address).map(|a| a.into())
    }
}
