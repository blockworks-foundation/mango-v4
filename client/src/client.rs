use std::collections::HashMap;

use anchor_client::{Client, Cluster, Program};

use anchor_lang::__private::bytemuck;
use anchor_lang::prelude::System;
use anchor_lang::{AccountDeserialize, Id};
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::{Mint, Token};

use fixed::types::I80F48;
use itertools::Itertools;
use mango_v4::instructions::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};
use mango_v4::state::{
    Bank, Group, MangoAccount, MintInfo, PerpMarket, PerpMarketIndex, Serum3Market, TokenIndex,
};

use solana_client::rpc_client::RpcClient;

use crate::util::MyClone;
use anyhow::Context;
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::sysvar;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signer::Signer};

// todo: might want to integrate geyser, websockets, or simple http polling for keeping data fresh
pub struct MangoClient {
    pub rpc: RpcClient,
    pub cluster: Cluster,
    pub commitment: CommitmentConfig,
    pub payer: Keypair,
    pub mango_account_cache: (Pubkey, MangoAccount),
    pub group: Pubkey,
    pub group_cache: Group,
    // TODO: future: this may not scale if there's thousands of mints, probably some function
    // wrapping getMultipleAccounts is needed (or bettew: we provide this data as a service)
    pub banks_cache: HashMap<String, Vec<(Pubkey, Bank)>>,
    pub banks_cache_by_token_index: HashMap<TokenIndex, Vec<(Pubkey, Bank)>>,
    pub mint_infos_cache: HashMap<Pubkey, (Pubkey, MintInfo, Mint)>,
    pub mint_infos_cache_by_token_index: HashMap<TokenIndex, (Pubkey, MintInfo, Mint)>,
    pub serum3_markets_cache: HashMap<String, (Pubkey, Serum3Market)>,
    pub serum3_external_markets_cache: HashMap<String, (Pubkey, Vec<u8>)>,
    pub perp_markets_cache: HashMap<String, (Pubkey, PerpMarket)>,
    pub perp_markets_cache_by_perp_market_index: HashMap<PerpMarketIndex, (Pubkey, PerpMarket)>,
}

// TODO: add retry framework for sending tx and rpc calls
// 1/ this works right now, but I think mid-term the MangoClient will want to interact with multiple mango accounts
// -- then we should probably specify accounts by owner+account_num / or pubkey
// 2/ pubkey, can be both owned, but also delegated accouns

impl MangoClient {
    pub fn group_for_admin(admin: Pubkey, num: u32) -> Pubkey {
        Pubkey::find_program_address(
            &["Group".as_ref(), admin.as_ref(), num.to_le_bytes().as_ref()],
            &mango_v4::ID,
        )
        .0
    }

    pub fn new(
        cluster: Cluster,
        commitment: CommitmentConfig,
        payer: Keypair,
        group: Pubkey,
        mango_account_name: &str,
    ) -> anyhow::Result<Self> {
        let program =
            Client::new_with_options(cluster.clone(), std::rc::Rc::new(payer.clone()), commitment)
                .program(mango_v4::ID);

        let rpc = program.rpc();

        let group_data = program.account::<Group>(group)?;

        // Mango Account
        let mut mango_account_tuples = program.accounts::<MangoAccount>(vec![
            RpcFilterType::Memcmp(Memcmp {
                offset: 8,
                bytes: MemcmpEncodedBytes::Base58(group.to_string()),
                encoding: None,
            }),
            RpcFilterType::Memcmp(Memcmp {
                offset: 40,
                bytes: MemcmpEncodedBytes::Base58(payer.pubkey().to_string()),
                encoding: None,
            }),
        ])?;
        let mango_account_opt = mango_account_tuples
            .iter()
            .find(|tuple| tuple.1.name() == mango_account_name);
        if mango_account_opt.is_none() {
            mango_account_tuples
                .sort_by(|a, b| a.1.account_num.partial_cmp(&b.1.account_num).unwrap());
            let account_num = match mango_account_tuples.last() {
                Some(tuple) => tuple.1.account_num + 1,
                None => 0u8,
            };
            program
                .request()
                .instruction(Instruction {
                    program_id: mango_v4::id(),
                    accounts: anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::AccountCreate {
                            group,
                            owner: payer.pubkey(),
                            account: {
                                Pubkey::find_program_address(
                                    &[
                                        group.as_ref(),
                                        b"MangoAccount".as_ref(),
                                        payer.pubkey().as_ref(),
                                        &account_num.to_le_bytes(),
                                    ],
                                    &mango_v4::id(),
                                )
                                .0
                            },
                            payer: payer.pubkey(),
                            system_program: System::id(),
                        },
                        None,
                    ),
                    data: anchor_lang::InstructionData::data(
                        &mango_v4::instruction::AccountCreate {
                            account_num,
                            name: mango_account_name.to_owned(),
                        },
                    ),
                })
                .send()
                .context("Failed to create account...")?;
        }
        let mango_account_tuples = program.accounts::<MangoAccount>(vec![
            RpcFilterType::Memcmp(Memcmp {
                offset: 8,
                bytes: MemcmpEncodedBytes::Base58(group.to_string()),
                encoding: None,
            }),
            RpcFilterType::Memcmp(Memcmp {
                offset: 40,
                bytes: MemcmpEncodedBytes::Base58(payer.pubkey().to_string()),
                encoding: None,
            }),
        ])?;
        let index = mango_account_tuples
            .iter()
            .position(|tuple| tuple.1.name() == mango_account_name)
            .unwrap();
        let mango_account_cache = mango_account_tuples[index];

        // banks cache
        let mut banks_cache = HashMap::new();
        let mut banks_cache_by_token_index = HashMap::new();
        let bank_tuples = program.accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp {
            offset: 8,
            bytes: MemcmpEncodedBytes::Base58(group.to_string()),
            encoding: None,
        })])?;
        for (k, v) in bank_tuples {
            banks_cache
                .entry(v.name().to_owned())
                .or_insert_with(|| Vec::new())
                .push((k, v));
            banks_cache_by_token_index
                .entry(v.token_index)
                .or_insert_with(|| Vec::new())
                .push((k, v));
        }

        // mintinfo cache
        let mut mint_infos_cache = HashMap::new();
        let mut mint_infos_cache_by_token_index = HashMap::new();
        let mint_info_tuples =
            program.accounts::<MintInfo>(vec![RpcFilterType::Memcmp(Memcmp {
                offset: 8,
                bytes: MemcmpEncodedBytes::Base58(group.to_string()),
                encoding: None,
            })])?;
        for (k, v) in mint_info_tuples {
            let data = program
                .rpc()
                .get_account_with_commitment(&v.mint, commitment)?
                .value
                .unwrap()
                .data;
            let mint = Mint::try_deserialize(&mut &data[..])?;

            mint_infos_cache.insert(v.mint, (k, v, mint.clone()));
            mint_infos_cache_by_token_index.insert(v.token_index, (k, v, mint));
        }

        // serum3 markets cache
        let mut serum3_markets_cache = HashMap::new();
        let mut serum3_external_markets_cache = HashMap::new();
        let serum3_market_tuples =
            program.accounts::<Serum3Market>(vec![RpcFilterType::Memcmp(Memcmp {
                offset: 8,
                bytes: MemcmpEncodedBytes::Base58(group.to_string()),
                encoding: None,
            })])?;
        for (k, v) in serum3_market_tuples {
            serum3_markets_cache.insert(v.name().to_owned(), (k, v));

            let market_external_bytes = program
                .rpc()
                .get_account_with_commitment(&v.serum_market_external, commitment)?
                .value
                .unwrap()
                .data;
            serum3_external_markets_cache.insert(
                v.name().to_owned(),
                (v.serum_market_external, market_external_bytes),
            );
        }

        // perp markets cache
        let mut perp_markets_cache = HashMap::new();
        let mut perp_markets_cache_by_perp_market_index = HashMap::new();
        let perp_market_tuples =
            program.accounts::<PerpMarket>(vec![RpcFilterType::Memcmp(Memcmp {
                offset: 8,
                bytes: MemcmpEncodedBytes::Base58(group.to_string()),
                encoding: None,
            })])?;
        for (k, v) in perp_market_tuples {
            perp_markets_cache.insert(v.name().to_owned(), (k, v));
            perp_markets_cache_by_perp_market_index.insert(v.perp_market_index, (k, v));
        }

        Ok(Self {
            rpc,
            cluster,
            commitment,
            payer,
            mango_account_cache,
            group,
            group_cache: group_data,
            banks_cache,
            banks_cache_by_token_index,
            mint_infos_cache,
            mint_infos_cache_by_token_index,
            serum3_markets_cache,
            serum3_external_markets_cache,
            perp_markets_cache,
            perp_markets_cache_by_perp_market_index,
        })
    }

    pub fn get_mint_info(&self, token_index: &TokenIndex) -> Pubkey {
        self.mint_infos_cache_by_token_index
            .get(token_index)
            .unwrap()
            .0
    }

    pub fn client(&self) -> Client {
        Client::new_with_options(
            self.cluster.clone(),
            std::rc::Rc::new(self.payer.clone()),
            self.commitment,
        )
    }

    pub fn program(&self) -> Program {
        self.client().program(mango_v4::ID)
    }

    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }

    pub fn group(&self) -> Pubkey {
        self.group
    }

    pub fn get_account(&self) -> Result<(Pubkey, MangoAccount), anchor_client::ClientError> {
        let mango_accounts = self.program().accounts::<MangoAccount>(vec![
            RpcFilterType::Memcmp(Memcmp {
                offset: 8,
                bytes: MemcmpEncodedBytes::Base58(self.group().to_string()),
                encoding: None,
            }),
            RpcFilterType::Memcmp(Memcmp {
                offset: 40,
                bytes: MemcmpEncodedBytes::Base58(self.payer().to_string()),
                encoding: None,
            }),
        ])?;
        Ok(mango_accounts[0])
    }

    pub fn derive_health_check_remaining_account_metas(
        &self,
        affected_bank: Option<(Pubkey, Bank)>,
        writable_banks: bool,
    ) -> Result<Vec<AccountMeta>, anchor_client::ClientError> {
        // figure out all the banks/oracles that need to be passed for the health check
        let mut banks = vec![];
        let mut oracles = vec![];
        let account = self.get_account()?;
        for position in account.1.tokens.iter_active() {
            let mint_info = self
                .mint_infos_cache_by_token_index
                .get(&position.token_index)
                .unwrap()
                .1;
            // TODO: ALTs are unavailable
            // let lookup_table = account_loader
            //     .load_bytes(&mint_info.address_lookup_table)
            //     .await
            //     .unwrap();
            // let addresses = mango_v4::address_lookup_table::addresses(&lookup_table);
            // banks.push(addresses[mint_info.address_lookup_table_bank_index as usize]);
            // oracles.push(addresses[mint_info.address_lookup_table_oracle_index as usize]);
            banks.push(mint_info.first_bank());
            oracles.push(mint_info.oracle);
        }
        if let Some(affected_bank) = affected_bank {
            if !banks.iter().any(|&v| v == affected_bank.0) {
                // If there is not yet an active position for the token, we need to pass
                // the bank/oracle for health check anyway.
                let new_position = account
                    .1
                    .tokens
                    .values
                    .iter()
                    .position(|p| !p.is_active())
                    .unwrap();
                banks.insert(new_position, affected_bank.0);
                oracles.insert(new_position, affected_bank.1.oracle);
            }
        }

        let serum_oos = account.1.serum3.iter_active().map(|&s| s.open_orders);
        let perp_markets = account.1.perps.iter_active_accounts().map(|&pa| {
            self.perp_markets_cache_by_perp_market_index
                .get(&pa.market_index)
                .unwrap()
                .0
        });

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
            .chain(serum_oos.map(|pubkey| AccountMeta {
                pubkey,
                is_writable: false,
                is_signer: false,
            }))
            .chain(perp_markets.map(|pubkey| AccountMeta {
                pubkey,
                is_writable: false,
                is_signer: false,
            }))
            .collect())
    }

    pub fn derive_liquidation_health_check_remaining_account_metas(
        &self,
        liqee: &MangoAccount,
        asset_token_index: TokenIndex,
        liab_token_index: TokenIndex,
    ) -> Result<Vec<AccountMeta>, anchor_client::ClientError> {
        // figure out all the banks/oracles that need to be passed for the health check
        let mut banks = vec![];
        let mut oracles = vec![];
        let account = self.get_account()?;

        let token_indexes = liqee
            .tokens
            .iter_active()
            .chain(account.1.tokens.iter_active())
            .map(|ta| ta.token_index)
            .unique();

        for token_index in token_indexes {
            let mint_info = self
                .mint_infos_cache_by_token_index
                .get(&token_index)
                .unwrap()
                .1;
            let writable_bank = token_index == asset_token_index || token_index == liab_token_index;
            // TODO: ALTs are unavailable
            // let lookup_table = account_loader
            //     .load_bytes(&mint_info.address_lookup_table)
            //     .await
            //     .unwrap();
            // let addresses = mango_v4::address_lookup_table::addresses(&lookup_table);
            // banks.push(addresses[mint_info.address_lookup_table_bank_index as usize]);
            // oracles.push(addresses[mint_info.address_lookup_table_oracle_index as usize]);
            banks.push((mint_info.first_bank(), writable_bank));
            oracles.push(mint_info.oracle);
        }

        let serum_oos = liqee
            .serum3
            .iter_active()
            .chain(account.1.serum3.iter_active())
            .map(|&s| s.open_orders);
        let perp_markets = liqee
            .perps
            .iter_active_accounts()
            .chain(account.1.perps.iter_active_accounts())
            .map(|&pa| {
                self.perp_markets_cache_by_perp_market_index
                    .get(&pa.market_index)
                    .unwrap()
                    .0
            });

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
            .chain(serum_oos.map(to_account_meta))
            .chain(perp_markets.map(to_account_meta))
            .collect())
    }

    pub fn token_deposit(&self, token_name: &str, amount: u64) -> anyhow::Result<Signature> {
        let bank = self.banks_cache.get(token_name).unwrap().get(0).unwrap();
        let mint_info: MintInfo = self.mint_infos_cache.get(&bank.1.mint).unwrap().1;

        let health_check_metas =
            self.derive_health_check_remaining_account_metas(Some(*bank), false)?;

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: {
                    let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::TokenDeposit {
                            group: self.group(),
                            account: self.mango_account_cache.0,
                            bank: bank.0,
                            vault: bank.1.vault,
                            token_account: get_associated_token_address(
                                &self.payer(),
                                &mint_info.mint,
                            ),
                            token_authority: self.payer(),
                            token_program: Token::id(),
                        },
                        None,
                    );
                    ams.extend(health_check_metas.into_iter());
                    ams
                },
                data: anchor_lang::InstructionData::data(&mango_v4::instruction::TokenDeposit {
                    amount,
                }),
            })
            .send()
            .map_err(prettify_client_error)
    }

    pub fn get_oracle_price(
        &self,
        token_name: &str,
    ) -> Result<pyth_sdk_solana::Price, anyhow::Error> {
        let bank = self.banks_cache.get(token_name).unwrap().get(0).unwrap().1;

        let data = self
            .program()
            .rpc()
            .get_account_with_commitment(&bank.oracle, self.commitment)?
            .value
            .unwrap()
            .data;

        Ok(pyth_sdk_solana::load_price(&data).unwrap())
    }

    //
    // Serum3
    //

    pub fn serum3_create_open_orders(&self, name: &str) -> anyhow::Result<Signature> {
        let (account_pubkey, _) = self.mango_account_cache;

        let serum3_market = self.serum3_markets_cache.get(name).unwrap();

        let open_orders = Pubkey::find_program_address(
            &[
                account_pubkey.as_ref(),
                b"Serum3OO".as_ref(),
                serum3_market.0.as_ref(),
            ],
            &self.program().id(),
        )
        .0;

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::Serum3CreateOpenOrders {
                        group: self.group(),
                        account: account_pubkey,

                        serum_market: serum3_market.0,
                        serum_program: serum3_market.1.serum_program,
                        serum_market_external: serum3_market.1.serum_market_external,
                        open_orders,
                        owner: self.payer(),
                        payer: self.payer(),
                        system_program: System::id(),
                        rent: sysvar::rent::id(),
                    },
                    None,
                ),
                data: anchor_lang::InstructionData::data(
                    &mango_v4::instruction::Serum3CreateOpenOrders {},
                ),
            })
            .send()
            .map_err(prettify_client_error)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn serum3_place_order(
        &self,
        name: &str,
        side: Serum3Side,
        price: f64,
        size: f64,
        self_trade_behavior: Serum3SelfTradeBehavior,
        order_type: Serum3OrderType,
        client_order_id: u64,
        limit: u16,
    ) -> anyhow::Result<Signature> {
        let (_, account) = self.get_account()?;

        let serum3_market = self.serum3_markets_cache.get(name).unwrap();
        let open_orders = account
            .serum3
            .find(serum3_market.1.market_index)
            .unwrap()
            .open_orders;
        let (_, quote_info, quote_mint) = self
            .mint_infos_cache_by_token_index
            .get(&serum3_market.1.quote_token_index)
            .unwrap();
        let (_, base_info, base_mint) = self
            .mint_infos_cache_by_token_index
            .get(&serum3_market.1.base_token_index)
            .unwrap();

        let market_external: &serum_dex::state::MarketState = bytemuck::from_bytes(
            &(self.serum3_external_markets_cache.get(name).unwrap().1)
                [5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
        );
        let bids = market_external.bids;
        let asks = market_external.asks;
        let event_q = market_external.event_q;
        let req_q = market_external.req_q;
        let coin_vault = market_external.coin_vault;
        let pc_vault = market_external.pc_vault;
        let vault_signer = serum_dex::state::gen_vault_signer_key(
            market_external.vault_signer_nonce,
            &serum3_market.1.serum_market_external,
            &serum3_market.1.serum_program,
        )
        .unwrap();

        let health_check_metas = self.derive_health_check_remaining_account_metas(None, false)?;

        // https://github.com/project-serum/serum-ts/blob/master/packages/serum/src/market.ts#L1306
        let limit_price = {
            (price
                * ((10u64.pow(quote_mint.decimals as u32) * market_external.coin_lot_size) as f64))
                as u64
                / (10u64.pow(base_mint.decimals as u32) * market_external.pc_lot_size)
        };
        // https://github.com/project-serum/serum-ts/blob/master/packages/serum/src/market.ts#L1333
        let max_base_qty = {
            (size * 10u64.pow(base_mint.decimals as u32) as f64) as u64
                / market_external.coin_lot_size
        };
        let max_native_quote_qty_including_fees = {
            fn get_fee_tier(msrm_balance: u64, srm_balance: u64) -> u64 {
                if msrm_balance >= 1 {
                    6
                } else if srm_balance >= 1_000_000 {
                    5
                } else if srm_balance >= 100_000 {
                    4
                } else if srm_balance >= 10_000 {
                    3
                } else if srm_balance >= 1_000 {
                    2
                } else if srm_balance >= 100 {
                    1
                } else {
                    0
                }
            }

            fn get_fee_rates(fee_tier: u64) -> (f64, f64) {
                if fee_tier == 1 {
                    // SRM2
                    return (0.002, -0.0003);
                } else if fee_tier == 2 {
                    // SRM3
                    return (0.0018, -0.0003);
                } else if fee_tier == 3 {
                    // SRM4
                    return (0.0016, -0.0003);
                } else if fee_tier == 4 {
                    // SRM5
                    return (0.0014, -0.0003);
                } else if fee_tier == 5 {
                    // SRM6
                    return (0.0012, -0.0003);
                } else if fee_tier == 6 {
                    // MSRM
                    return (0.001, -0.0005);
                }
                // Base
                (0.0022, -0.0003)
            }

            let fee_tier = get_fee_tier(0, 0);
            let rates = get_fee_rates(fee_tier);
            (market_external.pc_lot_size as f64 * (1f64 + rates.0)) as u64
                * (limit_price * max_base_qty)
        };

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: {
                    let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::Serum3PlaceOrder {
                            group: self.group(),
                            account: self.mango_account_cache.0,
                            open_orders,
                            quote_bank: quote_info.first_bank(),
                            quote_vault: quote_info.first_vault(),
                            base_bank: base_info.first_bank(),
                            base_vault: base_info.first_vault(),
                            serum_market: serum3_market.0,
                            serum_program: serum3_market.1.serum_program,
                            serum_market_external: serum3_market.1.serum_market_external,
                            market_bids: from_serum_style_pubkey(&bids),
                            market_asks: from_serum_style_pubkey(&asks),
                            market_event_queue: from_serum_style_pubkey(&event_q),
                            market_request_queue: from_serum_style_pubkey(&req_q),
                            market_base_vault: from_serum_style_pubkey(&coin_vault),
                            market_quote_vault: from_serum_style_pubkey(&pc_vault),
                            market_vault_signer: vault_signer,
                            owner: self.payer(),
                            token_program: Token::id(),
                        },
                        None,
                    );
                    ams.extend(health_check_metas.into_iter());
                    ams
                },
                data: anchor_lang::InstructionData::data(
                    &mango_v4::instruction::Serum3PlaceOrder {
                        side,
                        limit_price,
                        max_base_qty,
                        max_native_quote_qty_including_fees,
                        self_trade_behavior,
                        order_type,
                        client_order_id,
                        limit,
                    },
                ),
            })
            .send()
            .map_err(prettify_client_error)
    }

    pub fn serum3_settle_funds(&self, name: &str) -> anyhow::Result<Signature> {
        let (_, account) = self.get_account()?;

        let serum3_market = self.serum3_markets_cache.get(name).unwrap();
        let open_orders = account
            .serum3
            .find(serum3_market.1.market_index)
            .unwrap()
            .open_orders;
        let (_, quote_info, _) = self
            .mint_infos_cache_by_token_index
            .get(&serum3_market.1.quote_token_index)
            .unwrap();
        let (_, base_info, _) = self
            .mint_infos_cache_by_token_index
            .get(&serum3_market.1.base_token_index)
            .unwrap();

        let market_external: &serum_dex::state::MarketState = bytemuck::from_bytes(
            &(self.serum3_external_markets_cache.get(name).unwrap().1)
                [5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
        );
        let coin_vault = market_external.coin_vault;
        let pc_vault = market_external.pc_vault;
        let vault_signer = serum_dex::state::gen_vault_signer_key(
            market_external.vault_signer_nonce,
            &serum3_market.1.serum_market_external,
            &serum3_market.1.serum_program,
        )
        .unwrap();

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::Serum3SettleFunds {
                        group: self.group(),
                        account: self.mango_account_cache.0,
                        open_orders,
                        quote_bank: quote_info.first_bank(),
                        quote_vault: quote_info.first_vault(),
                        base_bank: base_info.first_bank(),
                        base_vault: base_info.first_vault(),
                        serum_market: serum3_market.0,
                        serum_program: serum3_market.1.serum_program,
                        serum_market_external: serum3_market.1.serum_market_external,
                        market_base_vault: from_serum_style_pubkey(&coin_vault),
                        market_quote_vault: from_serum_style_pubkey(&pc_vault),
                        market_vault_signer: vault_signer,
                        owner: self.payer(),
                        token_program: Token::id(),
                    },
                    None,
                ),
                data: anchor_lang::InstructionData::data(
                    &mango_v4::instruction::Serum3SettleFunds {},
                ),
            })
            .send()
            .map_err(prettify_client_error)
    }

    pub fn serum3_cancel_all_orders(&self, market_name: &str) -> Result<Vec<u128>, anyhow::Error> {
        let serum3_market = self.serum3_markets_cache.get(market_name).unwrap();

        let open_orders = Pubkey::find_program_address(
            &[
                self.mango_account_cache.0.as_ref(),
                b"Serum3OO".as_ref(),
                serum3_market.0.as_ref(),
            ],
            &self.program().id(),
        )
        .0;

        let open_orders_bytes = self
            .program()
            .rpc()
            .get_account_with_commitment(&open_orders, self.commitment)?
            .value
            .unwrap()
            .data;
        let open_orders_data: &serum_dex::state::OpenOrders = bytemuck::from_bytes(
            &open_orders_bytes[5..5 + std::mem::size_of::<serum_dex::state::OpenOrders>()],
        );

        let mut orders = vec![];
        for order_id in open_orders_data.orders {
            if order_id != 0 {
                // TODO: find side for order_id, and only cancel the relevant order
                self.serum3_cancel_order(market_name, Serum3Side::Bid, order_id)
                    .ok();
                self.serum3_cancel_order(market_name, Serum3Side::Ask, order_id)
                    .ok();

                orders.push(order_id);
            }
        }

        Ok(orders)
    }

    pub fn serum3_cancel_order(
        &self,
        market_name: &str,
        side: Serum3Side,
        order_id: u128,
    ) -> anyhow::Result<()> {
        let (account_pubkey, _account) = self.get_account()?;

        let serum3_market = self.serum3_markets_cache.get(market_name).unwrap();

        let open_orders = Pubkey::find_program_address(
            &[
                account_pubkey.as_ref(),
                b"Serum3OO".as_ref(),
                serum3_market.0.as_ref(),
            ],
            &self.program().id(),
        )
        .0;

        let market_external: &serum_dex::state::MarketState = bytemuck::from_bytes(
            &(self
                .serum3_external_markets_cache
                .get(market_name)
                .unwrap()
                .1)[5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
        );
        let bids = market_external.bids;
        let asks = market_external.asks;
        let event_q = market_external.event_q;

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: {
                    anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::Serum3CancelOrder {
                            group: self.group(),
                            account: account_pubkey,
                            serum_market: serum3_market.0,
                            serum_program: serum3_market.1.serum_program,
                            serum_market_external: serum3_market.1.serum_market_external,
                            open_orders,
                            market_bids: from_serum_style_pubkey(&bids),
                            market_asks: from_serum_style_pubkey(&asks),
                            market_event_queue: from_serum_style_pubkey(&event_q),
                            owner: self.payer(),
                        },
                        None,
                    )
                },
                data: anchor_lang::InstructionData::data(
                    &mango_v4::instruction::Serum3CancelOrder { side, order_id },
                ),
            })
            .send()
            .map_err(prettify_client_error)?;

        Ok(())
    }

    //
    // Perps
    //

    //
    // Liquidation
    //

    pub fn liq_token_with_token(
        &self,
        liqee: (&Pubkey, &MangoAccount),
        asset_token_index: TokenIndex,
        liab_token_index: TokenIndex,
        max_liab_transfer: I80F48,
    ) -> anyhow::Result<Signature> {
        let health_remaining_ams = self
            .derive_liquidation_health_check_remaining_account_metas(
                liqee.1,
                asset_token_index,
                liab_token_index,
            )
            .unwrap();

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: {
                    let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::LiqTokenWithToken {
                            group: self.group(),
                            liqee: *liqee.0,
                            liqor: self.mango_account_cache.0,
                            liqor_owner: self.payer.pubkey(),
                        },
                        None,
                    );
                    ams.extend(health_remaining_ams);
                    ams
                },
                data: anchor_lang::InstructionData::data(
                    &mango_v4::instruction::LiqTokenWithToken {
                        asset_token_index,
                        liab_token_index,
                        max_liab_transfer,
                    },
                ),
            })
            .send()
            .map_err(prettify_client_error)
    }

    pub fn liq_token_bankruptcy(
        &self,
        liqee: (&Pubkey, &MangoAccount),
        liab_token_index: TokenIndex,
        max_liab_transfer: I80F48,
    ) -> anyhow::Result<Signature> {
        let quote_token_index = 0;

        let (_, quote_mint_info, _) = self
            .mint_infos_cache_by_token_index
            .get(&quote_token_index)
            .unwrap();
        let (liab_mint_info_key, liab_mint_info, _) = self
            .mint_infos_cache_by_token_index
            .get(&liab_token_index)
            .unwrap();

        let bank_remaining_ams = liab_mint_info
            .banks()
            .iter()
            .map(|bank_pubkey| AccountMeta {
                pubkey: *bank_pubkey,
                is_signer: false,
                is_writable: true,
            })
            .collect::<Vec<_>>();

        let health_remaining_ams = self
            .derive_liquidation_health_check_remaining_account_metas(
                liqee.1,
                quote_token_index,
                liab_token_index,
            )
            .unwrap();

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: {
                    let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::LiqTokenBankruptcy {
                            group: self.group(),
                            liqee: *liqee.0,
                            liqor: self.mango_account_cache.0,
                            liqor_owner: self.payer.pubkey(),
                            liab_mint_info: *liab_mint_info_key,
                            quote_vault: quote_mint_info.first_vault(),
                            insurance_vault: self.group_cache.insurance_vault,
                            token_program: Token::id(),
                        },
                        None,
                    );
                    ams.extend(bank_remaining_ams);
                    ams.extend(health_remaining_ams);
                    ams
                },
                data: anchor_lang::InstructionData::data(
                    &mango_v4::instruction::LiqTokenBankruptcy {
                        liab_token_index,
                        max_liab_transfer,
                    },
                ),
            })
            .send()
            .map_err(prettify_client_error)
    }
}

fn from_serum_style_pubkey(d: &[u64; 4]) -> Pubkey {
    Pubkey::new(bytemuck::cast_slice(d as &[_]))
}

/// Do some manual unpacking on some ClientErrors
///
/// Unfortunately solana's RpcResponseError will very unhelpfully print [N log messages]
/// instead of showing the actual log messages. This unpacks the error to provide more useful
/// output.
fn prettify_client_error(err: anchor_client::ClientError) -> anyhow::Error {
    use solana_client::client_error::ClientErrorKind;
    use solana_client::rpc_request::{RpcError, RpcResponseErrorData};
    match &err {
        anchor_client::ClientError::SolanaClientError(c) => {
            match c.kind() {
                ClientErrorKind::RpcError(RpcError::RpcResponseError { data, .. }) => match data {
                    RpcResponseErrorData::SendTransactionPreflightFailure(s) => {
                        if let Some(logs) = s.logs.as_ref() {
                            return anyhow::anyhow!(
                                "transaction simulation error. logs:\n{}",
                                logs.iter().map(|l| format!("    {}", l)).join("\n")
                            );
                        }
                    }
                    _ => {}
                },
                _ => {}
            };
        }
        _ => {}
    };
    err.into()
}
