use std::cell::RefCell;
use std::{sync::Arc, sync::RwLock};

use log::*;
use solana_program::{program_option::COption, program_pack::Pack};
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
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

struct LoggerWrapper {
    inner: env_logger::Logger,
    program_log: Arc<RwLock<Vec<String>>>,
}

impl Log for LoggerWrapper {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if record
            .target()
            .starts_with("solana_runtime::message_processor")
        {
            let msg = record.args().to_string();
            if let Some(data) = msg.strip_prefix("Program log: ") {
                self.program_log.write().unwrap().push(data.into());
            }
        }
        self.inner.log(record);
    }

    fn flush(&self) {}
}

pub struct TestContext {
    pub solana: Arc<SolanaCookie>,
    pub mints: Vec<MintCookie>,
    pub users: Vec<UserCookie>,
    pub quote_index: usize,
    pub serum: Arc<SerumCookie>,
}

impl TestContext {
    pub async fn new(
        test_opt: Option<ProgramTest>,
        margin_trade_program_id: Option<&Pubkey>,
        margin_trade_token_account: Option<&Keypair>,
        mtta_owner: Option<&Pubkey>,
    ) -> Self {
        let mut test = if test_opt.is_some() {
            test_opt.unwrap()
        } else {
            ProgramTest::new("mango_v4", mango_v4::id(), processor!(mango_v4::entry))
        };

        let serum_program_id = anchor_spl::dex::id();
        test.add_program("serum_dex", serum_program_id, None);

        // We need to intercept logs to capture program log output
        let log_filter = "solana_rbpf=trace,\
                    solana_runtime::message_processor=debug,\
                    solana_runtime::system_instruction_processor=trace,\
                    solana_program_test=info";
        let env_logger =
            env_logger::Builder::from_env(env_logger::Env::new().default_filter_or(log_filter))
                .format_timestamp_nanos()
                .build();
        let program_log_capture = Arc::new(RwLock::new(vec![]));
        let _ = log::set_boxed_logger(Box::new(LoggerWrapper {
            inner: env_logger,
            program_log: program_log_capture.clone(),
        }));

        // intentionally set to half the limit, to catch potential problems early
        test.set_compute_max_units(100000);

        // Setup the environment

        // Mints
        let mut mints: Vec<MintCookie> = vec![
            MintCookie {
                index: 0,
                decimals: 6,
                unit: 10u64.pow(6) as f64,
                base_lot: 100 as f64,
                quote_lot: 10 as f64,
                pubkey: Pubkey::default(),
                authority: Keypair::new(),
            }, // symbol: "MNGO".to_string()
        ];
        for i in 1..10 {
            mints.push(MintCookie {
                index: i,
                decimals: 6,
                unit: 10u64.pow(6) as f64,
                base_lot: 0 as f64,
                quote_lot: 0 as f64,
                pubkey: Pubkey::default(),
                authority: Keypair::new(),
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

            test.add_packable_account(
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
        let quote_index = mints.len() - 1;

        // margin trade
        if margin_trade_program_id.is_some() {
            test.add_program(
                "margin_trade",
                *margin_trade_program_id.unwrap(),
                std::option::Option::None,
            );
            test.add_packable_account(
                margin_trade_token_account.unwrap().pubkey(),
                u32::MAX as u64,
                &Account {
                    mint: mints[0].pubkey,
                    owner: *mtta_owner.unwrap(),
                    amount: 0,
                    state: AccountState::Initialized,
                    is_native: COption::None,
                    ..Account::default()
                },
                &spl_token::id(),
            );
        }

        // Users
        let num_users = 4;
        let mut users = Vec::new();
        for _ in 0..num_users {
            let user_key = Keypair::new();
            test.add_account(
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
                let token_key = Pubkey::new_unique();
                test.add_packable_account(
                    token_key,
                    u32::MAX as u64,
                    &spl_token::state::Account {
                        mint: mints[mint_index].pubkey,
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

        let mut context = test.start_with_context().await;
        let rent = context.banks_client.get_rent().await.unwrap();

        let solana = Arc::new(SolanaCookie {
            context: RefCell::new(context),
            rent,
            program_log: program_log_capture.clone(),
        });

        let serum = Arc::new(SerumCookie {
            solana: solana.clone(),
            program_id: serum_program_id,
        });

        TestContext {
            solana: solana.clone(),
            mints,
            users,
            quote_index,
            serum,
        }
    }
}
