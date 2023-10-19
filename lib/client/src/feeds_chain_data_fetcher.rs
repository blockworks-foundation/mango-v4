
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::chain_data::*;

use anchor_lang::Discriminator;

use fixed::types::I80F48;
use mango_v4::accounts_zerocopy::{KeyedAccountSharedData, LoadZeroCopy};
use mango_v4::state::{Bank, MangoAccount, MangoAccountValue};

use anyhow::Context;

use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_sdk::account::{AccountSharedData, ReadableAccount};
use solana_sdk::clock::Slot;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;

/// A complex account fetcher that mostly depends on an external job keeping
/// the chain_data up to date.
///
/// In addition to the usual async fetching interface, it also has synchronous
/// functions to access some kinds of data with less overhead.
///
/// Also, there's functions for fetching up to date data via rpc.
pub struct FeedsAccountFetcher {
    pub chain_data: Arc<RwLock<ChainData>>,
    // pub rpc: RpcClientAsync,
}

impl FeedsAccountFetcher {
    pub fn feeds_fetch_raw(&self, address: &Pubkey) -> anyhow::Result<AccountSharedData> {
        let chain_data = self.chain_data.read().unwrap();
        Ok(chain_data
            .account(address)
            .map(|d| d.account.clone())
            .with_context(|| format!("fetch account {} via chain_data", address))?)
    }
}
