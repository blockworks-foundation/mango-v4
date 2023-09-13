use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use itertools::Itertools;
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::health::HealthType;
use mango_v4::state::{PerpMarket, PerpMarketIndex};
use mango_v4_client::{
    chain_data, health_cache, prettify_solana_client_error, MangoClient, TransactionBuilder,
};
use solana_sdk::address_lookup_table_account::AddressLookupTableAccount;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Signature;

use solana_sdk::signer::Signer;
use solana_sdk::transaction::VersionedTransaction;
use tracing::*;
use {anyhow::Context, fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

pub struct Config {}

pub struct State {
    pub mango_client: Arc<MangoClient>,
    pub account_fetcher: Arc<chain_data::AccountFetcher>,
    pub config: Config,
    // TODO: reuse liquidator error tracking and escalation?
    //pub recently_settled: HashMap<Pubkey, Instant>,
}

impl State {
    pub async fn run_pass(&mut self, mut accounts: Vec<Pubkey>) -> anyhow::Result<()> {
        {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            accounts.shuffle(&mut rng);
        }

        // self.expire_recently_settled();

        self.run_pass_inner(&accounts).await
    }

    // fn expire_recently_settled(&mut self) {
    //     let now = Instant::now();
    //     self.recently_settled.retain(|_, last_settle| {
    //         now.duration_since(*last_settle) < self.config.settle_cooldown
    //     });
    // }

    async fn run_pass_inner(&mut self, accounts: &Vec<Pubkey>) -> anyhow::Result<()> {
        let now_ts: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs()
            .try_into()?;

        let mango_client = &*self.mango_client;
        let account_fetcher = &*self.account_fetcher;

        let mut startable = vec![];
        for account_key in accounts.iter() {
            let account = account_fetcher.fetch_mango_account(account_key).unwrap();
            if account.fixed.group != mango_client.group() {
                continue;
            }
            // TODO: skip errors

            // TODO: check if any tcs is startable
            // - is premium auction (has incentive)?
            // - trigger condition is met?
            // - can pay incentive?
            let tcs_id = 0u64;
            startable.push((account_key, tcs_id));
        }

        for startable_chunk in startable.chunks(8) {
            let mut instructions = vec![];
            for (pubkey, tcs_id) in startable_chunk {
                let ix = match self.make_start_ix(pubkey, *tcs_id).await {
                    Ok(v) => v,
                    Err(e) => {
                        // TODO: error tracking
                        continue;
                    }
                };
                instructions.push(ix);
            }

            let txsig = match mango_client.send_and_confirm_owner_tx(instructions).await {
                Ok(v) => v,
                Err(e) => {
                    // TODO: error tracking for involved ones
                    continue;
                }
            };
            info!(%txsig, "started");
        }

        Ok(())
    }

    async fn make_start_ix(&self, pubkey: &Pubkey, tcs_id: u64) -> anyhow::Result<Instruction> {
        let account = self.account_fetcher.fetch_mango_account(pubkey).unwrap();
        self.mango_client
            .token_conditional_swap_start_instruction((pubkey, &account), tcs_id)
            .await
    }
}
