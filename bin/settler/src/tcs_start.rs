use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use itertools::Itertools;
use mango_v4::error::{IsAnchorErrorWithCode, MangoError};
use mango_v4::state::*;
use mango_v4_client::{chain_data, error_tracking::ErrorTracking, MangoClient};
use solana_sdk::instruction::Instruction;

use tracing::*;
use {fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

pub struct Config {
    pub persistent_error_report_interval: Duration,
    pub persistent_error_min_duration: Duration,
}

pub struct State {
    pub mango_client: Arc<MangoClient>,
    pub account_fetcher: Arc<chain_data::AccountFetcher>,
    pub config: Config,

    pub errors: ErrorTracking,
    pub last_persistent_error_report: Instant,
}

impl State {
    pub async fn run_pass(&mut self, mut accounts: Vec<Pubkey>) -> anyhow::Result<()> {
        {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            accounts.shuffle(&mut rng);
        }

        self.run_pass_inner(&accounts).await?;
        self.log_persistent_errors();
        Ok(())
    }

    fn log_persistent_errors(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_persistent_error_report)
            < self.config.persistent_error_report_interval
        {
            return;
        }
        self.last_persistent_error_report = now;

        let min_duration = self.config.persistent_error_min_duration;
        self.errors.log_persistent_errors("start_tcs", min_duration);
    }

    async fn run_pass_inner(&mut self, accounts: &Vec<Pubkey>) -> anyhow::Result<()> {
        let now_ts: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs()
            .try_into()?;
        let now = Instant::now();

        let mango_client = &*self.mango_client;
        let account_fetcher = &*self.account_fetcher;

        let mut startable = vec![];
        for account_key in accounts.iter() {
            let account = account_fetcher.fetch_mango_account(account_key).unwrap();
            if account.fixed.group != mango_client.group() {
                continue;
            }
            if self.errors.had_too_many_errors(account_key, now).is_some() {
                continue;
            }

            let mut had_tcs = false;
            for tcs in account.active_token_conditional_swaps() {
                match self.is_tcs_startable(&account, tcs, now_ts) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(e) => {
                        self.errors.record_error(
                            account_key,
                            now,
                            format!("error in is_tcs_startable: tcsid={}, {e:?}", tcs.id),
                        );
                    }
                }
                had_tcs = true;
                startable.push((account_key, tcs.id, tcs.sell_token_index));
            }

            if !had_tcs {
                self.errors.clear_errors(account_key);
            }
        }

        for startable_chunk in startable.chunks(8) {
            let mut instructions = vec![];
            let mut ix_targets = vec![];
            let mut caller_account = mango_client.mango_account().await?;
            for (pubkey, tcs_id, incentive_token_index) in startable_chunk {
                let ix = match self.make_start_ix(pubkey, *tcs_id).await {
                    Ok(v) => v,
                    Err(e) => {
                        self.errors.record_error(
                            pubkey,
                            now,
                            format!("error in make_start_ix: tcsid={tcs_id}, {e:?}"),
                        );
                        continue;
                    }
                };
                instructions.push(ix);
                ix_targets.push((*pubkey, *tcs_id));
                caller_account.ensure_token_position(*incentive_token_index)?;
            }

            // Clear newly created token positions, so the caller account is mostly empty
            for token_index in startable_chunk.iter().map(|(_, _, ti)| *ti).unique() {
                let mint = mango_client.context.token(token_index).mint_info.mint;
                instructions.append(&mut mango_client.token_withdraw_instructions(
                    &caller_account,
                    mint,
                    u64::MAX,
                    false,
                )?);
            }

            let txsig = match mango_client.send_and_confirm_owner_tx(instructions).await {
                Ok(v) => v,
                Err(e) => {
                    warn!("error sending transaction: {e:?}");
                    for pubkey in ix_targets.iter().map(|(pk, _)| pk).unique() {
                        let tcs_ids = ix_targets
                            .iter()
                            .filter_map(|(pk, tcs_id)| (pk == pubkey).then_some(tcs_id))
                            .collect_vec();
                        self.errors.record_error(
                            pubkey,
                            now,
                            format!("error sending transaction: tcsids={tcs_ids:?}, {e:?}"),
                        );
                    }
                    continue;
                }
            };

            info!(%txsig, "sent starting transaction");

            // clear errors on pubkeys with successes
            for pubkey in ix_targets.iter().map(|(pk, _)| pk).unique() {
                self.errors.clear_errors(pubkey);
            }
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
