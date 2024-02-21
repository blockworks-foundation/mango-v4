use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use itertools::Itertools;
use mango_v4::error::{IsAnchorErrorWithCode, MangoError};
use mango_v4::state::*;
use mango_v4_client::PreparedInstructions;
use mango_v4_client::{chain_data, error_tracking::ErrorTracking, MangoClient};

use tracing::*;
use {fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

pub struct Config {
    pub persistent_error_report_interval: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    StartTcs,
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartTcs => write!(f, "start-tcs"),
        }
    }
}

pub struct State {
    pub mango_client: Arc<MangoClient>,
    pub account_fetcher: Arc<chain_data::AccountFetcher>,
    pub config: Config,

    pub errors: ErrorTracking<Pubkey, ErrorType>,
}

impl State {
    pub async fn run_pass(&mut self, mut accounts: Vec<Pubkey>) -> anyhow::Result<()> {
        {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            accounts.shuffle(&mut rng);
        }

        self.run_pass_inner(&accounts).await?;
        self.errors.update();
        Ok(())
    }

    async fn run_pass_inner(&mut self, accounts: &[Pubkey]) -> anyhow::Result<()> {
        let now_ts: u64 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let now = Instant::now();

        let mango_client = &*self.mango_client;
        let account_fetcher = &*self.account_fetcher;

        let mut startable = vec![];
        for account_key in accounts.iter() {
            let account = match account_fetcher.fetch_mango_account(account_key) {
                Ok(acc) => acc,
                Err(e) => {
                    info!("could not fetch account, skipping {account_key}: {e:?}");
                    continue;
                }
            };
            if account.fixed.group != mango_client.group() {
                continue;
            }
            if self
                .errors
                .had_too_many_errors(ErrorType::StartTcs, account_key, now)
                .is_some()
            {
                continue;
            }

            let mut had_tcs = false;
            for tcs in account.active_token_conditional_swaps() {
                match self.is_tcs_startable(&account, tcs, now_ts) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(e) => {
                        self.errors.record(
                            ErrorType::StartTcs,
                            account_key,
                            format!("error in is_tcs_startable: tcsid={}, {e:?}", tcs.id),
                        );
                    }
                }
                had_tcs = true;
                startable.push((account_key, tcs.id, tcs.sell_token_index));
            }

            if !had_tcs {
                self.errors.clear(ErrorType::StartTcs, account_key);
            }
        }

        for startable_chunk in startable.chunks(8) {
            let mut instructions = PreparedInstructions::new();
            let mut ix_targets = vec![];
            let mut liqor_account = mango_client.mango_account().await?;
            for (pubkey, tcs_id, incentive_token_index) in startable_chunk {
                // can only batch until all token positions are full
                if let Err(_) = liqor_account.ensure_token_position(*incentive_token_index) {
                    break;
                }

                let ixs = match self.make_start_ix(pubkey, *tcs_id).await {
                    Ok(v) => v,
                    Err(e) => {
                        self.errors.record(
                            ErrorType::StartTcs,
                            pubkey,
                            format!("error in make_start_ix: tcsid={tcs_id}, {e:?}"),
                        );
                        continue;
                    }
                };
                instructions.append(ixs);
                ix_targets.push((*pubkey, *tcs_id));
            }

            // Clear newly created token positions, so the liqor account is mostly empty
            let new_token_pos_indices = startable_chunk
                .iter()
                .map(|(_, _, ti)| *ti)
                .unique()
                .collect_vec();
            for token_index in new_token_pos_indices {
                let mint = mango_client.context.token(token_index).mint;
                let ix = match mango_client
                    .token_withdraw_instructions(&liqor_account, mint, u64::MAX, false)
                    .await
                {
                    Ok(ix) => ix,
                    Err(_) => continue,
                };

                instructions.append(ix)
            }

            let txsig = match mango_client
                .send_and_confirm_owner_tx(instructions.to_instructions())
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    warn!("error sending transaction: {e:?}");
                    for pubkey in ix_targets.iter().map(|(pk, _)| pk).unique() {
                        let tcs_ids = ix_targets
                            .iter()
                            .filter_map(|(pk, tcs_id)| (pk == pubkey).then_some(tcs_id))
                            .collect_vec();
                        self.errors.record(
                            ErrorType::StartTcs,
                            pubkey,
                            format!("error sending transaction: tcsids={tcs_ids:?}, {e:?}"),
                        );
                    }
                    continue;
                }
            };

            info!(%txsig, "sent starting transaction");

            // clear errors on pubkeys with successes
            for pubkey in ix_targets.iter().map(|(pk, _)| pk).unique() {
                self.errors.clear(ErrorType::StartTcs, pubkey);
            }
        }

        Ok(())
    }

    async fn make_start_ix(
        &self,
        pubkey: &Pubkey,
        tcs_id: u64,
    ) -> anyhow::Result<PreparedInstructions> {
        let account = self.account_fetcher.fetch_mango_account(pubkey)?;
        self.mango_client
            .token_conditional_swap_start_instruction((pubkey, &account), tcs_id)
            .await
    }

    fn oracle_for_token(&self, token_index: TokenIndex) -> anyhow::Result<I80F48> {
        let bank_pk = self.mango_client.context.token(token_index).first_bank();
        self.account_fetcher.fetch_bank_price(&bank_pk)
    }

    fn is_tcs_startable(
        &self,
        account: &MangoAccountValue,
        tcs: &TokenConditionalSwap,
        now_ts: u64,
    ) -> anyhow::Result<bool> {
        if !tcs.is_startable_type() || tcs.is_expired(now_ts) || tcs.passed_start(now_ts) {
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
