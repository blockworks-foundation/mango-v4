#![allow(dead_code)]

use std::cell::RefCell;
use std::str::FromStr;
use std::sync::Arc;

use solana_program::{program_option::COption, program_pack::Pack};
use solana_program_test::*;
use solana_sdk::pubkey::Pubkey;
use spl_token::{state::*, *};

pub use cookies::*;
pub use mango_client::*;
pub use serum::*;
pub use solana::*;
pub use utils::*;

pub mod cookies;
pub mod mango_client;
pub mod mango_setup;
pub mod serum;
pub mod solana;
pub mod utils;

trait AddPacked {
    fn add_packable_account<T: Pack>(
        &mut self,
        pubkey: Pubkey,
        amount: u64,
        data: &T,
        owner: &Pubkey,
    );
}

impl AddPacked for ProgramTest {
    fn add_packable_account<T: Pack>(
        &mut self,
        pubkey: Pubkey,
        amount: u64,
        data: &T,
        owner: &Pubkey,
    ) {
        let mut account = solana_sdk::account::Account::new(amount, T::get_packed_len(), owner);
        data.pack_into_slice(&mut account.data);
        self.add_account(pubkey, account);
    }
}

pub struct MarginTradeCookie {
    pub program: Pubkey,
    pub token_account: TestKeypair,
    pub token_account_owner: Pubkey,
    pub token_account_bump: u8,
}

pub struct TestContextBuilder {
    test: ProgramTest,
    mint0: Pubkey,
}

impl TestContextBuilder {
    pub fn new() -> Self {
        // We need to intercept logs to capture program log output
        let log_filter = "solana_rbpf=trace,\
                    solana_runtime::message_processor=debug,\
                    solana_runtime::system_instruction_processor=trace,\
                    solana_program_test=info";
        let env_logger =
            env_logger::Builder::from_env(env_logger::Env::new().default_filter_or(log_filter))
                .format_timestamp_nanos()
                .build();
        let _ = log::set_boxed_logger(Box::new(env_logger));

        let mut test = ProgramTest::new("mango_v4", mango_v4::id(), processor!(mango_v4::entry));

        // intentionally set to as tight as possible, to catch potential problems early
        test.set_compute_max_units(80000);

        Self {
            test,
            mint0: Pubkey::new_unique(),
        }
    }

    pub fn test(&mut self) -> &mut ProgramTest {
        &mut self.test
    }

    pub fn create_mints(&mut self) -> Vec<MintCookie> {
        let mut mints: Vec<MintCookie> = vec![
            MintCookie {
                index: 0,
                decimals: 6,
                unit: 10u64.pow(6) as f64,
                base_lot: 100 as f64,
                quote_lot: 10 as f64,
                pubkey: self.mint0,
                authority: TestKeypair::new(),
            }, // symbol: "MNGO".to_string()
        ];
        for i in 1..10 {
            mints.push(MintCookie {
                index: i,
                decimals: 6,
                unit: 10u64.pow(6) as f64,
                base_lot: 100 as f64,
                quote_lot: 10 as f64,
                pubkey: Pubkey::default(),
                authority: TestKeypair::new(),
            });
        }
        // Add mints in loop
        for mint_index in 0..mints.len() {
            let mint_pk: Pubkey;
            if mints[mint_index].pubkey == Pubkey::default() {
                mint_pk = Pubkey::new_unique();
            } else {
                mint_pk = mints[mint_index].pubkey;
            }
            mints[mint_index].pubkey = mint_pk;

            self.test.add_packable_account(
                mint_pk,
                u32::MAX as u64,
                &Mint {
                    is_initialized: true,
                    mint_authority: COption::Some(mints[mint_index].authority.pubkey()),
                    decimals: mints[mint_index].decimals,
                    ..Mint::default()
                },
                &spl_token::id(),
            );
        }

        mints
    }

    pub fn create_users(&mut self, mints: &[MintCookie]) -> Vec<UserCookie> {
        let num_users = 4;
        let mut users = Vec::new();
        for _ in 0..num_users {
            let user_key = TestKeypair::new();
            self.test.add_account(
                user_key.pubkey(),
                solana_sdk::account::Account::new(
                    u32::MAX as u64,
                    0,
                    &solana_sdk::system_program::id(),
                ),
            );

            // give every user 10^18 (< 2^60) of every token
            // ~~ 1 trillion in case of 6 decimals
            let mut token_accounts = Vec::new();
            for mint_index in 0..mints.len() {
                let mint = mints[mint_index].pubkey;
                let token_key = anchor_spl::associated_token::get_associated_token_address(
                    &user_key.pubkey(),
                    &mint,
                );
                self.test.add_packable_account(
                    token_key,
                    u32::MAX as u64,
                    &spl_token::state::Account {
                        mint,
                        owner: user_key.pubkey(),
                        amount: 1_000_000_000_000_000_000,
                        state: spl_token::state::AccountState::Initialized,
                        ..spl_token::state::Account::default()
                    },
                    &spl_token::id(),
                );

                token_accounts.push(token_key);
            }
            users.push(UserCookie {
                key: user_key,
                token_accounts,
            });
        }

        users
    }

    pub fn add_serum_program(&mut self) -> Pubkey {
        let serum_program_id = Pubkey::new_unique();
        self.test.add_program("serum_dex", serum_program_id, None);
        serum_program_id
    }

    pub fn add_margin_trade_program(&mut self) -> MarginTradeCookie {
        let program = Pubkey::from_str("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix").unwrap();
        let token_account = TestKeypair::new();
        let (token_account_owner, token_account_bump) =
            Pubkey::find_program_address(&[b"MarginTrade"], &program);

        self.test
            .add_program("margin_trade", program, std::option::Option::None);
        self.test.add_packable_account(
            token_account.pubkey(),
            u32::MAX as u64,
            &Account {
                mint: self.mint0,
                owner: token_account_owner,
                amount: 0,
                state: AccountState::Initialized,
                is_native: COption::None,
                ..Account::default()
            },
            &spl_token::id(),
        );

        MarginTradeCookie {
            program,
            token_account,
            token_account_owner,
            token_account_bump,
        }
    }

    pub async fn start_default(mut self) -> TestContext {
        let mints = self.create_mints();
        let users = self.create_users(&mints);
        let serum_program_id = self.add_serum_program();

        let solana = self.start().await;

        let serum = Arc::new(SerumCookie {
            solana: solana.clone(),
            program_id: serum_program_id,
        });

        TestContext {
            solana: solana.clone(),
            mints,
            users,
            serum,
        }
    }

    pub async fn start(self) -> Arc<SolanaCookie> {
        let mut context = self.test.start_with_context().await;
        let rent = context.banks_client.get_rent().await.unwrap();

        let solana = Arc::new(SolanaCookie {
            context: RefCell::new(context),
            rent,
            last_transaction_log: RefCell::new(vec![]),
        });

        solana
    }
}

pub struct TestContext {
    pub solana: Arc<SolanaCookie>,
    pub mints: Vec<MintCookie>,
    pub users: Vec<UserCookie>,
    pub serum: Arc<SerumCookie>,
}

impl TestContext {
    pub async fn new() -> Self {
        TestContextBuilder::new().start_default().await
    }
}
