use std::collections::HashMap;

use anchor_client::{Client, ClientError, Cluster, Program};

use anchor_lang::__private::bytemuck;

use mango_v4::state::{
    MangoAccountValue, MintInfo, PerpMarket, PerpMarketIndex, Serum3Market, Serum3MarketIndex,
    TokenIndex,
};

use fixed::types::I80F48;

use crate::gpa::*;

use solana_sdk::account::Account;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::signature::Keypair;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};

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

    pub fn token_by_mint(&self, mint: &Pubkey) -> anyhow::Result<&TokenContext> {
        self.tokens
            .iter()
            .find_map(|(_, tc)| (tc.mint_info.mint == *mint).then(|| tc))
            .ok_or_else(|| anyhow::anyhow!("no token for mint {}", mint))
    }

    pub fn perp_market_address(&self, perp_market_index: PerpMarketIndex) -> Pubkey {
        self.perp_markets.get(&perp_market_index).unwrap().address
    }

    pub fn new_from_rpc(
        group: Pubkey,
        cluster: Cluster,
        commitment: CommitmentConfig,
    ) -> Result<Self, ClientError> {
        let program =
            Client::new_with_options(cluster, std::rc::Rc::new(Keypair::new()), commitment)
                .program(mango_v4::ID);

        // tokens
        let mint_info_tuples = fetch_mint_infos(&program, group)?;
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
        let bank_tuples = fetch_banks(&program, group)?;
        for (_, bank) in bank_tuples {
            let token = tokens.get_mut(&bank.token_index).unwrap();
            token.name = bank.name().into();
            token.decimals = bank.mint_decimals;
        }
        assert!(tokens.values().all(|t| t.decimals != u8::MAX));

        // serum3 markets
        let serum3_market_tuples = fetch_serum3_markets(&program, group)?;
        let serum3_markets = serum3_market_tuples
            .iter()
            .map(|(pk, s)| {
                let market_external_account = fetch_raw_account(&program, s.serum_market_external)?;
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
                Ok((
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
                ))
            })
            .collect::<Result<HashMap<_, _>, ClientError>>()?;

        // perp markets
        let perp_market_tuples = fetch_perp_markets(&program, group)?;
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

        Ok(MangoGroupContext {
            group,
            tokens,
            token_indexes_by_name,
            serum3_markets,
            serum3_market_indexes_by_name,
            perp_markets,
            perp_market_indexes_by_name,
        })
    }

    pub fn derive_health_check_remaining_account_metas(
        &self,
        account: &MangoAccountValue,
        affected_tokens: Vec<TokenIndex>,
        writable_banks: bool,
    ) -> anyhow::Result<Vec<AccountMeta>> {
        // figure out all the banks/oracles that need to be passed for the health check
        let mut banks = vec![];
        let mut oracles = vec![];
        for position in account.active_token_positions() {
            let mint_info = self.mint_info(position.token_index);
            banks.push(mint_info.first_bank());
            oracles.push(mint_info.oracle);
        }
        for affected_token_index in affected_tokens {
            if !account
                .active_token_positions()
                .any(|p| p.token_index == affected_token_index)
            {
                // If there is not yet an active position for the token, we need to pass
                // the bank/oracle for health check anyway.
                let new_position = account
                    .all_token_positions()
                    .position(|p| !p.is_active())
                    .unwrap();
                let mint_info = self.mint_info(affected_token_index);
                banks.insert(new_position, mint_info.first_bank());
                oracles.insert(new_position, mint_info.oracle);
            }
        }

        let serum_oos = account.active_serum3_orders().map(|&s| s.open_orders);
        let perp_markets = account
            .active_perp_positions()
            .map(|&pa| self.perp_market_address(pa.market_index));

        Ok(banks
            .iter()
            .map(|&pubkey| AccountMeta {
                pubkey,
                is_writable: writable_banks,
                is_signer: false,
            })
            .chain(oracles.iter().map(|&pubkey| AccountMeta {
                pubkey,
                is_writable: false,
                is_signer: false,
            }))
            .chain(perp_markets.map(|pubkey| AccountMeta {
                pubkey,
                is_writable: false,
                is_signer: false,
            }))
            .chain(serum_oos.map(|pubkey| AccountMeta {
                pubkey,
                is_writable: false,
                is_signer: false,
            }))
            .collect())
    }
}

fn from_serum_style_pubkey(d: [u64; 4]) -> Pubkey {
    Pubkey::new(bytemuck::cast_slice(&d as &[_]))
}

fn fetch_raw_account(program: &Program, address: Pubkey) -> Result<Account, ClientError> {
    let rpc = program.rpc();
    rpc.get_account_with_commitment(&address, rpc.commitment())?
        .value
        .ok_or(ClientError::AccountNotFound)
}
