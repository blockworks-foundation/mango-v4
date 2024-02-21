use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::health::HealthType;
use mango_v4::state::{OracleAccountInfos, PerpMarket, PerpMarketIndex};
use mango_v4_client::{chain_data, MangoClient, PreparedInstructions, TransactionBuilder};
use solana_sdk::address_lookup_table_account::AddressLookupTableAccount;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;

use solana_sdk::signer::Signer;
use solana_sdk::transaction::VersionedTransaction;
use tracing::*;
use {fixed::types::I80F48, solana_sdk::pubkey::Pubkey};

pub struct Config {
    /// Amount of time to wait before reusing a positive-pnl account
    pub settle_cooldown: Duration,
}

fn perp_markets_and_prices(
    mango_client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
) -> HashMap<PerpMarketIndex, (PerpMarket, I80F48, I80F48)> {
    mango_client
        .context
        .perp_markets
        .iter()
        .map(|(market_index, perp)| {
            let perp_market = account_fetcher.fetch::<PerpMarket>(&perp.address)?;

            let oracle = account_fetcher.fetch_raw(&perp_market.oracle)?;
            let oracle_acc = &KeyedAccountSharedData::new(perp_market.oracle, oracle);
            let oracle_price =
                perp_market.oracle_price(&OracleAccountInfos::from_reader(oracle_acc), None)?;

            let settle_token = mango_client.context.token(perp_market.settle_token_index);
            let settle_token_price =
                account_fetcher.fetch_bank_price(&settle_token.first_bank())?;

            Ok((
                *market_index,
                (perp_market, oracle_price, settle_token_price),
            ))
        })
        .filter_map(|v: anyhow::Result<_>| match v {
            Ok(v) => Some(v),
            Err(err) => {
                error!("error while retriving perp market and price: {:?}", err);
                None
            }
        })
        .collect()
}

pub struct SettlementState {
    pub mango_client: Arc<MangoClient>,
    pub account_fetcher: Arc<chain_data::AccountFetcher>,
    pub config: Config,

    pub recently_settled: HashMap<Pubkey, Instant>,
}

impl SettlementState {
    pub async fn settle(&mut self, mut accounts: Vec<Pubkey>) -> anyhow::Result<()> {
        {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            accounts.shuffle(&mut rng);
        }

        self.expire_recently_settled();

        self.run_settles(&accounts).await
    }

    fn expire_recently_settled(&mut self) {
        let now = Instant::now();
        self.recently_settled.retain(|_, last_settle| {
            now.duration_since(*last_settle) < self.config.settle_cooldown
        });
    }

    async fn run_settles(&mut self, accounts: &[Pubkey]) -> anyhow::Result<()> {
        let now_ts: u64 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let mango_client = &*self.mango_client;
        let account_fetcher = &*self.account_fetcher;
        let perp_market_info = perp_markets_and_prices(mango_client, account_fetcher);

        // Get settleable pnl for all accounts and markets
        let mut all_positive_settleable =
            HashMap::<PerpMarketIndex, Vec<(Pubkey, I80F48, I80F48)>>::new();
        let mut all_negative_settleable =
            HashMap::<PerpMarketIndex, priority_queue::PriorityQueue<Pubkey, I80F48>>::new();
        for account_key in accounts.iter() {
            let mut account = match account_fetcher.fetch_mango_account(account_key) {
                Ok(acc) => acc,
                Err(e) => {
                    info!("could not fetch account, skipping {account_key}: {e:?}");
                    continue;
                }
            };
            if account.fixed.group != mango_client.group() {
                continue;
            }
            if self.recently_settled.contains_key(account_key) {
                continue;
            }
            let perp_indexes = account
                .active_perp_positions()
                .map(|pp| pp.market_index)
                .collect::<Vec<_>>();
            if perp_indexes.is_empty() {
                continue;
            }

            let health_cache = match mango_client.health_cache(&account).await {
                Ok(hc) => hc,
                Err(_) => continue, // Skip for stale/unconfident oracles
            };
            let liq_end_health = health_cache.health(HealthType::LiquidationEnd);

            for perp_market_index in perp_indexes {
                let (perp_market, perp_price, settle_token_price) =
                    match perp_market_info.get(&perp_market_index) {
                        Some(v) => v,
                        None => continue, // skip accounts with perp positions where we couldn't get the price and market
                    };
                let perp_max_settle = health_cache
                    .perp_max_settle(perp_market.settle_token_index)
                    .expect("perp_max_settle always succeeds when the token index is valid");

                let perp_position = account
                    .perp_position_mut(perp_market_index)
                    .expect("index comes from active_perp_positions()");
                perp_position.settle_funding(perp_market);
                perp_position.update_settle_limit(perp_market, now_ts);

                let unsettled = perp_position
                    .unsettled_pnl(perp_market, *perp_price)
                    .expect("unsettled_pnl always succeeds with the right perp market");
                let limited = perp_position.apply_pnl_settle_limit(perp_market, unsettled);
                let settleable = if limited >= 0 {
                    limited
                } else {
                    limited.max(-perp_max_settle).min(I80F48::ZERO)
                };

                if settleable > 0 {
                    // compute maint health only when needed
                    let maint_health = if liq_end_health < 0 {
                        health_cache.health(HealthType::Maint)
                    } else {
                        liq_end_health
                    };

                    let pnl_value = unsettled * settle_token_price;
                    let position_value =
                        perp_position.base_position_native(perp_market) * perp_price;
                    let fee = perp_market
                        .compute_settle_fee(
                            settleable,
                            pnl_value,
                            position_value,
                            liq_end_health,
                            maint_health,
                        )
                        .expect("always ok");

                    // Assume that settle_fee_flat is near the tx fee, and if we can't possibly
                    // make up for the tx fee even with multiple settle ix in one tx, skip.
                    if fee <= perp_market.settle_fee_flat / 10.0 {
                        continue;
                    }

                    all_positive_settleable
                        .entry(perp_market_index)
                        .or_default()
                        .push((*account_key, settleable, fee));
                } else if settleable < 0 {
                    all_negative_settleable
                        .entry(perp_market_index)
                        .or_default()
                        .push(*account_key, -settleable);
                }
            }
        }

        let address_lookup_tables = mango_client.mango_address_lookup_tables().await?;

        for (perp_market_index, mut positive_settleable) in all_positive_settleable {
            let (perp_market, _, _) = perp_market_info
                .get(&perp_market_index)
                .expect("perp market must exist");
            let negative_settleable = match all_negative_settleable.get_mut(&perp_market_index) {
                None => continue,
                Some(v) => v,
            };
            // sort by fee, descending
            positive_settleable.sort_by_key(|v| -v.2);

            let mut batch_processor = SettleBatchProcessor {
                mango_client,
                account_fetcher,
                perp_market_index,
                instructions: PreparedInstructions::new(),
                max_batch_size: 8, // the 1.4M max CU limit if we assume settle ix can be up to around 150k
                blockhash: mango_client
                    .client
                    .rpc_async()
                    .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
                    .await?
                    .0,
                address_lookup_tables: &address_lookup_tables,
            };

            for (account_a, mut settleable_a, fee) in positive_settleable {
                // Settle account_a as much as we can while still getting a fee for it:
                // Could be that all counterparties are small and we can settle multiple times
                // until account_a is exhausted.
                let mut settled_a_once = false;
                while settleable_a > perp_market.settle_fee_amount_threshold
                    || (settleable_a > 0 && fee != perp_market.settle_fee_flat)
                {
                    // find the best remaining counterparty
                    let (&account_b, &settleable_b) = match negative_settleable.peek() {
                        None => break,
                        Some(v) => v,
                    };

                    let settleable = settleable_a.min(settleable_b);
                    if settleable <= 0
                        || (settleable < perp_market.settle_fee_amount_threshold
                            && fee == perp_market.settle_fee_flat)
                    {
                        // no more interesting pairs that would produce fees
                        break;
                    }

                    batch_processor
                        .add_and_maybe_send(account_a, account_b)
                        .await?;

                    settled_a_once = true;
                    settleable_a -= settleable;
                    negative_settleable.change_priority(&account_b, settleable_b - settleable);
                }
                if settled_a_once {
                    let now = Instant::now();
                    self.recently_settled.insert(account_a, now);
                } else {
                    break;
                }
            }

            // send final batch, if any
            batch_processor.send().await?;
        }

        Ok(())
    }
}

struct SettleBatchProcessor<'a> {
    mango_client: &'a MangoClient,
    account_fetcher: &'a chain_data::AccountFetcher,
    perp_market_index: PerpMarketIndex,
    instructions: PreparedInstructions,
    max_batch_size: usize,
    blockhash: solana_sdk::hash::Hash,
    address_lookup_tables: &'a Vec<AddressLookupTableAccount>,
}

impl<'a> SettleBatchProcessor<'a> {
    fn transaction(&self) -> anyhow::Result<VersionedTransaction> {
        let client = &self.mango_client.client;
        let fee_payer = client.fee_payer();

        TransactionBuilder {
            instructions: self.instructions.clone().to_instructions(),
            address_lookup_tables: self.address_lookup_tables.clone(),
            payer: fee_payer.pubkey(),
            signers: vec![fee_payer],
            config: client.config().transaction_builder_config.clone(),
        }
        .transaction_with_blockhash(self.blockhash)
    }

    async fn send(&mut self) -> anyhow::Result<Option<Signature>> {
        if self.instructions.is_empty() {
            return Ok(None);
        }

        let tx = self.transaction()?;
        self.instructions.clear();

        let send_result = self.mango_client.client.send_transaction(&tx).await;

        match send_result {
            Ok(txsig) => {
                info!("sent settle tx: {txsig}");
                Ok(Some(txsig))
            }
            Err(err) => {
                info!("error while sending settle batch: {}", err);
                Ok(None)
            }
        }
    }

    async fn add_and_maybe_send(
        &mut self,
        account_a: Pubkey,
        account_b: Pubkey,
    ) -> anyhow::Result<Option<Signature>> {
        let a_value = self.account_fetcher.fetch_mango_account(&account_a)?;
        let b_value = self.account_fetcher.fetch_mango_account(&account_b)?;
        let new_ixs = self
            .mango_client
            .perp_settle_pnl_instruction(
                self.perp_market_index,
                (&account_a, &a_value),
                (&account_b, &b_value),
            )
            .await?;
        let previous = self.instructions.clone();
        self.instructions.append(new_ixs.clone());

        // if we exceed the batch limit or tx size limit, send a batch without the new ix
        let max_cu_per_tx = 1_400_000;
        let needs_send = if self.instructions.len() > self.max_batch_size
            || self.instructions.cu >= max_cu_per_tx
        {
            true
        } else {
            let tx = self.transaction()?;
            let serialized = bincode::serialize(&tx)?;
            let too_big = serialized.len() >= solana_sdk::packet::PACKET_DATA_SIZE;
            if too_big && self.instructions.len() == 1 {
                anyhow::bail!(
                    "settle instruction for accounts {} and {} does not fit tx size: {} bytes",
                    account_a,
                    account_b,
                    serialized.len()
                );
            }
            too_big
        };
        if needs_send {
            self.instructions = previous;
            let txsig = self.send().await?;
            self.instructions.append(new_ixs);
            return Ok(txsig);
        }

        Ok(None)
    }
}
