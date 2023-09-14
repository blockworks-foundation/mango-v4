use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use itertools::Itertools;
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::error::{IsAnchorErrorWithCode, MangoError};
use mango_v4::health::HealthType;
use mango_v4::state::*;
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

            for tcs in account.active_token_conditional_swaps() {
                match self.is_tcs_startable(&account, tcs, now_ts) {
                    Ok(true) => startable.push((account_key, tcs.id)),
                    Ok(false) => {}
                    Err(e) => {
                        // TODO: error tracking
                    }
                }
            }
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
            // TODO: also track successses, so we don't try to start the same thing too often
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

    fn oracle_for_token(&self, token_index: TokenIndex) -> anyhow::Result<I80F48> {
        let bank_pk = self
            .mango_client
            .context
            .token(token_index)
            .mint_info
            .first_bank();
        self.account_fetcher.fetch_bank_price(&bank_pk)
    }

    fn is_tcs_startable(
        &self,
        account: &MangoAccountValue,
        tcs: &TokenConditionalSwap,
        now_ts: u64,
    ) -> anyhow::Result<bool> {
        if !tcs.has_incentive_for_starting() || tcs.is_expired(now_ts) || tcs.passed_start(now_ts) {
            return Ok(false);
        }

        let buy_price = self.oracle_for_token(tcs.buy_token_index)?;
        let sell_price = self.oracle_for_token(tcs.sell_token_index)?;

        let price = buy_price.to_num::<f64>() / sell_price.to_num::<f64>();
        if !tcs.is_startable(price, now_ts) {
            return Ok(false);
        }

        // Check if it's possible to deduct the incentive:
        // borrow limitations in the tcs, the bank or the net borrow limit may intervene
        let incentive = (I80F48::from(TCS_START_INCENTIVE) / sell_price)
            .min(I80F48::from(tcs.remaining_sell()));
        let sell_bank_pk = self
            .mango_client
            .context
            .token(tcs.sell_token_index)
            .mint_info
            .first_bank();
        let mut sell_bank: Bank = self.account_fetcher.fetch(&sell_bank_pk)?;
        let sell_pos = account.token_position(tcs.sell_token_index)?;
        let sell_pos_native = sell_pos.native(&sell_bank);
        if sell_pos_native < incentive {
            if !tcs.allow_creating_borrows() || sell_bank.are_borrows_reduce_only() {
                return Ok(false);
            }

            let mut account_copy = account.clone();
            let sell_pos_mut = account_copy.token_position_mut(tcs.sell_token_index)?.0;
            sell_bank.withdraw_with_fee(sell_pos_mut, incentive, now_ts)?;

            let result = sell_bank.check_net_borrows(sell_price);
            if result.is_anchor_error_with_code(MangoError::BankNetBorrowsLimitReached.into()) {
                return Ok(false);
            }
            result?;
        }

        Ok(true)
    }
}
