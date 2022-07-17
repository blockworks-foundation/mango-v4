use std::collections::HashMap;
use std::sync::Mutex;

use anchor_client::{Client, ClientError, Cluster, Program};

use anchor_lang::__private::bytemuck;
use anchor_lang::prelude::System;
use anchor_lang::{AccountDeserialize, Id};
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::Token;

use fixed::types::I80F48;
use itertools::Itertools;
use mango_v4::instructions::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};
use mango_v4::state::{
    Bank, Group, MangoAccount, MintInfo, PerpMarket, PerpMarketIndex, Serum3Market,
    Serum3MarketIndex, TokenIndex,
};

use solana_client::rpc_client::RpcClient;

use crate::util::MyClone;
use anyhow::Context;
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_sdk::account::Account;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::sysvar;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signer::Signer};

// todo: might want to integrate geyser, websockets, or simple http polling for keeping data fresh
pub struct MangoClient {
    pub rpc: RpcClient,
    pub cluster: Cluster,
    pub commitment: CommitmentConfig,
    pub account_fetcher: Box<dyn AccountFetcher>,
    pub payer: Keypair,

    pub mango_account_address: Pubkey,

    pub context: MangoGroupContext,
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

    /// Conveniently creates a RPC based client
    pub fn new(
        cluster: Cluster,
        commitment: CommitmentConfig,
        group: Pubkey,
        payer: Keypair,
        mango_account_name: &str,
    ) -> anyhow::Result<Self> {
        let program =
            Client::new_with_options(cluster.clone(), std::rc::Rc::new(payer.clone()), commitment)
                .program(mango_v4::ID);

        let group_context = MangoGroupContext::new_from_rpc(&program, group)?;

        let account_fetcher = Box::new(CachedAccountFetcher::new(RpcAccountFetcher {
            rpc: program.rpc(),
        }));

        Self::new_detail(
            cluster,
            commitment,
            payer,
            mango_account_name,
            group_context,
            account_fetcher,
        )
    }

    /// Allows control of AccountFetcher and externally created MangoGroupContext
    pub fn new_detail(
        cluster: Cluster,
        commitment: CommitmentConfig,
        payer: Keypair,
        mango_account_name: &str,
        // future: maybe pass Arc<MangoGroupContext>, so it can be extenally updated?
        group_context: MangoGroupContext,
        account_fetcher: Box<dyn AccountFetcher>,
    ) -> anyhow::Result<Self> {
        let program =
            Client::new_with_options(cluster.clone(), std::rc::Rc::new(payer.clone()), commitment)
                .program(mango_v4::ID);

        let rpc = program.rpc();
        let group = group_context.group;

        // Mango Account
        let mut mango_account_tuples = fetch_mango_accounts(&program, group, payer.pubkey())?;
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
        let mango_account_tuples = fetch_mango_accounts(&program, group, payer.pubkey())?;
        let index = mango_account_tuples
            .iter()
            .position(|tuple| tuple.1.name() == mango_account_name)
            .unwrap();
        let mango_account_cache = mango_account_tuples[index];

        Ok(Self {
            rpc,
            cluster: cluster.clone(),
            commitment,
            account_fetcher,
            payer,
            mango_account_address: mango_account_cache.0,
            context: group_context,
        })
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
        self.context.group
    }

    pub fn mango_account(&self) -> Result<MangoAccount, ClientError> {
        account_fetcher_fetch_anchor_account(&*self.account_fetcher, self.mango_account_address)
    }

    pub fn first_bank(&self, token_index: TokenIndex) -> Result<Bank, ClientError> {
        let bank_address = self.context.mint_info(token_index).first_bank();
        account_fetcher_fetch_anchor_account(&*self.account_fetcher, bank_address)
    }

    pub fn derive_health_check_remaining_account_metas(
        &self,
        affected_token: Option<TokenIndex>,
        writable_banks: bool,
    ) -> Result<Vec<AccountMeta>, anchor_client::ClientError> {
        // figure out all the banks/oracles that need to be passed for the health check
        let mut banks = vec![];
        let mut oracles = vec![];
        let account = self.mango_account()?;
        for position in account.tokens.iter_active() {
            let mint_info = self.context.mint_info(position.token_index);
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
        if let Some(affected_token_index) = affected_token {
            if account
                .tokens
                .iter_active()
                .find(|p| p.token_index == affected_token_index)
                .is_none()
            {
                // If there is not yet an active position for the token, we need to pass
                // the bank/oracle for health check anyway.
                let new_position = account
                    .tokens
                    .values
                    .iter()
                    .position(|p| !p.is_active())
                    .unwrap();
                let mint_info = self.context.mint_info(affected_token_index);
                banks.insert(new_position, mint_info.first_bank());
                oracles.insert(new_position, mint_info.oracle);
            }
        }

        let serum_oos = account.serum3.iter_active().map(|&s| s.open_orders);
        let perp_markets = account
            .perps
            .iter_active_accounts()
            .map(|&pa| self.context.perp_market_address(pa.market_index));

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
        let account = self.mango_account()?;

        let token_indexes = liqee
            .tokens
            .iter_active()
            .chain(account.tokens.iter_active())
            .map(|ta| ta.token_index)
            .unique();

        for token_index in token_indexes {
            let mint_info = self.context.mint_info(token_index);
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
            .chain(account.serum3.iter_active())
            .map(|&s| s.open_orders);
        let perp_markets = liqee
            .perps
            .iter_active_accounts()
            .chain(account.perps.iter_active_accounts())
            .map(|&pa| self.context.perp_market_address(pa.market_index));

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
        let token_index = *self.context.token_indexes_by_name.get(token_name).unwrap();
        let mint_info = self.context.mint_info(token_index);

        let health_check_metas =
            self.derive_health_check_remaining_account_metas(Some(token_index), false)?;

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: {
                    let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::TokenDeposit {
                            group: self.group(),
                            account: self.mango_account_address,
                            bank: mint_info.first_bank(),
                            vault: mint_info.first_vault(),
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
        let token_index = *self.context.token_indexes_by_name.get(token_name).unwrap();
        let mint_info = self.context.mint_info(token_index);
        let oracle_account = self.account_fetcher.fetch_raw_account(mint_info.oracle)?;
        Ok(pyth_sdk_solana::load_price(&oracle_account.data).unwrap())
    }

    //
    // Serum3
    //

    pub fn serum3_create_open_orders(&self, name: &str) -> anyhow::Result<Signature> {
        let account_pubkey = self.mango_account_address;

        let market_index = *self
            .context
            .serum3_market_indexes_by_name
            .get(name)
            .unwrap();
        let serum3_info = self.context.serum3_markets.get(&market_index).unwrap();

        let open_orders = Pubkey::find_program_address(
            &[
                account_pubkey.as_ref(),
                b"Serum3OO".as_ref(),
                serum3_info.address.as_ref(),
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

                        serum_market: serum3_info.address,
                        serum_program: serum3_info.market.serum_program,
                        serum_market_external: serum3_info.market.serum_market_external,
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

    fn serum3_data<'a>(&'a self, name: &str) -> Result<Serum3Data<'a>, ClientError> {
        let market_index = *self
            .context
            .serum3_market_indexes_by_name
            .get(name)
            .unwrap();
        let serum3_info = self.context.serum3_markets.get(&market_index).unwrap();

        let quote_info = self.context.token(serum3_info.market.quote_token_index);
        let base_info = self.context.token(serum3_info.market.base_token_index);

        Ok(Serum3Data {
            market_index,
            market: serum3_info,
            quote: quote_info,
            base: base_info,
        })
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
        let s3 = self.serum3_data(name)?;

        let account = self.mango_account()?;
        let open_orders = account.serum3.find(s3.market_index).unwrap().open_orders;

        let health_check_metas = self.derive_health_check_remaining_account_metas(None, false)?;

        // https://github.com/project-serum/serum-ts/blob/master/packages/serum/src/market.ts#L1306
        let limit_price = {
            (price * ((10u64.pow(s3.quote.decimals as u32) * s3.market.coin_lot_size) as f64))
                as u64
                / (10u64.pow(s3.base.decimals as u32) * s3.market.pc_lot_size)
        };
        // https://github.com/project-serum/serum-ts/blob/master/packages/serum/src/market.ts#L1333
        let max_base_qty =
            { (size * 10u64.pow(s3.base.decimals as u32) as f64) as u64 / s3.market.coin_lot_size };
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
            (s3.market.pc_lot_size as f64 * (1f64 + rates.0)) as u64 * (limit_price * max_base_qty)
        };

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: {
                    let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::Serum3PlaceOrder {
                            group: self.group(),
                            account: self.mango_account_address,
                            open_orders,
                            quote_bank: s3.quote.mint_info.first_bank(),
                            quote_vault: s3.quote.mint_info.first_vault(),
                            base_bank: s3.base.mint_info.first_bank(),
                            base_vault: s3.base.mint_info.first_vault(),
                            serum_market: s3.market.address,
                            serum_program: s3.market.market.serum_program,
                            serum_market_external: s3.market.market.serum_market_external,
                            market_bids: s3.market.bids,
                            market_asks: s3.market.asks,
                            market_event_queue: s3.market.event_q,
                            market_request_queue: s3.market.req_q,
                            market_base_vault: s3.market.coin_vault,
                            market_quote_vault: s3.market.pc_vault,
                            market_vault_signer: s3.market.vault_signer,
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
        let s3 = self.serum3_data(name)?;

        let account = self.mango_account()?;
        let open_orders = account.serum3.find(s3.market_index).unwrap().open_orders;

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::Serum3SettleFunds {
                        group: self.group(),
                        account: self.mango_account_address,
                        open_orders,
                        quote_bank: s3.quote.mint_info.first_bank(),
                        quote_vault: s3.quote.mint_info.first_vault(),
                        base_bank: s3.base.mint_info.first_bank(),
                        base_vault: s3.base.mint_info.first_vault(),
                        serum_market: s3.market.address,
                        serum_program: s3.market.market.serum_program,
                        serum_market_external: s3.market.market.serum_market_external,
                        market_base_vault: s3.market.coin_vault,
                        market_quote_vault: s3.market.pc_vault,
                        market_vault_signer: s3.market.vault_signer,
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
        let market_index = *self
            .context
            .serum3_market_indexes_by_name
            .get(market_name)
            .unwrap();
        let account = self.mango_account()?;
        let open_orders = account.serum3.find(market_index).unwrap().open_orders;

        let open_orders_bytes = self.account_fetcher.fetch_raw_account(open_orders)?.data;
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
        let s3 = self.serum3_data(market_name)?;

        let account = self.mango_account()?;
        let open_orders = account.serum3.find(s3.market_index).unwrap().open_orders;

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: {
                    anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::Serum3CancelOrder {
                            group: self.group(),
                            account: self.mango_account_address,
                            serum_market: s3.market.address,
                            serum_program: s3.market.market.serum_program,
                            serum_market_external: s3.market.market.serum_market_external,
                            open_orders,
                            market_bids: s3.market.bids,
                            market_asks: s3.market.asks,
                            market_event_queue: s3.market.event_q,
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
                            liqor: self.mango_account_address,
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

        let quote_info = self.context.token(quote_token_index);
        let liab_info = self.context.token(liab_token_index);

        let bank_remaining_ams = liab_info
            .mint_info
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

        let group = account_fetcher_fetch_anchor_account::<Group>(
            &*self.account_fetcher,
            self.context.group,
        )?;

        self.program()
            .request()
            .instruction(Instruction {
                program_id: mango_v4::id(),
                accounts: {
                    let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::LiqTokenBankruptcy {
                            group: self.group(),
                            liqee: *liqee.0,
                            liqor: self.mango_account_address,
                            liqor_owner: self.payer.pubkey(),
                            liab_mint_info: liab_info.mint_info_address,
                            quote_vault: quote_info.mint_info.first_vault(),
                            insurance_vault: group.insurance_vault,
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

pub trait AccountFetcher: Sync + Send {
    fn fetch_raw_account(&self, address: Pubkey) -> Result<Account, ClientError>;
}

// Can't be in the trait, since then it would no longer be object-safe...
fn account_fetcher_fetch_anchor_account<T: AccountDeserialize>(
    fetcher: &dyn AccountFetcher,
    address: Pubkey,
) -> Result<T, ClientError> {
    let account = fetcher.fetch_raw_account(address)?;
    let mut data: &[u8] = &account.data;
    Ok(T::try_deserialize(&mut data)?)
}

fn fetch_mango_accounts(
    program: &Program,
    group: Pubkey,
    owner: Pubkey,
) -> Result<Vec<(Pubkey, MangoAccount)>, ClientError> {
    program.accounts::<MangoAccount>(vec![
        RpcFilterType::Memcmp(Memcmp {
            offset: 8,
            bytes: MemcmpEncodedBytes::Base58(group.to_string()),
            encoding: None,
        }),
        RpcFilterType::Memcmp(Memcmp {
            offset: 40,
            bytes: MemcmpEncodedBytes::Base58(owner.to_string()),
            encoding: None,
        }),
    ])
}

fn fetch_raw_account(program: &Program, address: Pubkey) -> Result<Account, ClientError> {
    let rpc = program.rpc();
    rpc.get_account_with_commitment(&address, rpc.commitment())?
        .value
        .ok_or(ClientError::AccountNotFound)
}

fn fetch_banks(program: &Program, group: Pubkey) -> Result<Vec<(Pubkey, Bank)>, ClientError> {
    program.accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp {
        offset: 8,
        bytes: MemcmpEncodedBytes::Base58(group.to_string()),
        encoding: None,
    })])
}

fn fetch_mint_infos(
    program: &Program,
    group: Pubkey,
) -> Result<Vec<(Pubkey, MintInfo)>, ClientError> {
    program.accounts::<MintInfo>(vec![RpcFilterType::Memcmp(Memcmp {
        offset: 8,
        bytes: MemcmpEncodedBytes::Base58(group.to_string()),
        encoding: None,
    })])
}

fn fetch_serum3_markets(
    program: &Program,
    group: Pubkey,
) -> Result<Vec<(Pubkey, Serum3Market)>, ClientError> {
    program.accounts::<Serum3Market>(vec![RpcFilterType::Memcmp(Memcmp {
        offset: 8,
        bytes: MemcmpEncodedBytes::Base58(group.to_string()),
        encoding: None,
    })])
}

fn fetch_perp_markets(
    program: &Program,
    group: Pubkey,
) -> Result<Vec<(Pubkey, PerpMarket)>, ClientError> {
    program.accounts::<PerpMarket>(vec![RpcFilterType::Memcmp(Memcmp {
        offset: 8,
        bytes: MemcmpEncodedBytes::Base58(group.to_string()),
        encoding: None,
    })])
}

pub struct RpcAccountFetcher {
    rpc: RpcClient,
}

impl AccountFetcher for RpcAccountFetcher {
    fn fetch_raw_account(&self, address: Pubkey) -> Result<Account, ClientError> {
        self.rpc
            .get_account_with_commitment(&address, self.rpc.commitment())?
            .value
            .ok_or(ClientError::AccountNotFound)
    }
}

pub struct CachedAccountFetcher<T: AccountFetcher> {
    fetcher: T,
    cache: Mutex<HashMap<Pubkey, Account>>,
}

impl<T: AccountFetcher> CachedAccountFetcher<T> {
    fn new(fetcher: T) -> Self {
        Self {
            fetcher,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}

impl<T: AccountFetcher> AccountFetcher for CachedAccountFetcher<T> {
    fn fetch_raw_account(&self, address: Pubkey) -> Result<Account, ClientError> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(account) = cache.get(&address) {
            return Ok(account.clone());
        }
        let account = self.fetcher.fetch_raw_account(address)?;
        cache.insert(address, account.clone());
        Ok(account)
    }
}

pub struct TokenContext {
    pub name: String,
    pub mint_info: MintInfo,
    pub mint_info_address: Pubkey,
    pub decimals: u8,
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

    pub fn perp_market_address(&self, perp_market_index: PerpMarketIndex) -> Pubkey {
        self.perp_markets.get(&perp_market_index).unwrap().address
    }

    pub fn new_from_rpc(program: &Program, group: Pubkey) -> Result<Self, ClientError> {
        // tokens
        let mint_info_tuples = fetch_mint_infos(&program, group)?;
        let mut tokens = mint_info_tuples
            .iter()
            .map(|(pk, mi)| {
                (
                    mi.token_index,
                    TokenContext {
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
        let bank_tuples = fetch_banks(program, group)?;
        for (_, bank) in bank_tuples {
            let token = tokens.get_mut(&bank.token_index).unwrap();
            token.name = bank.name().into();
            token.decimals = bank.mint_decimals;
        }
        assert!(tokens.values().all(|t| t.decimals != u8::MAX));

        // serum3 markets
        let serum3_market_tuples = fetch_serum3_markets(program, group)?;
        let serum3_markets = serum3_market_tuples
            .iter()
            .map(|(pk, s)| {
                let market_external_account = fetch_raw_account(program, s.serum_market_external)?;
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
        let perp_market_tuples = fetch_perp_markets(program, group)?;
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
}

struct Serum3Data<'a> {
    market_index: Serum3MarketIndex,
    market: &'a Serum3MarketContext,
    quote: &'a TokenContext,
    base: &'a TokenContext,
}

fn from_serum_style_pubkey(d: [u64; 4]) -> Pubkey {
    Pubkey::new(bytemuck::cast_slice(&d as &[_]))
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
