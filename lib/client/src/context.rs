use std::collections::HashMap;

use anchor_client::ClientError;

use anchor_lang::__private::bytemuck;

use mango_v4::{
    accounts_zerocopy::{KeyedAccountReader, KeyedAccountSharedData},
    state::{
        determine_oracle_type, load_whirlpool_state, oracle_state_unchecked, Group,
        MangoAccountValue, OracleAccountInfos, OracleConfig, OracleConfigParams, OracleType,
        PerpMarketIndex, Serum3MarketIndex, TokenIndex, MAX_BANKS,
    },
};

use fixed::types::I80F48;
use futures::{stream, StreamExt, TryStreamExt};
use itertools::Itertools;

use crate::{gpa::*, AccountFetcher, FallbackOracleConfig};

use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_sdk::account::Account;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, PartialEq, Eq)]
pub struct TokenContext {
    pub group: Pubkey,
    pub token_index: TokenIndex,
    pub name: String,
    pub mint: Pubkey,
    pub oracle: Pubkey,
    pub banks: [Pubkey; MAX_BANKS],
    pub vaults: [Pubkey; MAX_BANKS],
    pub fallback_context: FallbackOracleContext,
    pub mint_info_address: Pubkey,
    pub decimals: u8,
    pub oracle_config: OracleConfig,
}

impl TokenContext {
    pub fn native_to_ui(&self, native: I80F48) -> f64 {
        (native / I80F48::from(10u64.pow(self.decimals.into()))).to_num()
    }

    pub fn first_bank(&self) -> Pubkey {
        self.banks[0]
    }

    pub fn first_vault(&self) -> Pubkey {
        self.vaults[0]
    }

    pub fn banks(&self) -> &[Pubkey] {
        let n_banks = self
            .banks
            .iter()
            .position(|&b| b == Pubkey::default())
            .unwrap_or(MAX_BANKS);
        &self.banks[..n_banks]
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FallbackOracleContext {
    pub key: Pubkey,
    // only used for CLMM fallback oracles, otherwise Pubkey::default
    pub quote_key: Pubkey,
}
impl FallbackOracleContext {
    pub fn keys(&self) -> Vec<Pubkey> {
        vec![self.key, self.quote_key]
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Serum3MarketContext {
    pub address: Pubkey,
    pub name: String,
    pub serum_program: Pubkey,
    pub serum_market_external: Pubkey,
    pub base_token_index: TokenIndex,
    pub quote_token_index: TokenIndex,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_q: Pubkey,
    pub req_q: Pubkey,
    pub coin_vault: Pubkey,
    pub pc_vault: Pubkey,
    pub vault_signer: Pubkey,
    pub coin_lot_size: u64,
    pub pc_lot_size: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PerpMarketContext {
    pub group: Pubkey,
    pub perp_market_index: PerpMarketIndex,
    pub settle_token_index: TokenIndex,
    pub address: Pubkey,
    pub name: String,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_queue: Pubkey,
    pub oracle: Pubkey,
    pub base_lot_size: i64,
    pub quote_lot_size: i64,
    pub base_decimals: u8,
    pub init_overall_asset_weight: I80F48,
}

pub struct ComputeEstimates {
    pub cu_per_mango_instruction: u32,
    pub health_cu_per_token: u32,
    pub health_cu_per_perp: u32,
    pub health_cu_per_serum: u32,
    pub cu_per_serum3_order_match: u32,
    pub cu_per_serum3_order_cancel: u32,
    pub cu_per_perp_order_match: u32,
    pub cu_per_perp_order_cancel: u32,
    pub cu_per_oracle_fallback: u32,
    pub cu_per_charge_collateral_fees: u32,
    pub cu_per_charge_collateral_fees_token: u32,
}

impl Default for ComputeEstimates {
    fn default() -> Self {
        Self {
            cu_per_mango_instruction: 100_000,
            health_cu_per_token: 5000,
            health_cu_per_perp: 8000,
            health_cu_per_serum: 6000,
            // measured around 1.5k, see test_serum_compute
            cu_per_serum3_order_match: 3_000,
            // measured around 11k, see test_serum_compute
            cu_per_serum3_order_cancel: 20_000,
            // measured around 3.5k, see test_perp_compute
            cu_per_perp_order_match: 7_000,
            // measured around 3.5k, see test_perp_compute
            cu_per_perp_order_cancel: 7_000,
            // measured around 2k, see test_health_compute_tokens_fallback_oracles
            cu_per_oracle_fallback: 2000,
            // the base cost is mostly the division
            cu_per_charge_collateral_fees: 20_000,
            // per-chargable-token cost
            cu_per_charge_collateral_fees_token: 12_000,
        }
    }
}

impl ComputeEstimates {
    pub fn health_for_counts(
        &self,
        tokens: usize,
        perps: usize,
        serums: usize,
        fallbacks: usize,
    ) -> u32 {
        let tokens: u32 = tokens.try_into().unwrap();
        let perps: u32 = perps.try_into().unwrap();
        let serums: u32 = serums.try_into().unwrap();
        let fallbacks: u32 = fallbacks.try_into().unwrap();
        tokens * self.health_cu_per_token
            + perps * self.health_cu_per_perp
            + serums * self.health_cu_per_serum
            + fallbacks * self.cu_per_oracle_fallback
    }

    pub fn health_for_account(&self, account: &MangoAccountValue, num_fallbacks: usize) -> u32 {
        self.health_for_counts(
            account.active_token_positions().count(),
            account.active_perp_positions().count(),
            account.active_serum3_orders().count(),
            num_fallbacks,
        )
    }
}

pub struct MangoGroupContext {
    pub group: Pubkey,

    pub tokens: HashMap<TokenIndex, TokenContext>,
    pub token_indexes_by_name: HashMap<String, TokenIndex>,

    pub serum3_markets: HashMap<Serum3MarketIndex, Serum3MarketContext>,
    pub serum3_market_indexes_by_name: HashMap<String, Serum3MarketIndex>,

    pub perp_markets: HashMap<PerpMarketIndex, PerpMarketContext>,
    pub perp_market_indexes_by_name: HashMap<String, PerpMarketIndex>,

    pub address_lookup_tables: Vec<Pubkey>,

    pub compute_estimates: ComputeEstimates,
}

impl MangoGroupContext {
    pub fn mint_info_address(&self, token_index: TokenIndex) -> Pubkey {
        self.token(token_index).mint_info_address
    }

    pub fn perp(&self, perp_market_index: PerpMarketIndex) -> &PerpMarketContext {
        self.perp_markets.get(&perp_market_index).unwrap()
    }

    pub fn perp_market_address(&self, perp_market_index: PerpMarketIndex) -> Pubkey {
        self.perp(perp_market_index).address
    }

    pub fn serum3_market_index(&self, name: &str) -> Serum3MarketIndex {
        *self.serum3_market_indexes_by_name.get(name).unwrap()
    }

    pub fn serum3(&self, market_index: Serum3MarketIndex) -> &Serum3MarketContext {
        self.serum3_markets.get(&market_index).unwrap()
    }

    pub fn serum3_base_token(&self, market_index: Serum3MarketIndex) -> &TokenContext {
        self.token(self.serum3(market_index).base_token_index)
    }

    pub fn serum3_quote_token(&self, market_index: Serum3MarketIndex) -> &TokenContext {
        self.token(self.serum3(market_index).quote_token_index)
    }

    pub fn token(&self, token_index: TokenIndex) -> &TokenContext {
        self.tokens.get(&token_index).unwrap()
    }

    pub fn token_by_mint(&self, mint: &Pubkey) -> anyhow::Result<&TokenContext> {
        self.tokens
            .values()
            .find(|tc| tc.mint == *mint)
            .ok_or_else(|| anyhow::anyhow!("no token for mint {}", mint))
    }

    pub fn token_by_name(&self, name: &str) -> &TokenContext {
        let mut tc_iter = self.tokens.values().filter(|tc| tc.name == name);
        let tc = tc_iter.next();
        assert!(
            tc.is_some(),
            "token {name} not found; names {:?}",
            self.tokens.values().map(|tc| tc.name.clone()).collect_vec()
        );
        assert!(tc_iter.next().is_none(), "multiple token {name} found");
        tc.unwrap()
    }

    pub async fn new_from_rpc(rpc: &RpcClientAsync, group: Pubkey) -> anyhow::Result<Self> {
        let program = mango_v4::ID;

        // tokens
        let mint_info_tuples = fetch_mint_infos(rpc, program, group).await?;
        let mut tokens = mint_info_tuples
            .iter()
            .map(|(pk, mi)| {
                (
                    mi.token_index,
                    TokenContext {
                        token_index: mi.token_index,
                        name: String::new(),
                        mint_info_address: *pk,
                        decimals: u8::MAX,
                        banks: mi.banks,
                        vaults: mi.vaults,
                        oracle: mi.oracle,
                        fallback_context: FallbackOracleContext {
                            key: mi.fallback_oracle,
                            quote_key: Pubkey::default(),
                        },
                        oracle_config: OracleConfigParams::default().to_oracle_config(),
                        group: mi.group,
                        mint: mi.mint,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        // reading the banks is only needed for the token names, decimals and oracle configs
        // FUTURE: either store the names on MintInfo as well, or maybe don't store them at all
        //         because they are in metaplex?
        let bank_tuples = fetch_banks(rpc, program, group).await?;
        let fallback_keys: Vec<Pubkey> = bank_tuples
            .iter()
            .map(|tup| tup.1.fallback_oracle)
            .collect();
        let fallback_oracle_accounts = fetch_multiple_accounts(rpc, &fallback_keys[..]).await?;
        for (index, (_, bank)) in bank_tuples.iter().enumerate() {
            let token = tokens.get_mut(&bank.token_index).unwrap();
            token.name = bank.name().into();
            token.decimals = bank.mint_decimals;
            token.oracle_config = bank.oracle_config;
            let (key, acc_info) = fallback_oracle_accounts[index].clone();
            token.fallback_context.quote_key =
                get_fallback_quote_key(&KeyedAccountSharedData::new(key, acc_info));
        }
        assert!(tokens.values().all(|t| t.decimals != u8::MAX));

        // serum3 markets
        let serum3_market_tuples = fetch_serum3_markets(rpc, program, group).await?;
        let serum3_markets_external = stream::iter(serum3_market_tuples.iter())
            .then(|(_, s)| fetch_raw_account(rpc, s.serum_market_external))
            .try_collect::<Vec<_>>()
            .await?;
        let serum3_markets = serum3_market_tuples
            .iter()
            .zip(serum3_markets_external.iter())
            .map(|((pk, s), market_external_account)| {
                let market_external: &serum_dex::state::MarketState = bytemuck::from_bytes(
                    &market_external_account.data
                        [5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
                );
                let vault_signer = serum_dex::state::gen_vault_signer_key(
                    market_external.vault_signer_nonce,
                    &s.serum_market_external,
                    &s.serum_program,
                )
                .unwrap();
                (
                    s.market_index,
                    Serum3MarketContext {
                        address: *pk,
                        base_token_index: s.base_token_index,
                        quote_token_index: s.quote_token_index,
                        name: s.name().to_string(),
                        serum_program: s.serum_program,
                        serum_market_external: s.serum_market_external,
                        bids: from_serum_style_pubkey(market_external.bids),
                        asks: from_serum_style_pubkey(market_external.asks),
                        event_q: from_serum_style_pubkey(market_external.event_q),
                        req_q: from_serum_style_pubkey(market_external.req_q),
                        coin_vault: from_serum_style_pubkey(market_external.coin_vault),
                        pc_vault: from_serum_style_pubkey(market_external.pc_vault),
                        vault_signer,
                        coin_lot_size: market_external.coin_lot_size,
                        pc_lot_size: market_external.pc_lot_size,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        // perp markets
        let perp_market_tuples = fetch_perp_markets(rpc, program, group).await?;
        let perp_markets = perp_market_tuples
            .iter()
            .map(|(pk, pm)| {
                (
                    pm.perp_market_index,
                    PerpMarketContext {
                        address: *pk,
                        group: pm.group,
                        oracle: pm.oracle,
                        perp_market_index: pm.perp_market_index,
                        settle_token_index: pm.settle_token_index,
                        asks: pm.asks,
                        bids: pm.bids,
                        event_queue: pm.event_queue,
                        base_decimals: pm.base_decimals,
                        base_lot_size: pm.base_lot_size,
                        quote_lot_size: pm.quote_lot_size,
                        init_overall_asset_weight: pm.init_overall_asset_weight,
                        name: pm.name().to_string(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        // Name lookup tables
        let token_indexes_by_name = tokens
            .iter()
            .map(|(i, t)| (t.name.clone(), *i))
            .collect::<HashMap<_, _>>();
        let serum3_market_indexes_by_name = serum3_markets
            .iter()
            .map(|(i, s)| (s.name.clone(), *i))
            .collect::<HashMap<_, _>>();
        let perp_market_indexes_by_name = perp_markets
            .iter()
            .map(|(i, p)| (p.name.clone(), *i))
            .collect::<HashMap<_, _>>();

        let group_data = fetch_anchor_account::<Group>(rpc, &group).await?;
        let address_lookup_tables = group_data
            .address_lookup_tables
            .iter()
            .filter(|&&k| k != Pubkey::default())
            .cloned()
            .collect::<Vec<Pubkey>>();

        Ok(MangoGroupContext {
            group,
            tokens,
            token_indexes_by_name,
            serum3_markets,
            serum3_market_indexes_by_name,
            perp_markets,
            perp_market_indexes_by_name,
            address_lookup_tables,
            compute_estimates: ComputeEstimates::default(),
        })
    }

    pub fn derive_health_check_remaining_account_metas(
        &self,
        account: &MangoAccountValue,
        affected_tokens: Vec<TokenIndex>,
        writable_banks: Vec<TokenIndex>,
        affected_perp_markets: Vec<PerpMarketIndex>,
        fallback_contexts: HashMap<Pubkey, FallbackOracleContext>,
    ) -> anyhow::Result<(Vec<AccountMeta>, u32)> {
        let mut account = account.clone();
        for affected_token_index in affected_tokens.iter().chain(writable_banks.iter()) {
            account.ensure_token_position(*affected_token_index)?;
        }
        for affected_perp_market_index in affected_perp_markets {
            let settle_token_index = self.perp(affected_perp_market_index).settle_token_index;
            account.ensure_perp_position(affected_perp_market_index, settle_token_index)?;
        }

        // figure out all the banks/oracles that need to be passed for the health check
        let mut banks = vec![];
        let mut oracles = vec![];
        let mut fallbacks = vec![];
        for position in account.active_token_positions() {
            let token = self.token(position.token_index);
            banks.push((
                token.first_bank(),
                writable_banks.iter().any(|&ti| ti == position.token_index),
            ));
            oracles.push(token.oracle);
            if let Some(fallback_context) = fallback_contexts.get(&token.oracle) {
                fallbacks.extend(fallback_context.keys());
            }
        }

        let serum_oos = account.active_serum3_orders().map(|&s| s.open_orders);
        let perp_markets = account
            .active_perp_positions()
            .map(|&pa| self.perp_market_address(pa.market_index));
        let perp_oracles = account
            .active_perp_positions()
            .map(|&pa| self.perp(pa.market_index).oracle);
        // FUTURE: implement fallback oracles for perps

        let fallback_oracles: Vec<Pubkey> = fallbacks
            .into_iter()
            .unique()
            .filter(|key| !oracles.contains(key) && key != &Pubkey::default())
            .collect();
        let fallbacks_len = fallback_oracles.len();

        let to_account_meta = |pubkey| AccountMeta {
            pubkey,
            is_writable: false,
            is_signer: false,
        };

        let accounts = banks
            .iter()
            .map(|&(pubkey, is_writable)| AccountMeta {
                pubkey,
                is_writable,
                is_signer: false,
            })
            .chain(oracles.into_iter().map(to_account_meta))
            .chain(perp_markets.map(to_account_meta))
            .chain(perp_oracles.map(to_account_meta))
            .chain(serum_oos.map(to_account_meta))
            .chain(fallback_oracles.into_iter().map(to_account_meta))
            .collect();

        let cu = self
            .compute_estimates
            .health_for_account(&account, fallbacks_len);

        Ok((accounts, cu))
    }

    pub fn derive_health_check_remaining_account_metas_two_accounts(
        &self,
        account1: &MangoAccountValue,
        account2: &MangoAccountValue,
        affected_tokens: &[TokenIndex],
        writable_banks: &[TokenIndex],
        fallback_contexts: HashMap<Pubkey, FallbackOracleContext>,
    ) -> anyhow::Result<(Vec<AccountMeta>, u32)> {
        // figure out all the banks/oracles that need to be passed for the health check
        let mut banks = vec![];
        let mut oracles = vec![];
        let mut fallbacks = vec![];

        let token_indexes = account2
            .active_token_positions()
            .chain(account1.active_token_positions())
            .map(|ta| ta.token_index)
            .chain(affected_tokens.iter().copied())
            .unique();

        for token_index in token_indexes {
            let token = self.token(token_index);
            let writable_bank = writable_banks.iter().contains(&token_index);
            banks.push((token.first_bank(), writable_bank));
            oracles.push(token.oracle);
            if let Some(fallback_context) = fallback_contexts.get(&token.oracle) {
                fallbacks.extend(fallback_context.keys());
            }
        }

        let serum_oos = account2
            .active_serum3_orders()
            .chain(account1.active_serum3_orders())
            .map(|&s| s.open_orders);
        let perp_market_indexes = account2
            .active_perp_positions()
            .chain(account1.active_perp_positions())
            .map(|&pa| pa.market_index)
            .unique()
            .collect::<Vec<_>>();
        let perp_markets = perp_market_indexes
            .iter()
            .map(|&index| self.perp_market_address(index));
        let perp_oracles = perp_market_indexes
            .iter()
            .map(|&index| self.perp(index).oracle);
        // FUTURE: implement fallback oracles for perps

        let fallback_oracles: Vec<Pubkey> = fallbacks
            .into_iter()
            .unique()
            .filter(|key| !oracles.contains(key) && key != &Pubkey::default())
            .collect();
        let fallbacks_len = fallback_oracles.len();

        let to_account_meta = |pubkey| AccountMeta {
            pubkey,
            is_writable: false,
            is_signer: false,
        };

        let accounts = banks
            .iter()
            .map(|(pubkey, is_writable)| AccountMeta {
                pubkey: *pubkey,
                is_writable: *is_writable,
                is_signer: false,
            })
            .chain(oracles.into_iter().map(to_account_meta))
            .chain(perp_markets.map(to_account_meta))
            .chain(perp_oracles.map(to_account_meta))
            .chain(serum_oos.map(to_account_meta))
            .chain(fallback_oracles.into_iter().map(to_account_meta))
            .collect();

        // Since health is likely to be computed separately for both accounts, we don't use the
        // unique'd counts to estimate health cu cost.
        let account1_token_count = account1
            .active_token_positions()
            .map(|ta| ta.token_index)
            .chain(affected_tokens.iter().copied())
            .unique()
            .count();
        let account2_token_count = account2
            .active_token_positions()
            .map(|ta| ta.token_index)
            .chain(affected_tokens.iter().copied())
            .unique()
            .count();
        let cu = self.compute_estimates.health_for_counts(
            account1_token_count,
            account1.active_perp_positions().count(),
            account1.active_serum3_orders().count(),
            fallbacks_len,
        ) + self.compute_estimates.health_for_counts(
            account2_token_count,
            account2.active_perp_positions().count(),
            account2.active_serum3_orders().count(),
            fallbacks_len,
        );

        Ok((accounts, cu))
    }

    /// Returns true if the on-chain context changed significantly, this currently means:
    /// - new listings (token, serum, perp)
    /// - oracle pubkey or config changes
    /// - other config changes visible through the context
    /// This is done because those would affect the pubkeys the websocket streams need to listen to,
    /// or change limits, oracle staleness or other relevant configuration.
    pub fn changed_significantly(&self, other: &Self) -> bool {
        if other.tokens.len() != self.tokens.len() {
            return true;
        }
        for (&ti, old) in self.tokens.iter() {
            if old != other.token(ti) {
                return true;
            }
        }

        if other.serum3_markets.len() != self.serum3_markets.len() {
            return true;
        }
        for (&mi, old) in self.serum3_markets.iter() {
            if old != other.serum3(mi) {
                return true;
            }
        }

        if other.perp_markets.len() != self.perp_markets.len() {
            return true;
        }
        for (&pi, old) in self.perp_markets.iter() {
            if old != other.perp(pi) {
                return true;
            }
        }

        if other.address_lookup_tables != self.address_lookup_tables {
            return true;
        }

        false
    }

    pub async fn new_tokens_listed(&self, rpc: &RpcClientAsync) -> anyhow::Result<bool> {
        let mint_infos = fetch_mint_infos(rpc, mango_v4::id(), self.group).await?;
        Ok(mint_infos.len() > self.tokens.len())
    }

    pub async fn new_serum3_markets_listed(&self, rpc: &RpcClientAsync) -> anyhow::Result<bool> {
        let serum3_markets = fetch_serum3_markets(rpc, mango_v4::id(), self.group).await?;
        Ok(serum3_markets.len() > self.serum3_markets.len())
    }

    pub async fn new_perp_markets_listed(&self, rpc: &RpcClientAsync) -> anyhow::Result<bool> {
        let new_perp_markets = fetch_perp_markets(rpc, mango_v4::id(), self.group).await?;
        Ok(new_perp_markets.len() > self.perp_markets.len())
    }

    /// Returns a map of oracle pubkey -> FallbackOracleContext
    pub async fn derive_fallback_oracle_keys(
        &self,
        fallback_oracle_config: &FallbackOracleConfig,
        account_fetcher: &dyn AccountFetcher,
    ) -> anyhow::Result<HashMap<Pubkey, FallbackOracleContext>> {
        // FUTURE: implement for perp oracles as well
        let fallbacks_by_oracle = match fallback_oracle_config {
            FallbackOracleConfig::Never => HashMap::new(),
            FallbackOracleConfig::Fixed(keys) => self
                .tokens
                .iter()
                .filter(|token| {
                    token.1.fallback_context.key != Pubkey::default()
                        && keys.contains(&token.1.fallback_context.key)
                })
                .map(|t| (t.1.oracle, t.1.fallback_context.clone()))
                .collect(),
            FallbackOracleConfig::All => self
                .tokens
                .iter()
                .filter(|token| token.1.fallback_context.key != Pubkey::default())
                .map(|t| (t.1.oracle, t.1.fallback_context.clone()))
                .collect(),
            FallbackOracleConfig::Dynamic => {
                let tokens_by_oracle: HashMap<Pubkey, &TokenContext> =
                    self.tokens.iter().map(|t| (t.1.oracle, t.1)).collect();
                let oracle_keys: Vec<Pubkey> =
                    tokens_by_oracle.values().map(|b| b.oracle).collect();
                let oracle_accounts = account_fetcher
                    .fetch_multiple_accounts(&oracle_keys)
                    .await?;
                let now_slot = account_fetcher.get_slot().await?;

                let mut stale_oracles_with_fallbacks = vec![];
                for (key, acc) in oracle_accounts {
                    let token = tokens_by_oracle.get(&key).unwrap();
                    let state = oracle_state_unchecked(
                        &OracleAccountInfos::from_reader(&KeyedAccountSharedData::new(key, acc)),
                        token.decimals,
                    )?;
                    let oracle_is_valid = state
                        .check_confidence_and_maybe_staleness(&token.oracle_config, Some(now_slot));
                    if oracle_is_valid.is_err() && token.fallback_context.key != Pubkey::default() {
                        stale_oracles_with_fallbacks
                            .push((token.oracle, token.fallback_context.clone()));
                    }
                }
                stale_oracles_with_fallbacks.into_iter().collect()
            }
        };

        Ok(fallbacks_by_oracle)
    }
}

fn from_serum_style_pubkey(d: [u64; 4]) -> Pubkey {
    let b: [u8; 32] = bytemuck::cast(d);
    Pubkey::from(b)
}

async fn fetch_raw_account(rpc: &RpcClientAsync, address: Pubkey) -> Result<Account, ClientError> {
    rpc.get_account_with_commitment(&address, rpc.commitment())
        .await?
        .value
        .ok_or(ClientError::AccountNotFound)
}

/// Fetch the quote key for a fallback oracle account info.
/// Returns Pubkey::default if no quote key is found or there are any
/// errors occur when trying to fetch the quote oracle.
/// This function will only return a non-default key when a CLMM oracle is used
fn get_fallback_quote_key(acc_info: &impl KeyedAccountReader) -> Pubkey {
    let maybe_key = match determine_oracle_type(acc_info).ok() {
        Some(oracle_type) => match oracle_type {
            OracleType::OrcaCLMM => match load_whirlpool_state(acc_info).ok() {
                Some(whirlpool) => whirlpool.get_quote_oracle().ok(),
                None => None,
            },
            _ => None,
        },
        None => None,
    };

    maybe_key.unwrap_or_else(|| Pubkey::default())
}
