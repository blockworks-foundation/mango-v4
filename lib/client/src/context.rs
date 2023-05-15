use std::collections::HashMap;

use anchor_client::ClientError;

use anchor_lang::__private::bytemuck;

use mango_v4::state::{
    Group, MangoAccountValue, MintInfo, PerpMarket, PerpMarketIndex, Serum3Market,
    Serum3MarketIndex, TokenIndex,
};

use fixed::types::I80F48;
use futures::{stream, StreamExt, TryStreamExt};
use itertools::Itertools;

use crate::gpa::*;

use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_sdk::account::Account;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;

#[derive(Clone)]
pub struct TokenContext {
    pub token_index: TokenIndex,
    pub name: String,
    pub mint_info: MintInfo,
    pub mint_info_address: Pubkey,
    pub decimals: u8,
}

impl TokenContext {
    pub fn native_to_ui(&self, native: I80F48) -> f64 {
        (native / I80F48::from(10u64.pow(self.decimals.into()))).to_num()
    }
}

pub struct Serum3MarketContext {
    pub address: Pubkey,
    pub market: Serum3Market,
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

pub struct PerpMarketContext {
    pub address: Pubkey,
    pub market: PerpMarket,
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
}

impl MangoGroupContext {
    pub fn mint_info_address(&self, token_index: TokenIndex) -> Pubkey {
        self.token(token_index).mint_info_address
    }

    pub fn mint_info(&self, token_index: TokenIndex) -> MintInfo {
        self.token(token_index).mint_info
    }

    pub fn token(&self, token_index: TokenIndex) -> &TokenContext {
        self.tokens.get(&token_index).unwrap()
    }

    pub fn perp(&self, perp_market_index: PerpMarketIndex) -> &PerpMarketContext {
        self.perp_markets.get(&perp_market_index).unwrap()
    }

    pub fn token_by_mint(&self, mint: &Pubkey) -> anyhow::Result<&TokenContext> {
        self.tokens
            .iter()
            .find_map(|(_, tc)| (tc.mint_info.mint == *mint).then(|| tc))
            .ok_or_else(|| anyhow::anyhow!("no token for mint {}", mint))
    }

    pub fn perp_market_address(&self, perp_market_index: PerpMarketIndex) -> Pubkey {
        self.perp(perp_market_index).address
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
                        mint_info: *mi,
                        mint_info_address: *pk,
                        decimals: u8::MAX,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        // reading the banks is only needed for the token names and decimals
        // FUTURE: either store the names on MintInfo as well, or maybe don't store them at all
        //         because they are in metaplex?
        let bank_tuples = fetch_banks(rpc, program, group).await?;
        for (_, bank) in bank_tuples {
            let token = tokens.get_mut(&bank.token_index).unwrap();
            token.name = bank.name().into();
            token.decimals = bank.mint_decimals;
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
                        market: *s,
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
                        market: *pm,
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
            .map(|(i, s)| (s.market.name().to_string(), *i))
            .collect::<HashMap<_, _>>();
        let perp_market_indexes_by_name = perp_markets
            .iter()
            .map(|(i, p)| (p.market.name().to_string(), *i))
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
        })
    }

    pub fn derive_health_check_remaining_account_metas(
        &self,
        account: &MangoAccountValue,
        affected_tokens: Vec<TokenIndex>,
        writable_banks: Vec<TokenIndex>,
        affected_perp_markets: Vec<PerpMarketIndex>,
    ) -> anyhow::Result<Vec<AccountMeta>> {
        let mut account = account.clone();
        for affected_token_index in affected_tokens {
            account.ensure_token_position(affected_token_index)?;
        }
        for affected_perp_market_index in affected_perp_markets {
            let settle_token_index = self
                .perp(affected_perp_market_index)
                .market
                .settle_token_index;
            account.ensure_perp_position(affected_perp_market_index, settle_token_index)?;
        }

        // figure out all the banks/oracles that need to be passed for the health check
        let mut banks = vec![];
        let mut oracles = vec![];
        for position in account.active_token_positions() {
            let mint_info = self.mint_info(position.token_index);
            banks.push((
                mint_info.first_bank(),
                writable_banks.iter().any(|&ti| ti == position.token_index),
            ));
            oracles.push(mint_info.oracle);
        }

        let serum_oos = account.active_serum3_orders().map(|&s| s.open_orders);
        let perp_markets = account
            .active_perp_positions()
            .map(|&pa| self.perp_market_address(pa.market_index));
        let perp_oracles = account
            .active_perp_positions()
            .map(|&pa| self.perp(pa.market_index).market.oracle);

        let to_account_meta = |pubkey| AccountMeta {
            pubkey,
            is_writable: false,
            is_signer: false,
        };

        Ok(banks
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
            .collect())
    }

    pub fn derive_health_check_remaining_account_metas_two_accounts(
        &self,
        account1: &MangoAccountValue,
        account2: &MangoAccountValue,
        affected_tokens: &[TokenIndex],
        writable_banks: &[TokenIndex],
    ) -> anyhow::Result<Vec<AccountMeta>> {
        // figure out all the banks/oracles that need to be passed for the health check
        let mut banks = vec![];
        let mut oracles = vec![];

        let token_indexes = account2
            .active_token_positions()
            .chain(account1.active_token_positions())
            .map(|ta| ta.token_index)
            .chain(affected_tokens.iter().copied())
            .unique();

        for token_index in token_indexes {
            let mint_info = self.mint_info(token_index);
            let writable_bank = writable_banks.iter().contains(&token_index);
            banks.push((mint_info.first_bank(), writable_bank));
            oracles.push(mint_info.oracle);
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
            .map(|&index| self.perp(index).market.oracle);

        let to_account_meta = |pubkey| AccountMeta {
            pubkey,
            is_writable: false,
            is_signer: false,
        };

        Ok(banks
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
            .collect())
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
