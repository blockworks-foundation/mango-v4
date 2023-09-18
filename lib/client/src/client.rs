use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anchor_client::Cluster;

use anchor_lang::__private::bytemuck;
use anchor_lang::prelude::System;
use anchor_lang::{AccountDeserialize, Id};
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::Token;

use fixed::types::I80F48;
use futures::{stream, StreamExt, TryStreamExt};
use itertools::Itertools;

use mango_v4::accounts_ix::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::state::{
    Bank, Group, MangoAccountValue, PerpMarketIndex, PlaceOrderType, SelfTradeBehavior,
    Serum3MarketIndex, Side, TokenIndex, INSURANCE_TOKEN_INDEX,
};

use solana_address_lookup_table_program::state::AddressLookupTable;
use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::address_lookup_table_account::AddressLookupTableAccount;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::hash::Hash;
use solana_sdk::signer::keypair;
use solana_sdk::transaction::TransactionError;

use crate::account_fetcher::*;
use crate::context::MangoGroupContext;
use crate::gpa::{fetch_anchor_account, fetch_mango_accounts};
use crate::{jupiter, util};

use anyhow::Context;
use solana_sdk::account::ReadableAccount;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::sysvar;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signer::Signer};

pub const MAX_ACCOUNTS_PER_TRANSACTION: usize = 64;

// very close to anchor_client::Client, which unfortunately has no accessors or Clone
#[derive(Clone, Debug)]
pub struct Client {
    pub cluster: Cluster,
    pub fee_payer: Arc<Keypair>,
    pub commitment: CommitmentConfig,
    pub timeout: Option<Duration>,
    pub transaction_builder_config: TransactionBuilderConfig,
    pub rpc_send_transaction_config: RpcSendTransactionConfig,
}

impl Client {
    pub fn new(
        cluster: Cluster,
        commitment: CommitmentConfig,
        fee_payer: Arc<Keypair>,
        timeout: Option<Duration>,
        transaction_builder_config: TransactionBuilderConfig,
    ) -> Self {
        Self {
            cluster,
            fee_payer,
            commitment,
            timeout,
            transaction_builder_config,
            rpc_send_transaction_config: RpcSendTransactionConfig {
                preflight_commitment: Some(CommitmentLevel::Processed),
                ..Default::default()
            },
        }
    }

    pub fn rpc_async(&self) -> RpcClientAsync {
        let url = self.cluster.url().to_string();
        if let Some(timeout) = self.timeout.as_ref() {
            RpcClientAsync::new_with_timeout_and_commitment(url, *timeout, self.commitment)
        } else {
            RpcClientAsync::new_with_commitment(url, self.commitment)
        }
    }

    // TODO: this function here is awkward, since it (intentionally) doesn't use MangoClient::account_fetcher
    pub async fn rpc_anchor_account<T: AccountDeserialize>(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<T> {
        fetch_anchor_account(&self.rpc_async(), address).await
    }
}

// todo: might want to integrate geyser, websockets, or simple http polling for keeping data fresh
pub struct MangoClient {
    pub client: Client,

    // todo: possibly this object should have cache-functions, so there can be one getMultipleAccounts
    // call to refresh banks etc -- if it's backed by websockets, these could just do nothing
    pub account_fetcher: Arc<dyn AccountFetcher>,

    pub owner: Arc<Keypair>,
    pub mango_account_address: Pubkey,

    pub context: MangoGroupContext,

    pub http_client: reqwest::Client,
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

    pub async fn find_accounts(
        client: &Client,
        group: Pubkey,
        owner: &Keypair,
    ) -> anyhow::Result<Vec<(Pubkey, MangoAccountValue)>> {
        fetch_mango_accounts(&client.rpc_async(), mango_v4::ID, group, owner.pubkey()).await
    }

    pub async fn find_or_create_account(
        client: &Client,
        group: Pubkey,
        owner: Arc<Keypair>,
        payer: Arc<Keypair>, // pays the SOL for the new account
        mango_account_name: &str,
    ) -> anyhow::Result<Pubkey> {
        let rpc = client.rpc_async();
        let program = mango_v4::ID;
        let owner_pk = owner.pubkey();

        // Mango Account
        let mut mango_account_tuples = fetch_mango_accounts(&rpc, program, group, owner_pk).await?;
        let mango_account_opt = mango_account_tuples
            .iter()
            .find(|(_, account)| account.fixed.name() == mango_account_name);
        if mango_account_opt.is_none() {
            mango_account_tuples.sort_by(|a, b| {
                a.1.fixed
                    .account_num
                    .partial_cmp(&b.1.fixed.account_num)
                    .unwrap()
            });
            let account_num = match mango_account_tuples.last() {
                Some(tuple) => tuple.1.fixed.account_num + 1,
                None => 0u32,
            };
            Self::create_account(
                client,
                group,
                owner.clone(),
                payer,
                account_num,
                mango_account_name,
            )
            .await
            .context("Failed to create account...")?;
        }
        let mango_account_tuples = fetch_mango_accounts(&rpc, program, group, owner_pk).await?;
        let index = mango_account_tuples
            .iter()
            .position(|tuple| tuple.1.fixed.name() == mango_account_name)
            .unwrap();
        Ok(mango_account_tuples[index].0)
    }

    pub async fn create_account(
        client: &Client,
        group: Pubkey,
        owner: Arc<Keypair>,
        payer: Arc<Keypair>, // pays the SOL for the new account
        account_num: u32,
        mango_account_name: &str,
    ) -> anyhow::Result<(Pubkey, Signature)> {
        let account = Pubkey::find_program_address(
            &[
                b"MangoAccount".as_ref(),
                group.as_ref(),
                owner.pubkey().as_ref(),
                &account_num.to_le_bytes(),
            ],
            &mango_v4::id(),
        )
        .0;
        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &mango_v4::accounts::AccountCreate {
                    group,
                    owner: owner.pubkey(),
                    account,
                    payer: payer.pubkey(),
                    system_program: System::id(),
                },
                None,
            ),
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::AccountCreate {
                account_num,
                name: mango_account_name.to_owned(),
                token_count: 8,
                serum3_count: 4,
                perp_count: 4,
                perp_oo_count: 8,
            }),
        };

        let txsig = TransactionBuilder {
            instructions: vec![ix],
            address_lookup_tables: vec![],
            payer: payer.pubkey(),
            signers: vec![owner, payer],
            config: client.transaction_builder_config,
        }
        .send_and_confirm(&client)
        .await?;

        Ok((account, txsig))
    }

    /// Conveniently creates a RPC based client
    pub async fn new_for_existing_account(
        client: Client,
        account: Pubkey,
        owner: Arc<Keypair>,
    ) -> anyhow::Result<Self> {
        let rpc = client.rpc_async();
        let account_fetcher = Arc::new(CachedAccountFetcher::new(Arc::new(RpcAccountFetcher {
            rpc,
        })));
        let mango_account =
            account_fetcher_fetch_mango_account(&*account_fetcher, &account).await?;
        let group = mango_account.fixed.group;
        if mango_account.fixed.owner != owner.pubkey() {
            anyhow::bail!(
                "bad owner for account: expected {} got {}",
                mango_account.fixed.owner,
                owner.pubkey()
            );
        }

        let rpc = client.rpc_async();
        let group_context = MangoGroupContext::new_from_rpc(&rpc, group).await?;

        Self::new_detail(client, account, owner, group_context, account_fetcher)
    }

    /// Allows control of AccountFetcher and externally created MangoGroupContext
    pub fn new_detail(
        client: Client,
        account: Pubkey,
        owner: Arc<Keypair>,
        // future: maybe pass Arc<MangoGroupContext>, so it can be extenally updated?
        group_context: MangoGroupContext,
        account_fetcher: Arc<dyn AccountFetcher>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            client,
            account_fetcher,
            owner,
            mango_account_address: account,
            context: group_context,
            http_client: reqwest::Client::new(),
        })
    }

    pub fn owner(&self) -> Pubkey {
        self.owner.pubkey()
    }

    pub fn group(&self) -> Pubkey {
        self.context.group
    }

    pub async fn mango_account(&self) -> anyhow::Result<MangoAccountValue> {
        account_fetcher_fetch_mango_account(&*self.account_fetcher, &self.mango_account_address)
            .await
    }

    pub async fn first_bank(&self, token_index: TokenIndex) -> anyhow::Result<Bank> {
        let bank_address = self.context.mint_info(token_index).first_bank();
        account_fetcher_fetch_anchor_account(&*self.account_fetcher, &bank_address).await
    }

    pub async fn derive_health_check_remaining_account_metas(
        &self,
        affected_tokens: Vec<TokenIndex>,
        writable_banks: Vec<TokenIndex>,
        affected_perp_markets: Vec<PerpMarketIndex>,
    ) -> anyhow::Result<Vec<AccountMeta>> {
        let account = self.mango_account().await?;
        self.context.derive_health_check_remaining_account_metas(
            &account,
            affected_tokens,
            writable_banks,
            affected_perp_markets,
        )
    }

    pub async fn derive_liquidation_health_check_remaining_account_metas(
        &self,
        liqee: &MangoAccountValue,
        affected_tokens: &[TokenIndex],
        writable_banks: &[TokenIndex],
    ) -> anyhow::Result<Vec<AccountMeta>> {
        let account = self.mango_account().await?;
        self.context
            .derive_health_check_remaining_account_metas_two_accounts(
                &account,
                liqee,
                affected_tokens,
                writable_banks,
            )
    }

    pub async fn token_deposit(
        &self,
        mint: Pubkey,
        amount: u64,
        reduce_only: bool,
    ) -> anyhow::Result<Signature> {
        let token = self.context.token_by_mint(&mint)?;
        let token_index = token.token_index;
        let mint_info = token.mint_info;

        let health_check_metas = self
            .derive_health_check_remaining_account_metas(vec![token_index], vec![], vec![])
            .await?;

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::TokenDeposit {
                        group: self.group(),
                        account: self.mango_account_address,
                        owner: self.owner(),
                        bank: mint_info.first_bank(),
                        vault: mint_info.first_vault(),
                        oracle: mint_info.oracle,
                        token_account: get_associated_token_address(&self.owner(), &mint_info.mint),
                        token_authority: self.owner(),
                        token_program: Token::id(),
                    },
                    None,
                );
                ams.extend(health_check_metas.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::TokenDeposit {
                amount,
                reduce_only,
            }),
        };
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    pub async fn token_withdraw(
        &self,
        mint: Pubkey,
        amount: u64,
        allow_borrow: bool,
    ) -> anyhow::Result<Signature> {
        let token = self.context.token_by_mint(&mint)?;
        let token_index = token.token_index;
        let mint_info = token.mint_info;

        let health_check_metas = self
            .derive_health_check_remaining_account_metas(vec![token_index], vec![], vec![])
            .await?;

        let ixs = vec![
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &self.owner(),
                &self.owner(),
                &mint,
                &Token::id(),
            ),
            Instruction {
                program_id: mango_v4::id(),
                accounts: {
                    let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                        &mango_v4::accounts::TokenWithdraw {
                            group: self.group(),
                            account: self.mango_account_address,
                            owner: self.owner(),
                            bank: mint_info.first_bank(),
                            vault: mint_info.first_vault(),
                            oracle: mint_info.oracle,
                            token_account: get_associated_token_address(
                                &self.owner(),
                                &mint_info.mint,
                            ),
                            token_program: Token::id(),
                        },
                        None,
                    );
                    ams.extend(health_check_metas.into_iter());
                    ams
                },
                data: anchor_lang::InstructionData::data(&mango_v4::instruction::TokenWithdraw {
                    amount,
                    allow_borrow,
                }),
            },
        ];
        self.send_and_confirm_owner_tx(ixs).await
    }

    pub async fn bank_oracle_price(&self, token_index: TokenIndex) -> anyhow::Result<I80F48> {
        let bank = self.first_bank(token_index).await?;
        let mint_info = self.context.mint_info(token_index);
        let oracle = self
            .account_fetcher
            .fetch_raw_account(&mint_info.oracle)
            .await?;
        let price = bank.oracle_price(
            &KeyedAccountSharedData::new(mint_info.oracle, oracle.into()),
            None,
        )?;
        Ok(price)
    }

    pub async fn perp_oracle_price(
        &self,
        perp_market_index: PerpMarketIndex,
    ) -> anyhow::Result<I80F48> {
        let perp = self.context.perp(perp_market_index);
        let oracle = self
            .account_fetcher
            .fetch_raw_account(&perp.market.oracle)
            .await?;
        let price = perp.market.oracle_price(
            &KeyedAccountSharedData::new(perp.market.oracle, oracle.into()),
            None,
        )?;
        Ok(price)
    }

    //
    // Serum3
    //

    pub fn serum3_create_open_orders_instruction(
        &self,
        market_index: Serum3MarketIndex,
    ) -> Instruction {
        let account_pubkey = self.mango_account_address;
        let s3 = self.context.serum3(market_index);

        let open_orders = Pubkey::find_program_address(
            &[
                b"Serum3OO".as_ref(),
                account_pubkey.as_ref(),
                s3.address.as_ref(),
            ],
            &mango_v4::ID,
        )
        .0;

        Instruction {
            program_id: mango_v4::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &mango_v4::accounts::Serum3CreateOpenOrders {
                    group: self.group(),
                    account: account_pubkey,
                    serum_market: s3.address,
                    serum_program: s3.market.serum_program,
                    serum_market_external: s3.market.serum_market_external,
                    open_orders,
                    owner: self.owner(),
                    payer: self.owner(),
                    system_program: System::id(),
                    rent: sysvar::rent::id(),
                },
                None,
            ),
            data: anchor_lang::InstructionData::data(
                &mango_v4::instruction::Serum3CreateOpenOrders {},
            ),
        }
    }

    pub async fn serum3_create_open_orders(&self, name: &str) -> anyhow::Result<Signature> {
        let market_index = self.context.serum3_market_index(name);
        let ix = self.serum3_create_open_orders_instruction(market_index);
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    #[allow(clippy::too_many_arguments)]
    pub fn serum3_place_order_instruction(
        &self,
        account: &MangoAccountValue,
        market_index: Serum3MarketIndex,
        side: Serum3Side,
        limit_price: u64,
        max_base_qty: u64,
        max_native_quote_qty_including_fees: u64,
        self_trade_behavior: Serum3SelfTradeBehavior,
        order_type: Serum3OrderType,
        client_order_id: u64,
        limit: u16,
    ) -> anyhow::Result<Instruction> {
        let s3 = self.context.serum3(market_index);
        let base = self.context.serum3_base_token(market_index);
        let quote = self.context.serum3_quote_token(market_index);
        let open_orders = account
            .serum3_orders(market_index)
            .expect("oo is created")
            .open_orders;

        let health_check_metas = self.context.derive_health_check_remaining_account_metas(
            account,
            vec![],
            vec![],
            vec![],
        )?;

        let payer_mint_info = match side {
            Serum3Side::Bid => quote.mint_info,
            Serum3Side::Ask => base.mint_info,
        };

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::Serum3PlaceOrder {
                        group: self.group(),
                        account: self.mango_account_address,
                        open_orders,
                        payer_bank: payer_mint_info.first_bank(),
                        payer_vault: payer_mint_info.first_vault(),
                        payer_oracle: payer_mint_info.oracle,
                        serum_market: s3.address,
                        serum_program: s3.market.serum_program,
                        serum_market_external: s3.market.serum_market_external,
                        market_bids: s3.bids,
                        market_asks: s3.asks,
                        market_event_queue: s3.event_q,
                        market_request_queue: s3.req_q,
                        market_base_vault: s3.coin_vault,
                        market_quote_vault: s3.pc_vault,
                        market_vault_signer: s3.vault_signer,
                        owner: self.owner(),
                        token_program: Token::id(),
                    },
                    None,
                );
                ams.extend(health_check_metas.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::Serum3PlaceOrder {
                side,
                limit_price,
                max_base_qty,
                max_native_quote_qty_including_fees,
                self_trade_behavior,
                order_type,
                client_order_id,
                limit,
            }),
        };

        Ok(ix)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn serum3_place_order(
        &self,
        name: &str,
        side: Serum3Side,
        limit_price: u64,
        max_base_qty: u64,
        max_native_quote_qty_including_fees: u64,
        self_trade_behavior: Serum3SelfTradeBehavior,
        order_type: Serum3OrderType,
        client_order_id: u64,
        limit: u16,
    ) -> anyhow::Result<Signature> {
        let account = self.mango_account().await?;
        let market_index = self.context.serum3_market_index(name);
        let ix = self.serum3_place_order_instruction(
            &account,
            market_index,
            side,
            limit_price,
            max_base_qty,
            max_native_quote_qty_including_fees,
            self_trade_behavior,
            order_type,
            client_order_id,
            limit,
        )?;
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    pub async fn serum3_settle_funds(&self, name: &str) -> anyhow::Result<Signature> {
        let market_index = self.context.serum3_market_index(name);
        let s3 = self.context.serum3(market_index);
        let base = self.context.serum3_base_token(market_index);
        let quote = self.context.serum3_quote_token(market_index);

        let account = self.mango_account().await?;
        let open_orders = account.serum3_orders(market_index).unwrap().open_orders;

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &mango_v4::accounts::Serum3SettleFundsV2 {
                    v1: mango_v4::accounts::Serum3SettleFunds {
                        group: self.group(),
                        account: self.mango_account_address,
                        open_orders,
                        quote_bank: quote.mint_info.first_bank(),
                        quote_vault: quote.mint_info.first_vault(),
                        base_bank: base.mint_info.first_bank(),
                        base_vault: base.mint_info.first_vault(),
                        serum_market: s3.address,
                        serum_program: s3.market.serum_program,
                        serum_market_external: s3.market.serum_market_external,
                        market_base_vault: s3.coin_vault,
                        market_quote_vault: s3.pc_vault,
                        market_vault_signer: s3.vault_signer,
                        owner: self.owner(),
                        token_program: Token::id(),
                    },
                    v2: mango_v4::accounts::Serum3SettleFundsV2Extra {
                        quote_oracle: quote.mint_info.oracle,
                        base_oracle: base.mint_info.oracle,
                    },
                },
                None,
            ),
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::Serum3SettleFundsV2 {
                fees_to_dao: true,
            }),
        };
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    pub fn serum3_cancel_all_orders_instruction(
        &self,
        account: &MangoAccountValue,
        market_index: Serum3MarketIndex,
        limit: u8,
    ) -> anyhow::Result<Instruction> {
        let s3 = self.context.serum3(market_index);
        let open_orders = account.serum3_orders(market_index)?.open_orders;

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &mango_v4::accounts::Serum3CancelAllOrders {
                    group: self.group(),
                    account: self.mango_account_address,
                    open_orders,
                    market_bids: s3.bids,
                    market_asks: s3.asks,
                    market_event_queue: s3.event_q,
                    serum_market: s3.address,
                    serum_program: s3.market.serum_program,
                    serum_market_external: s3.market.serum_market_external,
                    owner: self.owner(),
                },
                None,
            ),
            data: anchor_lang::InstructionData::data(
                &mango_v4::instruction::Serum3CancelAllOrders { limit },
            ),
        };

        Ok(ix)
    }

    pub async fn serum3_cancel_all_orders(
        &self,
        market_name: &str,
    ) -> Result<Vec<u128>, anyhow::Error> {
        let market_index = self.context.serum3_market_index(market_name);
        let account = self.mango_account().await?;
        let open_orders = account.serum3_orders(market_index).unwrap().open_orders;
        let open_orders_acc = self.account_fetcher.fetch_raw_account(&open_orders).await?;
        let open_orders_bytes = open_orders_acc.data();
        let open_orders_data: &serum_dex::state::OpenOrders = bytemuck::from_bytes(
            &open_orders_bytes[5..5 + std::mem::size_of::<serum_dex::state::OpenOrders>()],
        );

        let mut orders = vec![];
        for order_id in open_orders_data.orders {
            if order_id != 0 {
                // TODO: find side for order_id, and only cancel the relevant order
                self.serum3_cancel_order(market_name, Serum3Side::Bid, order_id)
                    .await
                    .ok();
                self.serum3_cancel_order(market_name, Serum3Side::Ask, order_id)
                    .await
                    .ok();

                orders.push(order_id);
            }
        }

        Ok(orders)
    }

    pub async fn serum3_liq_force_cancel_orders(
        &self,
        liqee: (&Pubkey, &MangoAccountValue),
        market_index: Serum3MarketIndex,
        open_orders: &Pubkey,
    ) -> anyhow::Result<Signature> {
        let s3 = self.context.serum3(market_index);
        let base = self.context.serum3_base_token(market_index);
        let quote = self.context.serum3_quote_token(market_index);

        let health_remaining_ams = self
            .context
            .derive_health_check_remaining_account_metas(liqee.1, vec![], vec![], vec![])
            .unwrap();

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::Serum3LiqForceCancelOrders {
                        group: self.group(),
                        account: *liqee.0,
                        open_orders: *open_orders,
                        serum_market: s3.address,
                        serum_program: s3.market.serum_program,
                        serum_market_external: s3.market.serum_market_external,
                        market_bids: s3.bids,
                        market_asks: s3.asks,
                        market_event_queue: s3.event_q,
                        market_base_vault: s3.coin_vault,
                        market_quote_vault: s3.pc_vault,
                        market_vault_signer: s3.vault_signer,
                        quote_bank: quote.mint_info.first_bank(),
                        quote_vault: quote.mint_info.first_vault(),
                        base_bank: base.mint_info.first_bank(),
                        base_vault: base.mint_info.first_vault(),
                        token_program: Token::id(),
                    },
                    None,
                );
                ams.extend(health_remaining_ams.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(
                &mango_v4::instruction::Serum3LiqForceCancelOrders { limit: 5 },
            ),
        };
        self.send_and_confirm_permissionless_tx(vec![ix]).await
    }

    pub async fn serum3_cancel_order(
        &self,
        market_name: &str,
        side: Serum3Side,
        order_id: u128,
    ) -> anyhow::Result<Signature> {
        let market_index = self.context.serum3_market_index(market_name);
        let s3 = self.context.serum3(market_index);

        let account = self.mango_account().await?;
        let open_orders = account.serum3_orders(market_index).unwrap().open_orders;

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::Serum3CancelOrder {
                        group: self.group(),
                        account: self.mango_account_address,
                        serum_market: s3.address,
                        serum_program: s3.market.serum_program,
                        serum_market_external: s3.market.serum_market_external,
                        open_orders,
                        market_bids: s3.bids,
                        market_asks: s3.asks,
                        market_event_queue: s3.event_q,
                        owner: self.owner(),
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::Serum3CancelOrder {
                side,
                order_id,
            }),
        };
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    //
    // Perps
    //

    #[allow(clippy::too_many_arguments)]
    pub fn perp_place_order_instruction(
        &self,
        account: &MangoAccountValue,
        market_index: PerpMarketIndex,
        side: Side,
        price_lots: i64,
        max_base_lots: i64,
        max_quote_lots: i64,
        client_order_id: u64,
        order_type: PlaceOrderType,
        reduce_only: bool,
        expiry_timestamp: u64,
        limit: u8,
        self_trade_behavior: SelfTradeBehavior,
    ) -> anyhow::Result<Instruction> {
        let perp = self.context.perp(market_index);
        let health_remaining_metas = self.context.derive_health_check_remaining_account_metas(
            account,
            vec![],
            vec![],
            vec![market_index],
        )?;

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::PerpPlaceOrder {
                        group: self.group(),
                        account: self.mango_account_address,
                        owner: self.owner(),
                        perp_market: perp.address,
                        bids: perp.market.bids,
                        asks: perp.market.asks,
                        event_queue: perp.market.event_queue,
                        oracle: perp.market.oracle,
                    },
                    None,
                );
                ams.extend(health_remaining_metas.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::PerpPlaceOrderV2 {
                side,
                price_lots,
                max_base_lots,
                max_quote_lots,
                client_order_id,
                order_type,
                reduce_only,
                expiry_timestamp,
                limit,
                self_trade_behavior,
            }),
        };

        Ok(ix)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn perp_place_order(
        &self,
        market_index: PerpMarketIndex,
        side: Side,
        price_lots: i64,
        max_base_lots: i64,
        max_quote_lots: i64,
        client_order_id: u64,
        order_type: PlaceOrderType,
        reduce_only: bool,
        expiry_timestamp: u64,
        limit: u8,
        self_trade_behavior: SelfTradeBehavior,
    ) -> anyhow::Result<Signature> {
        let account = self.mango_account().await?;
        let ix = self.perp_place_order_instruction(
            &account,
            market_index,
            side,
            price_lots,
            max_base_lots,
            max_quote_lots,
            client_order_id,
            order_type,
            reduce_only,
            expiry_timestamp,
            limit,
            self_trade_behavior,
        )?;
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    pub fn perp_cancel_all_orders_instruction(
        &self,
        market_index: PerpMarketIndex,
        limit: u8,
    ) -> anyhow::Result<Instruction> {
        let perp = self.context.perp(market_index);

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::PerpCancelAllOrders {
                        group: self.group(),
                        account: self.mango_account_address,
                        owner: self.owner(),
                        perp_market: perp.address,
                        bids: perp.market.bids,
                        asks: perp.market.asks,
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::PerpCancelAllOrders {
                limit,
            }),
        };
        Ok(ix)
    }

    pub async fn perp_deactivate_position(
        &self,
        market_index: PerpMarketIndex,
    ) -> anyhow::Result<Signature> {
        let perp = self.context.perp(market_index);

        let health_check_metas = self
            .derive_health_check_remaining_account_metas(vec![], vec![], vec![])
            .await?;

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::PerpDeactivatePosition {
                        group: self.group(),
                        account: self.mango_account_address,
                        owner: self.owner(),
                        perp_market: perp.address,
                    },
                    None,
                );
                ams.extend(health_check_metas.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(
                &mango_v4::instruction::PerpDeactivatePosition {},
            ),
        };
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    pub fn perp_settle_pnl_instruction(
        &self,
        market_index: PerpMarketIndex,
        account_a: (&Pubkey, &MangoAccountValue),
        account_b: (&Pubkey, &MangoAccountValue),
    ) -> anyhow::Result<Instruction> {
        let perp = self.context.perp(market_index);
        let settlement_token = self.context.token(perp.market.settle_token_index);

        let health_remaining_ams = self
            .context
            .derive_health_check_remaining_account_metas_two_accounts(
                account_a.1,
                account_b.1,
                &[],
                &[],
            )
            .unwrap();

        Ok(Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::PerpSettlePnl {
                        group: self.group(),
                        settler: self.mango_account_address,
                        settler_owner: self.owner(),
                        perp_market: perp.address,
                        account_a: *account_a.0,
                        account_b: *account_b.0,
                        oracle: perp.market.oracle,
                        settle_bank: settlement_token.mint_info.first_bank(),
                        settle_oracle: settlement_token.mint_info.oracle,
                    },
                    None,
                );
                ams.extend(health_remaining_ams.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::PerpSettlePnl {}),
        })
    }

    pub async fn perp_settle_pnl(
        &self,
        market_index: PerpMarketIndex,
        account_a: (&Pubkey, &MangoAccountValue),
        account_b: (&Pubkey, &MangoAccountValue),
    ) -> anyhow::Result<Signature> {
        let ix = self.perp_settle_pnl_instruction(market_index, account_a, account_b)?;
        self.send_and_confirm_permissionless_tx(vec![ix]).await
    }

    pub async fn perp_liq_force_cancel_orders(
        &self,
        liqee: (&Pubkey, &MangoAccountValue),
        market_index: PerpMarketIndex,
    ) -> anyhow::Result<Signature> {
        let perp = self.context.perp(market_index);

        let health_remaining_ams = self
            .context
            .derive_health_check_remaining_account_metas(liqee.1, vec![], vec![], vec![])
            .unwrap();

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::PerpLiqForceCancelOrders {
                        group: self.group(),
                        account: *liqee.0,
                        perp_market: perp.address,
                        bids: perp.market.bids,
                        asks: perp.market.asks,
                    },
                    None,
                );
                ams.extend(health_remaining_ams.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(
                &mango_v4::instruction::PerpLiqForceCancelOrders { limit: 5 },
            ),
        };
        self.send_and_confirm_permissionless_tx(vec![ix]).await
    }

    pub async fn perp_liq_base_or_positive_pnl_instruction(
        &self,
        liqee: (&Pubkey, &MangoAccountValue),
        market_index: PerpMarketIndex,
        max_base_transfer: i64,
        max_pnl_transfer: u64,
    ) -> anyhow::Result<Instruction> {
        let perp = self.context.perp(market_index);
        let settle_token_info = self.context.token(perp.market.settle_token_index);

        let health_remaining_ams = self
            .derive_liquidation_health_check_remaining_account_metas(liqee.1, &[], &[])
            .await
            .unwrap();

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::PerpLiqBaseOrPositivePnl {
                        group: self.group(),
                        perp_market: perp.address,
                        oracle: perp.market.oracle,
                        liqor: self.mango_account_address,
                        liqor_owner: self.owner(),
                        liqee: *liqee.0,
                        settle_bank: settle_token_info.mint_info.first_bank(),
                        settle_vault: settle_token_info.mint_info.first_vault(),
                        settle_oracle: settle_token_info.mint_info.oracle,
                    },
                    None,
                );
                ams.extend(health_remaining_ams.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(
                &mango_v4::instruction::PerpLiqBaseOrPositivePnl {
                    max_base_transfer,
                    max_pnl_transfer,
                },
            ),
        };
        Ok(ix)
    }

    pub async fn perp_liq_negative_pnl_or_bankruptcy_instruction(
        &self,
        liqee: (&Pubkey, &MangoAccountValue),
        market_index: PerpMarketIndex,
        max_liab_transfer: u64,
    ) -> anyhow::Result<Instruction> {
        let group = account_fetcher_fetch_anchor_account::<Group>(
            &*self.account_fetcher,
            &self.context.group,
        )
        .await?;

        let perp = self.context.perp(market_index);
        let settle_token_info = self.context.token(perp.market.settle_token_index);
        let insurance_token_info = self.context.token(INSURANCE_TOKEN_INDEX);

        let health_remaining_ams = self
            .derive_liquidation_health_check_remaining_account_metas(
                liqee.1,
                &[INSURANCE_TOKEN_INDEX],
                &[],
            )
            .await
            .unwrap();

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::PerpLiqNegativePnlOrBankruptcyV2 {
                        group: self.group(),
                        perp_market: perp.address,
                        oracle: perp.market.oracle,
                        liqor: self.mango_account_address,
                        liqor_owner: self.owner(),
                        liqee: *liqee.0,
                        settle_bank: settle_token_info.mint_info.first_bank(),
                        settle_vault: settle_token_info.mint_info.first_vault(),
                        settle_oracle: settle_token_info.mint_info.oracle,
                        insurance_vault: group.insurance_vault,
                        insurance_bank: insurance_token_info.mint_info.first_bank(),
                        insurance_bank_vault: insurance_token_info.mint_info.first_vault(),
                        insurance_oracle: insurance_token_info.mint_info.oracle,
                        token_program: Token::id(),
                    },
                    None,
                );
                ams.extend(health_remaining_ams.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(
                &mango_v4::instruction::PerpLiqNegativePnlOrBankruptcyV2 { max_liab_transfer },
            ),
        };
        Ok(ix)
    }

    //
    // Liquidation
    //

    pub async fn token_liq_with_token_instruction(
        &self,
        liqee: (&Pubkey, &MangoAccountValue),
        asset_token_index: TokenIndex,
        liab_token_index: TokenIndex,
        max_liab_transfer: I80F48,
    ) -> anyhow::Result<Instruction> {
        let health_remaining_ams = self
            .derive_liquidation_health_check_remaining_account_metas(
                liqee.1,
                &[],
                &[asset_token_index, liab_token_index],
            )
            .await
            .unwrap();

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::TokenLiqWithToken {
                        group: self.group(),
                        liqee: *liqee.0,
                        liqor: self.mango_account_address,
                        liqor_owner: self.owner(),
                    },
                    None,
                );
                ams.extend(health_remaining_ams);
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::TokenLiqWithToken {
                asset_token_index,
                liab_token_index,
                max_liab_transfer,
            }),
        };
        Ok(ix)
    }

    pub async fn token_liq_bankruptcy_instruction(
        &self,
        liqee: (&Pubkey, &MangoAccountValue),
        liab_token_index: TokenIndex,
        max_liab_transfer: I80F48,
    ) -> anyhow::Result<Instruction> {
        let quote_token_index = 0;

        let quote_info = self.context.token(quote_token_index);
        let liab_info = self.context.token(liab_token_index);

        let bank_remaining_ams = liab_info
            .mint_info
            .banks()
            .iter()
            .map(|bank_pubkey| util::to_writable_account_meta(*bank_pubkey))
            .collect::<Vec<_>>();

        let health_remaining_ams = self
            .derive_liquidation_health_check_remaining_account_metas(
                liqee.1,
                &[INSURANCE_TOKEN_INDEX],
                &[quote_token_index, liab_token_index],
            )
            .await
            .unwrap();

        let group = account_fetcher_fetch_anchor_account::<Group>(
            &*self.account_fetcher,
            &self.context.group,
        )
        .await?;

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::TokenLiqBankruptcy {
                        group: self.group(),
                        liqee: *liqee.0,
                        liqor: self.mango_account_address,
                        liqor_owner: self.owner(),
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
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::TokenLiqBankruptcy {
                max_liab_transfer,
            }),
        };
        Ok(ix)
    }

    pub async fn token_conditional_swap_trigger_instruction(
        &self,
        liqee: (&Pubkey, &MangoAccountValue),
        token_conditional_swap_id: u64,
        max_buy_token_to_liqee: u64,
        max_sell_token_to_liqor: u64,
        extra_affected_tokens: &[TokenIndex],
    ) -> anyhow::Result<Instruction> {
        let (tcs_index, tcs) = liqee
            .1
            .token_conditional_swap_by_id(token_conditional_swap_id)?;

        let affected_tokens = extra_affected_tokens
            .iter()
            .chain(&[tcs.buy_token_index, tcs.sell_token_index])
            .copied()
            .collect_vec();
        let health_remaining_ams = self
            .derive_liquidation_health_check_remaining_account_metas(
                liqee.1,
                &affected_tokens,
                &[tcs.buy_token_index, tcs.sell_token_index],
            )
            .await
            .unwrap();

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::TokenConditionalSwapTrigger {
                        group: self.group(),
                        liqee: *liqee.0,
                        liqor: self.mango_account_address,
                        liqor_authority: self.owner(),
                    },
                    None,
                );
                ams.extend(health_remaining_ams);
                ams
            },
            data: anchor_lang::InstructionData::data(
                &mango_v4::instruction::TokenConditionalSwapTrigger {
                    token_conditional_swap_id,
                    token_conditional_swap_index: tcs_index.try_into().unwrap(),
                    max_buy_token_to_liqee,
                    max_sell_token_to_liqor,
                },
            ),
        };
        Ok(ix)
    }

    // health region

    pub fn health_region_begin_instruction(
        &self,
        account: &MangoAccountValue,
        affected_tokens: Vec<TokenIndex>,
        writable_banks: Vec<TokenIndex>,
        affected_perp_markets: Vec<PerpMarketIndex>,
    ) -> anyhow::Result<Instruction> {
        let health_remaining_metas = self.context.derive_health_check_remaining_account_metas(
            account,
            affected_tokens,
            writable_banks,
            affected_perp_markets,
        )?;

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::HealthRegionBegin {
                        group: self.group(),
                        account: self.mango_account_address,
                        instructions: solana_sdk::sysvar::instructions::id(),
                    },
                    None,
                );
                ams.extend(health_remaining_metas.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::HealthRegionBegin {}),
        };

        Ok(ix)
    }

    pub fn health_region_end_instruction(
        &self,
        account: &MangoAccountValue,
        affected_tokens: Vec<TokenIndex>,
        writable_banks: Vec<TokenIndex>,
        affected_perp_markets: Vec<PerpMarketIndex>,
    ) -> anyhow::Result<Instruction> {
        let health_remaining_metas = self.context.derive_health_check_remaining_account_metas(
            account,
            affected_tokens,
            writable_banks,
            affected_perp_markets,
        )?;

        let ix = Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::HealthRegionEnd {
                        account: self.mango_account_address,
                    },
                    None,
                );
                ams.extend(health_remaining_metas.into_iter());
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::HealthRegionEnd {}),
        };

        Ok(ix)
    }

    // jupiter

    pub fn jupiter_v4(&self) -> jupiter::v4::JupiterV4 {
        jupiter::v4::JupiterV4 { mango_client: self }
    }

    pub fn jupiter_v6(&self) -> jupiter::v6::JupiterV6 {
        jupiter::v6::JupiterV6 { mango_client: self }
    }

    pub fn jupiter(&self) -> jupiter::Jupiter {
        jupiter::Jupiter { mango_client: self }
    }

    pub async fn fetch_address_lookup_table(
        &self,
        address: Pubkey,
    ) -> anyhow::Result<AddressLookupTableAccount> {
        let raw = self
            .account_fetcher
            .fetch_raw_account_lookup_table(&address)
            .await?;
        let data = AddressLookupTable::deserialize(&raw.data())?;
        Ok(AddressLookupTableAccount {
            key: address,
            addresses: data.addresses.to_vec(),
        })
    }

    pub async fn fetch_address_lookup_tables(
        &self,
        alts: impl Iterator<Item = &Pubkey>,
    ) -> anyhow::Result<Vec<AddressLookupTableAccount>> {
        stream::iter(alts)
            .then(|a| self.fetch_address_lookup_table(*a))
            .try_collect::<Vec<_>>()
            .await
    }

    pub async fn mango_address_lookup_tables(
        &self,
    ) -> anyhow::Result<Vec<AddressLookupTableAccount>> {
        stream::iter(self.context.address_lookup_tables.iter())
            .then(|&k| self.fetch_address_lookup_table(k))
            .try_collect::<Vec<_>>()
            .await
    }

    pub(crate) async fn deserialize_instructions_and_alts(
        &self,
        message: &solana_sdk::message::VersionedMessage,
    ) -> anyhow::Result<(Vec<Instruction>, Vec<AddressLookupTableAccount>)> {
        let lookups = message.address_table_lookups().unwrap_or_default();
        let address_lookup_tables = self
            .fetch_address_lookup_tables(lookups.iter().map(|a| &a.account_key))
            .await?;

        let mut account_keys = message.static_account_keys().to_vec();
        for (lookups, table) in lookups.iter().zip(address_lookup_tables.iter()) {
            account_keys.extend(
                lookups
                    .writable_indexes
                    .iter()
                    .map(|&index| table.addresses[index as usize]),
            );
        }
        for (lookups, table) in lookups.iter().zip(address_lookup_tables.iter()) {
            account_keys.extend(
                lookups
                    .readonly_indexes
                    .iter()
                    .map(|&index| table.addresses[index as usize]),
            );
        }

        let compiled_ix = message
            .instructions()
            .iter()
            .map(|ci| solana_sdk::instruction::Instruction {
                program_id: *ci.program_id(&account_keys),
                accounts: ci
                    .accounts
                    .iter()
                    .map(|&index| AccountMeta {
                        pubkey: account_keys[index as usize],
                        is_signer: message.is_signer(index.into()),
                        is_writable: message.is_maybe_writable(index.into()),
                    })
                    .collect(),
                data: ci.data.clone(),
            })
            .collect();

        Ok((compiled_ix, address_lookup_tables))
    }

    pub async fn send_and_confirm_owner_tx(
        &self,
        instructions: Vec<Instruction>,
    ) -> anyhow::Result<Signature> {
        TransactionBuilder {
            instructions,
            address_lookup_tables: self.mango_address_lookup_tables().await?,
            payer: self.client.fee_payer.pubkey(),
            signers: vec![self.owner.clone(), self.client.fee_payer.clone()],
            config: self.client.transaction_builder_config,
        }
        .send_and_confirm(&self.client)
        .await
    }

    pub async fn send_and_confirm_permissionless_tx(
        &self,
        instructions: Vec<Instruction>,
    ) -> anyhow::Result<Signature> {
        TransactionBuilder {
            instructions,
            address_lookup_tables: self.mango_address_lookup_tables().await?,
            payer: self.client.fee_payer.pubkey(),
            signers: vec![self.client.fee_payer.clone()],
            config: self.client.transaction_builder_config,
        }
        .send_and_confirm(&self.client)
        .await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MangoClientError {
    #[error("Transaction simulation error. Error: {err:?}, Logs: {}",
        .logs.iter().join("; ")
    )]
    SendTransactionPreflightFailure {
        err: Option<TransactionError>,
        logs: Vec<String>,
    },
}

#[derive(Clone, Debug)]
pub struct TransactionSize {
    pub accounts: usize,
    pub length: usize,
}

impl TransactionSize {
    pub fn is_ok(&self) -> bool {
        let limit = Self::limit();
        self.length <= limit.length && self.accounts <= limit.accounts
    }

    pub fn limit() -> Self {
        Self {
            accounts: MAX_ACCOUNTS_PER_TRANSACTION,
            length: solana_sdk::packet::PACKET_DATA_SIZE,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TransactionBuilderConfig {
    // adds a SetComputeUnitPrice instruction in front
    pub prioritization_micro_lamports: Option<u64>,
}

pub struct TransactionBuilder {
    pub instructions: Vec<Instruction>,
    pub address_lookup_tables: Vec<AddressLookupTableAccount>,
    pub signers: Vec<Arc<Keypair>>,
    pub payer: Pubkey,
    pub config: TransactionBuilderConfig,
}

impl TransactionBuilder {
    pub async fn transaction(
        &self,
        rpc: &RpcClientAsync,
    ) -> anyhow::Result<solana_sdk::transaction::VersionedTransaction> {
        let latest_blockhash = rpc.get_latest_blockhash().await?;
        self.transaction_with_blockhash(latest_blockhash)
    }

    pub fn transaction_with_blockhash(
        &self,
        blockhash: Hash,
    ) -> anyhow::Result<solana_sdk::transaction::VersionedTransaction> {
        let mut ix = self.instructions.clone();
        if let Some(prio_price) = self.config.prioritization_micro_lamports {
            ix.insert(
                0,
                solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(
                    prio_price,
                ),
            )
        }
        let v0_message = solana_sdk::message::v0::Message::try_compile(
            &self.payer,
            &ix,
            &self.address_lookup_tables,
            blockhash,
        )?;
        let versioned_message = solana_sdk::message::VersionedMessage::V0(v0_message);
        let signers = self
            .signers
            .iter()
            .unique_by(|s| s.pubkey())
            .map(|v| v.deref())
            .collect::<Vec<_>>();
        let tx =
            solana_sdk::transaction::VersionedTransaction::try_new(versioned_message, &signers)?;
        Ok(tx)
    }

    // These two send() functions don't really belong into the transaction builder!

    pub async fn send(&self, client: &Client) -> anyhow::Result<Signature> {
        let rpc = client.rpc_async();
        let tx = self.transaction(&rpc).await?;
        rpc.send_transaction_with_config(&tx, client.rpc_send_transaction_config)
            .await
            .map_err(prettify_solana_client_error)
    }

    pub async fn send_and_confirm(&self, client: &Client) -> anyhow::Result<Signature> {
        let rpc = client.rpc_async();
        let tx = self.transaction(&rpc).await?;
        // TODO: Wish we could use client.rpc_send_transaction_config here too!
        rpc.send_and_confirm_transaction(&tx)
            .await
            .map_err(prettify_solana_client_error)
    }

    pub fn transaction_size(&self) -> anyhow::Result<TransactionSize> {
        let tx = self.transaction_with_blockhash(solana_sdk::hash::Hash::default())?;
        let bytes = bincode::serialize(&tx)?;
        let accounts = tx.message.static_account_keys().len()
            + tx.message
                .address_table_lookups()
                .map(|alts| {
                    alts.iter()
                        .map(|alt| alt.readonly_indexes.len() + alt.writable_indexes.len())
                        .sum()
                })
                .unwrap_or(0);
        Ok(TransactionSize {
            accounts,
            length: bytes.len(),
        })
    }
}

/// Do some manual unpacking on some ClientErrors
///
/// Unfortunately solana's RpcResponseError will very unhelpfully print [N log messages]
/// instead of showing the actual log messages. This unpacks the error to provide more useful
/// output.
pub fn prettify_client_error(err: anchor_client::ClientError) -> anyhow::Error {
    match err {
        anchor_client::ClientError::SolanaClientError(c) => prettify_solana_client_error(c),
        _ => err.into(),
    }
}

pub fn prettify_solana_client_error(
    err: solana_client::client_error::ClientError,
) -> anyhow::Error {
    use solana_client::client_error::ClientErrorKind;
    use solana_client::rpc_request::{RpcError, RpcResponseErrorData};
    match err.kind() {
        ClientErrorKind::RpcError(RpcError::RpcResponseError { data, .. }) => match data {
            RpcResponseErrorData::SendTransactionPreflightFailure(s) => {
                return MangoClientError::SendTransactionPreflightFailure {
                    err: s.err.clone(),
                    logs: s.logs.clone().unwrap_or_default(),
                }
                .into();
            }
            _ => {}
        },
        _ => {}
    };
    err.into()
}

#[derive(Clone, Copy)]
pub enum JupiterSwapMode {
    ExactIn,
    ExactOut,
}

pub fn keypair_from_cli(keypair: &str) -> Keypair {
    let maybe_keypair = keypair::read_keypair(&mut keypair.as_bytes());
    match maybe_keypair {
        Ok(keypair) => keypair,
        Err(_) => {
            let path = std::path::PathBuf::from_str(&*shellexpand::tilde(keypair)).unwrap();
            keypair::read_keypair_file(path)
                .unwrap_or_else(|_| panic!("Failed to read keypair from {}", keypair))
        }
    }
}

pub fn pubkey_from_cli(pubkey: &str) -> Pubkey {
    match Pubkey::from_str(pubkey) {
        Ok(p) => p,
        Err(_) => keypair_from_cli(pubkey).pubkey(),
    }
}
