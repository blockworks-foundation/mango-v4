#![allow(dead_code)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::{self, SysvarId};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Token, TokenAccount};
use fixed::types::I80F48;
use itertools::Itertools;
use mango_v4::accounts_ix::{
    InterestRateParams, Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side,
};
use mango_v4::state::{MangoAccount, MangoAccountValue};
use solana_program::instruction::Instruction;
use solana_program_test::{BanksClientError, BanksTransactionResultWithMetadata};
use solana_sdk::instruction;
use solana_sdk::transport::TransportError;
use std::sync::Arc;

use super::solana::SolanaCookie;
use super::utils::TestKeypair;
use mango_v4::state::*;

#[async_trait::async_trait(?Send)]
pub trait ClientAccountLoader {
    async fn load_bytes(&self, pubkey: &Pubkey) -> Option<Vec<u8>>;
    async fn load<T: AccountDeserialize>(&self, pubkey: &Pubkey) -> Option<T> {
        let bytes = self.load_bytes(pubkey).await?;
        AccountDeserialize::try_deserialize(&mut &bytes[..]).ok()
    }
    async fn load_mango_account(&self, pubkey: &Pubkey) -> Option<MangoAccountValue> {
        self.load_bytes(pubkey)
            .await
            .map(|v| MangoAccountValue::from_bytes(&v[8..]).unwrap())
    }
}

#[async_trait::async_trait(?Send)]
impl ClientAccountLoader for &SolanaCookie {
    async fn load_bytes(&self, pubkey: &Pubkey) -> Option<Vec<u8>> {
        self.get_account_data(*pubkey).await
    }
}

// This will return a failure if the tx resulted in an error
pub async fn send_tx<CI: ClientInstruction>(
    solana: &SolanaCookie,
    ix: CI,
) -> std::result::Result<CI::Accounts, TransportError> {
    let (accounts, instruction) = ix.to_instruction(solana).await;
    let signers = ix.signers();
    let instructions = vec![instruction.clone()];
    let result = solana
        .process_transaction(&instructions, Some(&signers[..]))
        .await?;
    result.result?;
    Ok(accounts)
}

// This will return a failure if the tx resulted in an error
pub async fn send_tx_with_extra_accounts<CI: ClientInstruction>(
    solana: &SolanaCookie,
    ix: CI,
    account_metas: Vec<AccountMeta>,
) -> std::result::Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let (_, mut instruction) = ix.to_instruction(solana).await;
    instruction.accounts.extend(account_metas);
    let signers = ix.signers();
    let instructions = vec![instruction.clone()];
    solana
        .process_transaction(&instructions, Some(&signers[..]))
        .await
}

// This will return success even if the tx failed to finish
pub async fn send_tx_get_metadata<CI: ClientInstruction>(
    solana: &SolanaCookie,
    ix: CI,
) -> std::result::Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let (_, instruction) = ix.to_instruction(solana).await;
    let signers = ix.signers();
    let instructions = vec![instruction];
    solana
        .process_transaction(&instructions, Some(&signers[..]))
        .await
}

#[macro_export]
macro_rules! send_tx_expect_error {
    ($solana:expr, $ix:expr, $err:expr $(,)?) => {
        let result = send_tx($solana, $ix).await;
        let expected_err: u32 = $err.into();
        match result {
            Ok(_) => assert!(false, "no error returned"),
            Err(TransportError::TransactionError(
                solana_sdk::transaction::TransactionError::InstructionError(
                    _,
                    solana_program::instruction::InstructionError::Custom(err_num),
                ),
            )) => {
                assert_eq!(err_num, expected_err, "wrong error code");
            }
            _ => assert!(false, "not a mango error"),
        }
    };
}
pub use send_tx_expect_error;

/// Build a transaction from multiple instructions
pub struct ClientTransaction {
    solana: Arc<SolanaCookie>,
    instructions: Vec<instruction::Instruction>,
    signers: Vec<TestKeypair>,
}

impl<'a> ClientTransaction {
    pub fn new(solana: &Arc<SolanaCookie>) -> Self {
        Self {
            solana: solana.clone(),
            instructions: vec![],
            signers: vec![],
        }
    }

    pub async fn add_instruction<CI: ClientInstruction>(&mut self, ix: CI) -> CI::Accounts {
        let solana: &SolanaCookie = &self.solana;
        let (accounts, instruction) = ix.to_instruction(solana).await;
        self.instructions.push(instruction);
        self.signers.extend(ix.signers());
        accounts
    }

    pub fn add_instruction_direct(&mut self, ix: instruction::Instruction) {
        self.instructions.push(ix);
    }

    pub fn add_signer(&mut self, keypair: TestKeypair) {
        self.signers.push(keypair);
    }

    // Fails on tx error
    pub async fn send(&self) -> std::result::Result<(), BanksClientError> {
        let tx_result = self
            .solana
            .process_transaction(&self.instructions, Some(&self.signers))
            .await?;
        tx_result.result?;
        Ok(())
    }

    pub async fn send_expect_error(
        &self,
        error: mango_v4::error::MangoError,
    ) -> std::result::Result<(), BanksClientError> {
        let tx_result = self
            .solana
            .process_transaction(&self.instructions, Some(&self.signers))
            .await?;
        match tx_result.result {
            Ok(_) => assert!(false, "no error returned"),
            Err(solana_sdk::transaction::TransactionError::InstructionError(
                _,
                solana_program::instruction::InstructionError::Custom(err_num),
            )) => {
                let expected_err: u32 = error.into();
                assert_eq!(err_num, expected_err, "wrong error code");
            }
            _ => assert!(false, "not a mango error"),
        }
        Ok(())
    }

    // Tx error still returns success
    pub async fn send_get_metadata(
        &self,
    ) -> std::result::Result<BanksTransactionResultWithMetadata, BanksClientError> {
        self.solana
            .process_transaction(&self.instructions, Some(&self.signers))
            .await
    }
}

#[async_trait::async_trait(?Send)]
pub trait ClientInstruction {
    type Accounts: anchor_lang::ToAccountMetas;
    type Instruction: anchor_lang::InstructionData;

    async fn to_instruction(
        &self,
        loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction);
    fn signers(&self) -> Vec<TestKeypair>;
}

fn make_instruction(
    program_id: Pubkey,
    accounts: &impl anchor_lang::ToAccountMetas,
    data: &impl anchor_lang::InstructionData,
) -> instruction::Instruction {
    instruction::Instruction {
        program_id,
        accounts: anchor_lang::ToAccountMetas::to_account_metas(accounts, None),
        data: anchor_lang::InstructionData::data(data),
    }
}

async fn get_mint_info_by_mint(
    account_loader: &impl ClientAccountLoader,
    account: &MangoAccountValue,
    mint: Pubkey,
) -> MintInfo {
    let mint_info_pk = Pubkey::find_program_address(
        &[
            b"MintInfo".as_ref(),
            account.fixed.group.as_ref(),
            mint.as_ref(),
        ],
        &mango_v4::id(),
    )
    .0;
    account_loader.load(&mint_info_pk).await.unwrap()
}

async fn get_mint_info_by_token_index(
    account_loader: &impl ClientAccountLoader,
    account: &MangoAccountValue,
    token_index: TokenIndex,
) -> MintInfo {
    let bank_pk = Pubkey::find_program_address(
        &[
            b"Bank".as_ref(),
            account.fixed.group.as_ref(),
            &token_index.to_le_bytes(),
            &0u32.to_le_bytes(),
        ],
        &mango_v4::id(),
    )
    .0;
    let bank: Bank = account_loader.load(&bank_pk).await.unwrap();
    get_mint_info_by_mint(account_loader, account, bank.mint).await
}

fn get_perp_market_address_by_index(group: Pubkey, perp_market_index: PerpMarketIndex) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"PerpMarket".as_ref(),
            group.as_ref(),
            &perp_market_index.to_le_bytes(),
        ],
        &mango_v4::id(),
    )
    .0
}

async fn get_oracle_address_from_perp_market_address(
    account_loader: &impl ClientAccountLoader,
    perp_market_address: &Pubkey,
) -> Pubkey {
    let perp_market: PerpMarket = account_loader.load(&perp_market_address).await.unwrap();
    perp_market.oracle
}

// all the accounts that instructions like deposit/withdraw need to compute account health
async fn derive_health_check_remaining_account_metas(
    account_loader: &impl ClientAccountLoader,
    account: &MangoAccountValue,
    affected_bank: Option<Pubkey>,
    writable_banks: bool,
    affected_perp_market_index: Option<PerpMarketIndex>,
) -> Vec<AccountMeta> {
    let mut adjusted_account = account.clone();
    if let Some(affected_bank) = affected_bank {
        let bank: Bank = account_loader.load(&affected_bank).await.unwrap();
        adjusted_account
            .ensure_token_position(bank.token_index)
            .unwrap();
    }
    if let Some(affected_perp_market_index) = affected_perp_market_index {
        let pm: PerpMarket = account_loader
            .load(&get_perp_market_address_by_index(
                account.fixed.group,
                affected_perp_market_index,
            ))
            .await
            .unwrap();
        adjusted_account
            .ensure_perp_position(affected_perp_market_index, pm.settle_token_index)
            .unwrap();
    }

    // figure out all the banks/oracles that need to be passed for the health check
    let mut banks = vec![];
    let mut oracles = vec![];
    for position in adjusted_account.active_token_positions() {
        let mint_info =
            get_mint_info_by_token_index(account_loader, account, position.token_index).await;
        banks.push(mint_info.first_bank());
        oracles.push(mint_info.oracle);
    }

    let perp_markets = adjusted_account
        .active_perp_positions()
        .map(|perp| get_perp_market_address_by_index(account.fixed.group, perp.market_index));

    let mut perp_oracles = vec![];
    for perp in adjusted_account
        .active_perp_positions()
        .map(|perp| get_perp_market_address_by_index(account.fixed.group, perp.market_index))
    {
        perp_oracles.push(get_oracle_address_from_perp_market_address(account_loader, &perp).await)
    }

    let serum_oos = account.active_serum3_orders().map(|&s| s.open_orders);

    let to_account_meta = |pubkey| AccountMeta {
        pubkey,
        is_writable: false,
        is_signer: false,
    };

    banks
        .iter()
        .map(|&pubkey| AccountMeta {
            pubkey,
            is_writable: writable_banks,
            is_signer: false,
        })
        .chain(oracles.into_iter().map(to_account_meta))
        .chain(perp_markets.map(to_account_meta))
        .chain(perp_oracles.into_iter().map(to_account_meta))
        .chain(serum_oos.map(to_account_meta))
        .collect()
}

async fn derive_liquidation_remaining_account_metas(
    account_loader: &impl ClientAccountLoader,
    liqee: &MangoAccountValue,
    liqor: &MangoAccountValue,
    asset_token_index: TokenIndex,
    asset_bank_index: usize,
    liab_token_index: TokenIndex,
    liab_bank_index: usize,
) -> Vec<AccountMeta> {
    let mut banks = vec![];
    let mut oracles = vec![];
    let token_indexes = liqee
        .active_token_positions()
        .chain(liqor.active_token_positions())
        .map(|ta| ta.token_index)
        .unique();
    for token_index in token_indexes {
        let mint_info = get_mint_info_by_token_index(account_loader, liqee, token_index).await;
        let (bank_index, writable_bank) = if token_index == asset_token_index {
            (asset_bank_index, true)
        } else if token_index == liab_token_index {
            (liab_bank_index, true)
        } else {
            (0, false)
        };
        banks.push((mint_info.banks[bank_index], writable_bank));
        oracles.push(mint_info.oracle);
    }

    let perp_markets: Vec<Pubkey> = liqee
        .active_perp_positions()
        .chain(liqor.active_perp_positions())
        .map(|perp| get_perp_market_address_by_index(liqee.fixed.group, perp.market_index))
        .unique()
        .collect();

    let mut perp_oracles = vec![];
    for &perp in &perp_markets {
        perp_oracles.push(get_oracle_address_from_perp_market_address(account_loader, &perp).await)
    }

    let serum_oos = liqee
        .active_serum3_orders()
        .chain(liqor.active_serum3_orders())
        .map(|&s| s.open_orders);

    let to_account_meta = |pubkey| AccountMeta {
        pubkey,
        is_writable: false,
        is_signer: false,
    };

    banks
        .iter()
        .map(|(pubkey, is_writable)| AccountMeta {
            pubkey: *pubkey,
            is_writable: *is_writable,
            is_signer: false,
        })
        .chain(oracles.into_iter().map(to_account_meta))
        .chain(perp_markets.into_iter().map(to_account_meta))
        .chain(perp_oracles.into_iter().map(to_account_meta))
        .chain(serum_oos.map(to_account_meta))
        .collect()
}

fn from_serum_style_pubkey(d: &[u64; 4]) -> Pubkey {
    let b: &[u8; 32] = bytemuck::cast_ref(d);
    Pubkey::from(*b)
}

pub async fn get_mango_account(solana: &SolanaCookie, account: Pubkey) -> MangoAccountValue {
    let bytes = solana.get_account_data(account).await.unwrap();
    MangoAccountValue::from_bytes(&bytes[8..]).unwrap()
}

pub async fn account_position(solana: &SolanaCookie, account: Pubkey, bank: Pubkey) -> i64 {
    let account_data = get_mango_account(solana, account).await;
    let bank_data: Bank = solana.get_account(bank).await;
    let native = account_data
        .token_position(bank_data.token_index)
        .unwrap()
        .native(&bank_data);
    native.round().to_num::<i64>()
}

pub async fn account_position_closed(solana: &SolanaCookie, account: Pubkey, bank: Pubkey) -> bool {
    let account_data = get_mango_account(solana, account).await;
    let bank_data: Bank = solana.get_account(bank).await;
    account_data.token_position(bank_data.token_index).is_err()
}

pub async fn account_position_f64(solana: &SolanaCookie, account: Pubkey, bank: Pubkey) -> f64 {
    let account_data = get_mango_account(solana, account).await;
    let bank_data: Bank = solana.get_account(bank).await;
    let native = account_data
        .token_position(bank_data.token_index)
        .unwrap()
        .native(&bank_data);
    native.to_num::<f64>()
}

pub async fn account_init_health(solana: &SolanaCookie, account: Pubkey) -> f64 {
    send_tx(solana, ComputeAccountDataInstruction { account })
        .await
        .unwrap();
    let health_data = solana
        .program_log_events::<mango_v4::events::MangoAccountData>()
        .pop()
        .unwrap();
    health_data.init_health.to_num::<f64>()
}

pub async fn account_maint_health(solana: &SolanaCookie, account: Pubkey) -> f64 {
    send_tx(solana, ComputeAccountDataInstruction { account })
        .await
        .unwrap();
    let health_data = solana
        .program_log_events::<mango_v4::events::MangoAccountData>()
        .pop()
        .unwrap();
    health_data.maint_health.to_num::<f64>()
}

// Verifies that the "post_health: ..." log emitted by the previous instruction
// matches the init health of the account.
pub async fn check_prev_instruction_post_health(solana: &SolanaCookie, account: Pubkey) {
    let logs = solana.program_log();
    let post_health_str = logs
        .iter()
        .find_map(|line| line.strip_prefix("Program log: post_init_health: "))
        .unwrap();
    let post_health = post_health_str.parse::<f64>().unwrap();

    send_tx(solana, ComputeAccountDataInstruction { account })
        .await
        .unwrap();

    let health_data = solana
        .program_log_events::<mango_v4::events::MangoAccountData>()
        .pop()
        .unwrap();
    assert_eq!(health_data.init_health.to_num::<f64>(), post_health);
}

pub async fn set_bank_stub_oracle_price(
    solana: &SolanaCookie,
    group: Pubkey,
    token: &super::mango_setup::Token,
    admin: TestKeypair,
    price: f64,
) {
    send_tx(
        solana,
        StubOracleSetInstruction {
            oracle: token.oracle,
            group,
            admin,
            mint: token.mint.pubkey,
            price,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        TokenResetStablePriceModel {
            group,
            admin,
            mint: token.mint.pubkey,
        },
    )
    .await
    .unwrap();
}

pub async fn set_perp_stub_oracle_price(
    solana: &SolanaCookie,
    group: Pubkey,
    perp_market: Pubkey,
    token: &super::mango_setup::Token,
    admin: TestKeypair,
    price: f64,
) {
    set_bank_stub_oracle_price(solana, group, token, admin, price).await;
    send_tx(
        solana,
        PerpResetStablePriceModel {
            group,
            admin,
            perp_market,
        },
    )
    .await
    .unwrap();
}

//
// a struct for each instruction along with its
// ClientInstruction impl
//

#[derive(Clone)]
pub struct FlashLoanPart {
    pub bank: Pubkey,
    pub token_account: Pubkey,
    pub withdraw_amount: u64,
}

pub struct FlashLoanBeginInstruction {
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub loans: Vec<FlashLoanPart>,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for FlashLoanBeginInstruction {
    type Accounts = mango_v4::accounts::FlashLoanBegin;
    type Instruction = mango_v4::instruction::FlashLoanBegin;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();

        let accounts = Self::Accounts {
            account: self.account,
            owner: self.owner.pubkey(),
            token_program: Token::id(),
            instructions: solana_program::sysvar::instructions::id(),
        };

        let instruction = Self::Instruction {
            loan_amounts: self
                .loans
                .iter()
                .map(|v| v.withdraw_amount)
                .collect::<Vec<u64>>(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        for loan in self.loans.iter() {
            instruction.accounts.push(AccountMeta {
                pubkey: loan.bank,
                is_writable: true,
                is_signer: false,
            });
        }
        for loan in self.loans.iter() {
            let bank: Bank = account_loader.load(&loan.bank).await.unwrap();
            instruction.accounts.push(AccountMeta {
                pubkey: bank.vault,
                is_writable: true,
                is_signer: false,
            });
        }
        for loan in self.loans.iter() {
            instruction.accounts.push(AccountMeta {
                pubkey: loan.token_account,
                is_writable: true,
                is_signer: false,
            });
        }
        instruction.accounts.push(AccountMeta {
            pubkey: account.fixed.group,
            is_writable: false,
            is_signer: false,
        });

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct FlashLoanSwapBeginInstruction {
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub in_bank: Pubkey,
    pub out_bank: Pubkey,
    pub in_loan: u64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for FlashLoanSwapBeginInstruction {
    type Accounts = mango_v4::accounts::FlashLoanSwapBegin;
    type Instruction = mango_v4::instruction::FlashLoanSwapBegin;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();

        let in_bank: Bank = account_loader.load(&self.in_bank).await.unwrap();
        let out_bank: Bank = account_loader.load(&self.out_bank).await.unwrap();
        let in_account = anchor_spl::associated_token::get_associated_token_address(
            &self.owner.pubkey(),
            &in_bank.mint,
        );
        let out_account = anchor_spl::associated_token::get_associated_token_address(
            &self.owner.pubkey(),
            &out_bank.mint,
        );

        let accounts = Self::Accounts {
            account: self.account,
            owner: self.owner.pubkey(),
            input_mint: in_bank.mint,
            output_mint: out_bank.mint,
            system_program: System::id(),
            token_program: Token::id(),
            associated_token_program: AssociatedToken::id(),
            instructions: solana_program::sysvar::instructions::id(),
        };

        let instruction = Self::Instruction {
            loan_amount: self.in_loan,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.push(AccountMeta {
            pubkey: self.in_bank,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: self.out_bank,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: in_bank.vault,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: out_bank.vault,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: in_account,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: out_account,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: account.fixed.group,
            is_writable: false,
            is_signer: false,
        });

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct FlashLoanEndInstruction {
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub loans: Vec<FlashLoanPart>,
    pub flash_loan_type: mango_v4::accounts_ix::FlashLoanType,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for FlashLoanEndInstruction {
    type Accounts = mango_v4::accounts::FlashLoanEnd;
    type Instruction = mango_v4::instruction::FlashLoanEndV2;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            num_loans: self.loans.len() as u8,
            flash_loan_type: self.flash_loan_type,
        };

        let mut account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        for loan in self.loans.iter() {
            let bank: Bank = account_loader.load(&loan.bank).await.unwrap();
            account.ensure_token_position(bank.token_index).unwrap();
        }

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            true,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            account: self.account,
            owner: self.owner.pubkey(),
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());
        for loan in self.loans.iter() {
            let bank: Bank = account_loader.load(&loan.bank).await.unwrap();
            instruction.accounts.push(AccountMeta {
                pubkey: bank.vault,
                is_writable: true,
                is_signer: false,
            });
        }
        for loan in self.loans.iter() {
            instruction.accounts.push(AccountMeta {
                pubkey: loan.token_account,
                is_writable: true,
                is_signer: false,
            });
        }
        instruction.accounts.push(AccountMeta {
            pubkey: account.fixed.group,
            is_writable: false,
            is_signer: false,
        });

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

#[derive(Clone)]
pub struct TokenWithdrawInstruction {
    pub amount: u64,
    pub allow_borrow: bool,

    pub account: Pubkey,
    pub owner: TestKeypair,
    pub token_account: Pubkey,
    pub bank_index: usize,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenWithdrawInstruction {
    type Accounts = mango_v4::accounts::TokenWithdraw;
    type Instruction = mango_v4::instruction::TokenWithdraw;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            amount: self.amount,
            allow_borrow: self.allow_borrow,
        };

        // load accounts, find PDAs, find remainingAccounts
        let token_account: TokenAccount = account_loader.load(&self.token_account).await.unwrap();
        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let mint_info = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                account.fixed.group.as_ref(),
                token_account.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let mint_info: MintInfo = account_loader.load(&mint_info).await.unwrap();

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            Some(mint_info.banks[self.bank_index]),
            false,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            owner: self.owner.pubkey(),
            bank: mint_info.banks[self.bank_index],
            vault: mint_info.vaults[self.bank_index],
            oracle: mint_info.oracle,
            token_account: self.token_account,
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

#[derive(Clone)]
pub struct TokenDepositInstruction {
    pub amount: u64,
    pub reduce_only: bool,
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub token_account: Pubkey,
    pub token_authority: TestKeypair,
    pub bank_index: usize,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenDepositInstruction {
    type Accounts = mango_v4::accounts::TokenDeposit;
    type Instruction = mango_v4::instruction::TokenDeposit;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            amount: self.amount,
            reduce_only: self.reduce_only,
        };

        // load account so we know its mint
        let token_account: TokenAccount = account_loader.load(&self.token_account).await.unwrap();
        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let mint_info = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                account.fixed.group.as_ref(),
                token_account.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let mint_info: MintInfo = account_loader.load(&mint_info).await.unwrap();

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            Some(mint_info.banks[self.bank_index]),
            false,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            owner: self.owner.pubkey(),
            bank: mint_info.banks[self.bank_index],
            vault: mint_info.vaults[self.bank_index],
            oracle: mint_info.oracle,
            token_account: self.token_account,
            token_authority: self.token_authority.pubkey(),
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.token_authority, self.owner]
    }
}

pub struct TokenDepositIntoExistingInstruction {
    pub amount: u64,
    pub reduce_only: bool,
    pub account: Pubkey,
    pub token_account: Pubkey,
    pub token_authority: TestKeypair,
    pub bank_index: usize,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenDepositIntoExistingInstruction {
    type Accounts = mango_v4::accounts::TokenDepositIntoExisting;
    type Instruction = mango_v4::instruction::TokenDepositIntoExisting;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            amount: self.amount,
            reduce_only: self.reduce_only,
        };

        // load account so we know its mint
        let token_account: TokenAccount = account_loader.load(&self.token_account).await.unwrap();
        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let mint_info = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                account.fixed.group.as_ref(),
                token_account.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let mint_info: MintInfo = account_loader.load(&mint_info).await.unwrap();

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            Some(mint_info.banks[self.bank_index]),
            false,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            bank: mint_info.banks[self.bank_index],
            vault: mint_info.vaults[self.bank_index],
            oracle: mint_info.oracle,
            token_account: self.token_account,
            token_authority: self.token_authority.pubkey(),
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.token_authority]
    }
}

pub struct TokenRegisterInstruction {
    pub token_index: TokenIndex,
    pub decimals: u8,
    pub adjustment_factor: f32,
    pub util0: f32,
    pub rate0: f32,
    pub util1: f32,
    pub rate1: f32,
    pub max_rate: f32,
    pub loan_origination_fee_rate: f32,
    pub loan_fee_rate: f32,
    pub maint_asset_weight: f32,
    pub init_asset_weight: f32,
    pub maint_liab_weight: f32,
    pub init_liab_weight: f32,
    pub liquidation_fee: f32,
    pub platform_liquidation_fee: f32,

    pub min_vault_to_deposits_ratio: f64,
    pub net_borrow_limit_per_window_quote: i64,
    pub net_borrow_limit_window_size_ts: u64,

    pub group: Pubkey,
    pub admin: TestKeypair,
    pub mint: Pubkey,
    pub oracle: Pubkey,
    pub payer: TestKeypair,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenRegisterInstruction {
    type Accounts = mango_v4::accounts::TokenRegister;
    type Instruction = mango_v4::instruction::TokenRegister;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            name: format!(
                "{}{}",
                "some_ticker".to_string(),
                self.token_index.to_string()
            ),
            token_index: self.token_index,
            oracle_config: OracleConfigParams {
                conf_filter: 0.1,
                max_staleness_slots: None,
            },
            interest_rate_params: InterestRateParams {
                adjustment_factor: self.adjustment_factor,
                util0: self.util0,
                rate0: self.rate0,
                util1: self.util1,
                rate1: self.rate1,
                max_rate: self.max_rate,
            },
            loan_fee_rate: self.loan_fee_rate,
            loan_origination_fee_rate: self.loan_origination_fee_rate,
            maint_asset_weight: self.maint_asset_weight,
            init_asset_weight: self.init_asset_weight,
            maint_liab_weight: self.maint_liab_weight,
            init_liab_weight: self.init_liab_weight,
            liquidation_fee: self.liquidation_fee,
            stable_price_delay_interval_seconds: StablePriceModel::default().delay_interval_seconds,
            stable_price_delay_growth_limit: StablePriceModel::default().delay_growth_limit,
            stable_price_growth_limit: StablePriceModel::default().stable_growth_limit,
            min_vault_to_deposits_ratio: self.min_vault_to_deposits_ratio,
            net_borrow_limit_per_window_quote: self.net_borrow_limit_per_window_quote,
            net_borrow_limit_window_size_ts: self.net_borrow_limit_window_size_ts,
            borrow_weight_scale_start_quote: f64::MAX,
            deposit_weight_scale_start_quote: f64::MAX,
            reduce_only: 0,
            token_conditional_swap_taker_fee_rate: 0.0,
            token_conditional_swap_maker_fee_rate: 0.0,
            flash_loan_swap_fee_rate: 0.0,
            interest_curve_scaling: 1.0,
            interest_target_utilization: 0.5,
            group_insurance_fund: true,
            deposit_limit: 0,
            zero_util_rate: 0.0,
            platform_liquidation_fee: self.platform_liquidation_fee,
            disable_asset_liquidation: false,
            collateral_fee_per_day: 0.0,
        };

        let bank = Pubkey::find_program_address(
            &[
                b"Bank".as_ref(),
                self.group.as_ref(),
                &self.token_index.to_le_bytes(),
                &0u32.to_le_bytes(),
            ],
            &program_id,
        )
        .0;
        let vault = Pubkey::find_program_address(
            &[
                b"Vault".as_ref(),
                self.group.as_ref(),
                &self.token_index.to_le_bytes(),
                &0u32.to_le_bytes(),
            ],
            &program_id,
        )
        .0;
        let mint_info = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                self.group.as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let fallback_oracle = Pubkey::default();

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            mint: self.mint,
            bank,
            vault,
            mint_info,
            oracle: self.oracle,
            fallback_oracle,
            payer: self.payer.pubkey(),
            token_program: Token::id(),
            system_program: System::id(),
            rent: sysvar::rent::Rent::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin, self.payer]
    }
}

pub struct TokenAddBankInstruction {
    pub token_index: TokenIndex,
    pub bank_num: u32,

    pub group: Pubkey,
    pub admin: TestKeypair,
    pub payer: TestKeypair,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenAddBankInstruction {
    type Accounts = mango_v4::accounts::TokenAddBank;
    type Instruction = mango_v4::instruction::TokenAddBank;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            token_index: self.token_index,
            bank_num: self.bank_num,
        };

        let existing_bank = Pubkey::find_program_address(
            &[
                b"Bank".as_ref(),
                self.group.as_ref(),
                &self.token_index.to_le_bytes(),
                &0u32.to_le_bytes(),
            ],
            &program_id,
        )
        .0;
        let bank = Pubkey::find_program_address(
            &[
                b"Bank".as_ref(),
                self.group.as_ref(),
                &self.token_index.to_le_bytes(),
                &self.bank_num.to_le_bytes(),
            ],
            &program_id,
        )
        .0;
        let vault = Pubkey::find_program_address(
            &[
                b"Vault".as_ref(),
                self.group.as_ref(),
                &self.token_index.to_le_bytes(),
                &self.bank_num.to_le_bytes(),
            ],
            &program_id,
        )
        .0;

        let existing_bank_data: Bank = account_loader.load(&existing_bank).await.unwrap();
        let mint = existing_bank_data.mint;

        let mint_info = Pubkey::find_program_address(
            &[b"MintInfo".as_ref(), self.group.as_ref(), mint.as_ref()],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            mint,
            existing_bank,
            bank,
            vault,
            mint_info,
            payer: self.payer.pubkey(),
            token_program: Token::id(),
            system_program: System::id(),
            rent: sysvar::rent::Rent::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin, self.payer]
    }
}

pub struct TokenDeregisterInstruction {
    pub admin: TestKeypair,
    pub payer: TestKeypair,
    pub group: Pubkey,
    pub mint_info: Pubkey,
    pub banks: Vec<Pubkey>,
    pub vaults: Vec<Pubkey>,
    pub dust_vault: Pubkey,
    pub token_index: TokenIndex,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenDeregisterInstruction {
    type Accounts = mango_v4::accounts::TokenDeregister;
    type Instruction = mango_v4::instruction::TokenDeregister;

    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let accounts = Self::Accounts {
            admin: self.admin.pubkey(),
            group: self.group,
            mint_info: self.mint_info,
            dust_vault: self.dust_vault,
            sol_destination: self.sol_destination,
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);

        let mut ams = self
            .banks
            .iter()
            .zip(self.vaults.iter())
            .filter(|(bank, _)| **bank != Pubkey::default())
            .map(|(bank, vault)| {
                vec![
                    AccountMeta {
                        pubkey: *bank,
                        is_signer: false,
                        is_writable: true,
                    },
                    AccountMeta {
                        pubkey: *vault,
                        is_signer: false,
                        is_writable: true,
                    },
                ]
            })
            .flat_map(|vec| vec.into_iter())
            .collect::<Vec<_>>();
        instruction.accounts.append(&mut ams);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub fn token_edit_instruction_default() -> mango_v4::instruction::TokenEdit {
    mango_v4::instruction::TokenEdit {
        oracle_opt: None,
        oracle_config_opt: None,
        group_insurance_fund_opt: None,
        interest_rate_params_opt: None,
        loan_fee_rate_opt: None,
        loan_origination_fee_rate_opt: None,
        maint_asset_weight_opt: None,
        init_asset_weight_opt: None,
        maint_liab_weight_opt: None,
        init_liab_weight_opt: None,
        liquidation_fee_opt: None,
        stable_price_delay_interval_seconds_opt: None,
        stable_price_delay_growth_limit_opt: None,
        stable_price_growth_limit_opt: None,
        min_vault_to_deposits_ratio_opt: None,
        net_borrow_limit_per_window_quote_opt: None,
        net_borrow_limit_window_size_ts_opt: None,
        borrow_weight_scale_start_quote_opt: None,
        deposit_weight_scale_start_quote_opt: None,
        reset_stable_price: false,
        reset_net_borrow_limit: false,
        reduce_only_opt: None,
        name_opt: None,
        force_close_opt: None,
        token_conditional_swap_taker_fee_rate_opt: None,
        token_conditional_swap_maker_fee_rate_opt: None,
        flash_loan_swap_fee_rate_opt: None,
        interest_curve_scaling_opt: None,
        interest_target_utilization_opt: None,
        maint_weight_shift_start_opt: None,
        maint_weight_shift_end_opt: None,
        maint_weight_shift_asset_target_opt: None,
        maint_weight_shift_liab_target_opt: None,
        maint_weight_shift_abort: false,
        set_fallback_oracle: false,
        deposit_limit_opt: None,
        zero_util_rate_opt: None,
        platform_liquidation_fee_opt: None,
        disable_asset_liquidation_opt: None,
        collateral_fee_per_day_opt: None,
        force_withdraw_opt: None,
    }
}

pub struct TokenEdit {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub mint: Pubkey,
    pub fallback_oracle: Pubkey,
    pub options: mango_v4::instruction::TokenEdit,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenEdit {
    type Accounts = mango_v4::accounts::TokenEdit;
    type Instruction = mango_v4::instruction::TokenEdit;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let mint_info_key = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                self.group.as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let mint_info: MintInfo = account_loader.load(&mint_info_key).await.unwrap();

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            mint_info: mint_info_key,
            oracle: mint_info.oracle,
            fallback_oracle: self.fallback_oracle,
        };

        let mut instruction = make_instruction(program_id, &accounts, &self.options);
        instruction
            .accounts
            .extend(mint_info.banks().iter().map(|&k| AccountMeta {
                pubkey: k,
                is_signer: false,
                is_writable: true,
            }));
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct TokenEditWeights {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub mint: Pubkey,

    pub maint_asset_weight: f32,
    pub maint_liab_weight: f32,
    pub init_asset_weight: f32,
    pub init_liab_weight: f32,
}

#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenEditWeights {
    type Accounts = mango_v4::accounts::TokenEdit;
    type Instruction = mango_v4::instruction::TokenEdit;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let mint_info_key = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                self.group.as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let mint_info: MintInfo = account_loader.load(&mint_info_key).await.unwrap();

        let instruction = Self::Instruction {
            init_asset_weight_opt: Some(self.init_asset_weight),
            init_liab_weight_opt: Some(self.init_liab_weight),
            maint_asset_weight_opt: Some(self.maint_asset_weight),
            maint_liab_weight_opt: Some(self.maint_liab_weight),
            ..token_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            mint_info: mint_info_key,
            oracle: mint_info.oracle,
            fallback_oracle: mint_info.fallback_oracle,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction
            .accounts
            .extend(mint_info.banks().iter().map(|&k| AccountMeta {
                pubkey: k,
                is_signer: false,
                is_writable: true,
            }));
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct TokenResetStablePriceModel {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub mint: Pubkey,
}

#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenResetStablePriceModel {
    type Accounts = mango_v4::accounts::TokenEdit;
    type Instruction = mango_v4::instruction::TokenEdit;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let mint_info_key = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                self.group.as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let mint_info: MintInfo = account_loader.load(&mint_info_key).await.unwrap();

        let instruction = Self::Instruction {
            reset_stable_price: true,
            reset_net_borrow_limit: false,
            ..token_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            mint_info: mint_info_key,
            oracle: mint_info.oracle,
            fallback_oracle: mint_info.fallback_oracle,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction
            .accounts
            .extend(mint_info.banks().iter().map(|&k| AccountMeta {
                pubkey: k,
                is_signer: false,
                is_writable: true,
            }));
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct TokenResetNetBorrows {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub mint: Pubkey,
    pub min_vault_to_deposits_ratio_opt: Option<f64>,
    pub net_borrow_limit_per_window_quote_opt: Option<i64>,
    pub net_borrow_limit_window_size_ts_opt: Option<u64>,
}

#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenResetNetBorrows {
    type Accounts = mango_v4::accounts::TokenEdit;
    type Instruction = mango_v4::instruction::TokenEdit;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let mint_info_key = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                self.group.as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let mint_info: MintInfo = account_loader.load(&mint_info_key).await.unwrap();

        let instruction = Self::Instruction {
            min_vault_to_deposits_ratio_opt: self.min_vault_to_deposits_ratio_opt,
            net_borrow_limit_per_window_quote_opt: self.net_borrow_limit_per_window_quote_opt,
            net_borrow_limit_window_size_ts_opt: self.net_borrow_limit_window_size_ts_opt,
            reset_net_borrow_limit: true,
            ..token_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            mint_info: mint_info_key,
            oracle: mint_info.oracle,
            fallback_oracle: mint_info.fallback_oracle,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction
            .accounts
            .extend(mint_info.banks().iter().map(|&k| AccountMeta {
                pubkey: k,
                is_signer: false,
                is_writable: true,
            }));
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct TokenMakeReduceOnly {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub mint: Pubkey,
    pub reduce_only: u8,
    pub force_close: bool,
}

#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenMakeReduceOnly {
    type Accounts = mango_v4::accounts::TokenEdit;
    type Instruction = mango_v4::instruction::TokenEdit;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let mint_info_key = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                self.group.as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let mint_info: MintInfo = account_loader.load(&mint_info_key).await.unwrap();

        let instruction = Self::Instruction {
            reduce_only_opt: Some(self.reduce_only),
            force_close_opt: Some(self.force_close),
            ..token_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            mint_info: mint_info_key,
            oracle: mint_info.oracle,
            fallback_oracle: mint_info.fallback_oracle,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction
            .accounts
            .extend(mint_info.banks().iter().map(|&k| AccountMeta {
                pubkey: k,
                is_signer: false,
                is_writable: true,
            }));
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct StubOracleSetInstruction {
    pub mint: Pubkey,
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub price: f64,
    pub oracle: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for StubOracleSetInstruction {
    type Accounts = mango_v4::accounts::StubOracleSet;
    type Instruction = mango_v4::instruction::StubOracleSet;

    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            price: I80F48::from_num(self.price),
        };

        let accounts = Self::Accounts {
            oracle: self.oracle,
            group: self.group,
            admin: self.admin.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct StubOracleSetTestInstruction {
    pub oracle: Pubkey,
    pub mint: Pubkey,
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub price: f64,
    pub last_update_slot: u64,
    pub deviation: f64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for StubOracleSetTestInstruction {
    type Accounts = mango_v4::accounts::StubOracleSet;
    type Instruction = mango_v4::instruction::StubOracleSetTest;

    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            price: I80F48::from_num(self.price),
            last_update_slot: self.last_update_slot,
            deviation: I80F48::from_num(self.deviation),
        };

        let accounts = Self::Accounts {
            oracle: self.oracle,
            group: self.group,
            admin: self.admin.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct StubOracleCreate {
    pub oracle: TestKeypair,
    pub admin: TestKeypair,
    pub payer: TestKeypair,
    pub group: Pubkey,
    pub mint: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for StubOracleCreate {
    type Accounts = mango_v4::accounts::StubOracleCreate;
    type Instruction = mango_v4::instruction::StubOracleCreate;

    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            price: I80F48::from_num(1.0),
        };

        let accounts = Self::Accounts {
            group: self.group,
            oracle: self.oracle.pubkey(),
            mint: self.mint,
            admin: self.admin.pubkey(),
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.payer, self.admin, self.oracle]
    }
}

pub struct StubOracleCloseInstruction {
    pub oracle: Pubkey,
    pub group: Pubkey,
    pub mint: Pubkey,
    pub admin: TestKeypair,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for StubOracleCloseInstruction {
    type Accounts = mango_v4::accounts::StubOracleClose;
    type Instruction = mango_v4::instruction::StubOracleClose;

    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            oracle: self.oracle,
            sol_destination: self.sol_destination,
            token_program: Token::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct GroupCreateInstruction {
    pub creator: TestKeypair,
    pub payer: TestKeypair,
    pub insurance_mint: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for GroupCreateInstruction {
    type Accounts = mango_v4::accounts::GroupCreate;
    type Instruction = mango_v4::instruction::GroupCreate;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            group_num: 0,
            testing: 1,
            version: 0,
        };

        let group = Pubkey::find_program_address(
            &[
                b"Group".as_ref(),
                self.creator.pubkey().as_ref(),
                &instruction.group_num.to_le_bytes(),
            ],
            &program_id,
        )
        .0;

        let insurance_vault = Pubkey::find_program_address(
            &[b"InsuranceVault".as_ref(), group.as_ref()],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group,
            creator: self.creator.pubkey(),
            insurance_mint: self.insurance_mint,
            insurance_vault,
            payer: self.payer.pubkey(),
            token_program: Token::id(),
            system_program: System::id(),
            rent: sysvar::rent::Rent::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.creator, self.payer]
    }
}

pub fn group_edit_instruction_default() -> mango_v4::instruction::GroupEdit {
    mango_v4::instruction::GroupEdit {
        admin_opt: None,
        fast_listing_admin_opt: None,
        security_admin_opt: None,
        testing_opt: None,
        version_opt: None,
        deposit_limit_quote_opt: None,
        buyback_fees_opt: None,
        buyback_fees_bonus_factor_opt: None,
        buyback_fees_swap_mango_account_opt: None,
        mngo_token_index_opt: None,
        buyback_fees_expiry_interval_opt: None,
        allowed_fast_listings_per_interval_opt: None,
        collateral_fee_interval_opt: None,
    }
}

pub struct GroupEditFeeParameters {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub fees_mngo_bonus_factor: f32,
    pub fees_mngo_token_index: TokenIndex,
    pub fees_swap_mango_account: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for GroupEditFeeParameters {
    type Accounts = mango_v4::accounts::GroupEdit;
    type Instruction = mango_v4::instruction::GroupEdit;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            buyback_fees_opt: Some(true),
            buyback_fees_bonus_factor_opt: Some(self.fees_mngo_bonus_factor),
            buyback_fees_swap_mango_account_opt: Some(self.fees_swap_mango_account),
            mngo_token_index_opt: Some(self.fees_mngo_token_index),
            ..group_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct GroupEdit {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub options: mango_v4::instruction::GroupEdit,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for GroupEdit {
    type Accounts = mango_v4::accounts::GroupEdit;
    type Instruction = mango_v4::instruction::GroupEdit;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = &self.options;

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct IxGateSetInstruction {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub ix_gate: u128,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for IxGateSetInstruction {
    type Accounts = mango_v4::accounts::IxGateSet;
    type Instruction = mango_v4::instruction::IxGateSet;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            ix_gate: self.ix_gate,
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct GroupCloseInstruction {
    pub admin: TestKeypair,
    pub group: Pubkey,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for GroupCloseInstruction {
    type Accounts = mango_v4::accounts::GroupClose;
    type Instruction = mango_v4::instruction::GroupClose;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let insurance_vault = Pubkey::find_program_address(
            &[b"InsuranceVault".as_ref(), self.group.as_ref()],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            insurance_vault,
            sol_destination: self.sol_destination,
            token_program: Token::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct AccountCreateInstruction {
    pub account_num: u32,
    pub token_count: u8,
    pub serum3_count: u8,
    pub perp_count: u8,
    pub perp_oo_count: u8,
    pub token_conditional_swap_count: u8,
    pub group: Pubkey,
    pub owner: TestKeypair,
    pub payer: TestKeypair,
}
impl Default for AccountCreateInstruction {
    fn default() -> Self {
        AccountCreateInstruction {
            account_num: 0,
            token_count: 8,
            serum3_count: 4,
            perp_count: 4,
            perp_oo_count: 16,
            token_conditional_swap_count: 1,
            group: Default::default(),
            owner: Default::default(),
            payer: Default::default(),
        }
    }
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for AccountCreateInstruction {
    type Accounts = mango_v4::accounts::AccountCreate;
    type Instruction = mango_v4::instruction::AccountCreateV2;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            account_num: self.account_num,
            token_count: self.token_count,
            serum3_count: self.serum3_count,
            perp_count: self.perp_count,
            perp_oo_count: self.perp_oo_count,
            token_conditional_swap_count: self.token_conditional_swap_count,
            name: "my_mango_account".to_string(),
        };

        let account = Pubkey::find_program_address(
            &[
                b"MangoAccount".as_ref(),
                self.group.as_ref(),
                self.owner.pubkey().as_ref(),
                &self.account_num.to_le_bytes(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            owner: self.owner.pubkey(),
            account,
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner, self.payer]
    }
}

#[derive(Default)]
pub struct AccountExpandInstruction {
    pub account_num: u32,
    pub group: Pubkey,
    pub owner: TestKeypair,
    pub payer: TestKeypair,
    pub token_count: u8,
    pub serum3_count: u8,
    pub perp_count: u8,
    pub perp_oo_count: u8,
    pub token_conditional_swap_count: u8,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for AccountExpandInstruction {
    type Accounts = mango_v4::accounts::AccountExpand;
    type Instruction = mango_v4::instruction::AccountExpandV2;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            token_count: self.token_count,
            serum3_count: self.serum3_count,
            perp_count: self.perp_count,
            perp_oo_count: self.perp_oo_count,
            token_conditional_swap_count: self.token_conditional_swap_count,
        };

        let account = Pubkey::find_program_address(
            &[
                b"MangoAccount".as_ref(),
                self.group.as_ref(),
                self.owner.pubkey().as_ref(),
                &self.account_num.to_le_bytes(),
            ],
            &program_id,
        )
        .0;

        let accounts = mango_v4::accounts::AccountExpand {
            group: self.group,
            account,
            owner: self.owner.pubkey(),
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner, self.payer]
    }
}

#[derive(Default)]
pub struct AccountSizeMigrationInstruction {
    pub account: Pubkey,
    pub payer: TestKeypair,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for AccountSizeMigrationInstruction {
    type Accounts = mango_v4::accounts::AccountSizeMigration;
    type Instruction = mango_v4::instruction::AccountSizeMigration;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.payer]
    }
}

pub struct AccountEditInstruction {
    pub account_num: u32,
    pub group: Pubkey,
    pub owner: TestKeypair,
    pub name: String,
    pub delegate: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for AccountEditInstruction {
    type Accounts = mango_v4::accounts::AccountEdit;
    type Instruction = mango_v4::instruction::AccountEdit;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = mango_v4::instruction::AccountEdit {
            name_opt: Some(self.name.to_string()),
            delegate_opt: Some(self.delegate),
            temporary_delegate_opt: None,
            temporary_delegate_expiry_opt: None,
        };

        let account = Pubkey::find_program_address(
            &[
                b"MangoAccount".as_ref(),
                self.group.as_ref(),
                self.owner.pubkey().as_ref(),
                &self.account_num.to_le_bytes(),
            ],
            &program_id,
        )
        .0;

        let accounts = mango_v4::accounts::AccountEdit {
            group: self.group,
            account,
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct AccountCloseInstruction {
    pub group: Pubkey,
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for AccountCloseInstruction {
    type Accounts = mango_v4::accounts::AccountClose;
    type Instruction = mango_v4::instruction::AccountClose;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction { force_close: false };

        let accounts = Self::Accounts {
            group: self.group,
            owner: self.owner.pubkey(),
            account: self.account,
            sol_destination: self.sol_destination,
            token_program: Token::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct AccountBuybackFeesWithMngo {
    pub owner: TestKeypair,
    pub account: Pubkey,
    pub mngo_bank: Pubkey,
    pub fees_bank: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for AccountBuybackFeesWithMngo {
    type Accounts = mango_v4::accounts::AccountBuybackFeesWithMngo;
    type Instruction = mango_v4::instruction::AccountBuybackFeesWithMngo;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            max_buyback_usd: u64::MAX,
        };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let group = account_loader
            .load::<Group>(&account.fixed.group)
            .await
            .unwrap();
        let mngo_bank: Bank = account_loader.load(&self.mngo_bank).await.unwrap();
        let fees_bank: Bank = account_loader.load(&self.fees_bank).await.unwrap();
        let accounts = Self::Accounts {
            group: account.fixed.group,
            owner: self.owner.pubkey(),
            account: self.account,
            dao_account: group.buyback_fees_swap_mango_account,
            mngo_bank: self.mngo_bank,
            mngo_oracle: mngo_bank.oracle,
            fees_bank: self.fees_bank,
            fees_oracle: fees_bank.oracle,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct Serum3RegisterMarketInstruction {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub payer: TestKeypair,

    pub serum_program: Pubkey,
    pub serum_market_external: Pubkey,

    pub base_bank: Pubkey,
    pub quote_bank: Pubkey,

    pub market_index: Serum3MarketIndex,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3RegisterMarketInstruction {
    type Accounts = mango_v4::accounts::Serum3RegisterMarket;
    type Instruction = mango_v4::instruction::Serum3RegisterMarket;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            market_index: self.market_index,
            name: "UUU/usdc".to_string(),
            oracle_price_band: f32::MAX,
        };

        let serum_market = Pubkey::find_program_address(
            &[
                b"Serum3Market".as_ref(),
                self.group.as_ref(),
                self.serum_market_external.as_ref(),
            ],
            &program_id,
        )
        .0;

        let index_reservation = Pubkey::find_program_address(
            &[
                b"Serum3Index".as_ref(),
                self.group.as_ref(),
                &self.market_index.to_le_bytes(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            serum_program: self.serum_program,
            serum_market_external: self.serum_market_external,
            serum_market,
            index_reservation,
            base_bank: self.base_bank,
            quote_bank: self.quote_bank,
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin, self.payer]
    }
}

pub fn serum3_edit_market_instruction_default() -> mango_v4::instruction::Serum3EditMarket {
    mango_v4::instruction::Serum3EditMarket {
        reduce_only_opt: None,
        force_close_opt: None,
        name_opt: None,
        oracle_price_band_opt: None,
    }
}

pub struct Serum3EditMarketInstruction {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub market: Pubkey,
    pub options: mango_v4::instruction::Serum3EditMarket,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3EditMarketInstruction {
    type Accounts = mango_v4::accounts::Serum3EditMarket;
    type Instruction = mango_v4::instruction::Serum3EditMarket;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            market: self.market,
        };

        let instruction = make_instruction(program_id, &accounts, &self.options);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct Serum3DeregisterMarketInstruction {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub serum_market_external: Pubkey,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3DeregisterMarketInstruction {
    type Accounts = mango_v4::accounts::Serum3DeregisterMarket;
    type Instruction = mango_v4::instruction::Serum3DeregisterMarket;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let serum_market = Pubkey::find_program_address(
            &[
                b"Serum3Market".as_ref(),
                self.group.as_ref(),
                self.serum_market_external.as_ref(),
            ],
            &program_id,
        )
        .0;
        let serum_market_data: Serum3Market = account_loader.load(&serum_market).await.unwrap();

        let index_reservation = Pubkey::find_program_address(
            &[
                b"Serum3Index".as_ref(),
                self.group.as_ref(),
                &serum_market_data.market_index.to_le_bytes(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            serum_market,
            index_reservation,
            sol_destination: self.sol_destination,
            token_program: Token::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct Serum3CreateOpenOrdersInstruction {
    pub account: Pubkey,
    pub serum_market: Pubkey,
    pub owner: TestKeypair,
    pub payer: TestKeypair,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3CreateOpenOrdersInstruction {
    type Accounts = mango_v4::accounts::Serum3CreateOpenOrders;
    type Instruction = mango_v4::instruction::Serum3CreateOpenOrders;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = Pubkey::find_program_address(
            &[
                b"Serum3OO".as_ref(),
                self.account.as_ref(),
                self.serum_market.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: account.group,
            account: self.account,
            serum_market: self.serum_market,
            serum_program: serum_market.serum_program,
            serum_market_external: serum_market.serum_market_external,
            open_orders,
            owner: self.owner.pubkey(),
            payer: self.payer.pubkey(),
            system_program: System::id(),
            rent: sysvar::rent::Rent::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner, self.payer]
    }
}

pub struct Serum3CloseOpenOrdersInstruction {
    pub account: Pubkey,
    pub serum_market: Pubkey,
    pub owner: TestKeypair,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3CloseOpenOrdersInstruction {
    type Accounts = mango_v4::accounts::Serum3CloseOpenOrders;
    type Instruction = mango_v4::instruction::Serum3CloseOpenOrders;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = Pubkey::find_program_address(
            &[
                b"Serum3OO".as_ref(),
                self.account.as_ref(),
                self.serum_market.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: account.group,
            account: self.account,
            serum_market: self.serum_market,
            serum_program: serum_market.serum_program,
            serum_market_external: serum_market.serum_market_external,
            open_orders,
            owner: self.owner.pubkey(),
            sol_destination: self.sol_destination,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct Serum3PlaceOrderInstruction {
    pub side: Serum3Side,
    pub limit_price: u64,
    pub max_base_qty: u64,
    pub max_native_quote_qty_including_fees: u64,
    pub self_trade_behavior: Serum3SelfTradeBehavior,
    pub order_type: Serum3OrderType,
    pub client_order_id: u64,
    pub limit: u16,

    pub account: Pubkey,
    pub owner: TestKeypair,

    pub serum_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3PlaceOrderInstruction {
    type Accounts = mango_v4::accounts::Serum3PlaceOrder;
    type Instruction = mango_v4::instruction::Serum3PlaceOrderV2;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            side: self.side,
            limit_price: self.limit_price,
            max_base_qty: self.max_base_qty,
            max_native_quote_qty_including_fees: self.max_native_quote_qty_including_fees,
            self_trade_behavior: self.self_trade_behavior,
            order_type: self.order_type,
            client_order_id: self.client_order_id,
            limit: self.limit,
        };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3_orders(serum_market.market_index)
            .unwrap()
            .open_orders;
        let quote_info =
            get_mint_info_by_token_index(&account_loader, &account, serum_market.quote_token_index)
                .await;
        let base_info =
            get_mint_info_by_token_index(&account_loader, &account, serum_market.base_token_index)
                .await;

        let market_external_bytes = account_loader
            .load_bytes(&serum_market.serum_market_external)
            .await
            .unwrap();
        let market_external: &serum_dex::state::MarketState = bytemuck::from_bytes(
            &market_external_bytes[5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
        );
        // unpack the data, to avoid unaligned references
        let bids = market_external.bids;
        let asks = market_external.asks;
        let event_q = market_external.event_q;
        let req_q = market_external.req_q;
        let coin_vault = market_external.coin_vault;
        let pc_vault = market_external.pc_vault;
        let vault_signer = serum_dex::state::gen_vault_signer_key(
            market_external.vault_signer_nonce,
            &serum_market.serum_market_external,
            &serum_market.serum_program,
        )
        .unwrap();

        let mut health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            false,
            None,
        )
        .await;

        let (payer_info, receiver_info) = &match self.side {
            Serum3Side::Bid => (&quote_info, &base_info),
            Serum3Side::Ask => (&base_info, &quote_info),
        };

        let receiver_active_index = account
            .active_token_positions()
            .position(|tp| tp.token_index == receiver_info.token_index)
            .unwrap();
        health_check_metas[receiver_active_index].is_writable = true;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            open_orders,
            payer_bank: payer_info.first_bank(),
            payer_vault: payer_info.first_vault(),
            payer_oracle: payer_info.oracle,
            serum_market: self.serum_market,
            serum_program: serum_market.serum_program,
            serum_market_external: serum_market.serum_market_external,
            market_bids: from_serum_style_pubkey(&bids),
            market_asks: from_serum_style_pubkey(&asks),
            market_event_queue: from_serum_style_pubkey(&event_q),
            market_request_queue: from_serum_style_pubkey(&req_q),
            market_base_vault: from_serum_style_pubkey(&coin_vault),
            market_quote_vault: from_serum_style_pubkey(&pc_vault),
            market_vault_signer: vault_signer,
            owner: self.owner.pubkey(),
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct Serum3CancelOrderInstruction {
    pub side: Serum3Side,
    pub order_id: u128,

    pub account: Pubkey,
    pub owner: TestKeypair,

    pub serum_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3CancelOrderInstruction {
    type Accounts = mango_v4::accounts::Serum3CancelOrder;
    type Instruction = mango_v4::instruction::Serum3CancelOrder;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            side: self.side,
            order_id: self.order_id,
        };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3_orders(serum_market.market_index)
            .unwrap()
            .open_orders;

        let market_external_bytes = account_loader
            .load_bytes(&serum_market.serum_market_external)
            .await
            .unwrap();
        let market_external: &serum_dex::state::MarketState = bytemuck::from_bytes(
            &market_external_bytes[5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
        );
        // unpack the data, to avoid unaligned references
        let bids = market_external.bids;
        let asks = market_external.asks;
        let event_q = market_external.event_q;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            open_orders,
            serum_market: self.serum_market,
            serum_program: serum_market.serum_program,
            serum_market_external: serum_market.serum_market_external,
            market_bids: from_serum_style_pubkey(&bids),
            market_asks: from_serum_style_pubkey(&asks),
            market_event_queue: from_serum_style_pubkey(&event_q),
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct Serum3CancelOrderByClientOrderIdInstruction {
    pub client_order_id: u64,

    pub account: Pubkey,
    pub owner: TestKeypair,

    pub serum_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3CancelOrderByClientOrderIdInstruction {
    type Accounts = mango_v4::accounts::Serum3CancelOrder;
    type Instruction = mango_v4::instruction::Serum3CancelOrderByClientOrderId;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            client_order_id: self.client_order_id,
        };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3_orders(serum_market.market_index)
            .unwrap()
            .open_orders;

        let market_external_bytes = account_loader
            .load_bytes(&serum_market.serum_market_external)
            .await
            .unwrap();
        let market_external: &serum_dex::state::MarketState = bytemuck::from_bytes(
            &market_external_bytes[5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
        );
        // unpack the data, to avoid unaligned references
        let bids = market_external.bids;
        let asks = market_external.asks;
        let event_q = market_external.event_q;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            open_orders,
            serum_market: self.serum_market,
            serum_program: serum_market.serum_program,
            serum_market_external: serum_market.serum_market_external,
            market_bids: from_serum_style_pubkey(&bids),
            market_asks: from_serum_style_pubkey(&asks),
            market_event_queue: from_serum_style_pubkey(&event_q),
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct Serum3CancelAllOrdersInstruction {
    pub limit: u8,
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub serum_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3CancelAllOrdersInstruction {
    type Accounts = mango_v4::accounts::Serum3CancelAllOrders;
    type Instruction = mango_v4::instruction::Serum3CancelAllOrders;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction { limit: self.limit };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3_orders(serum_market.market_index)
            .unwrap()
            .open_orders;

        let market_external_bytes = account_loader
            .load_bytes(&serum_market.serum_market_external)
            .await
            .unwrap();
        let market_external: &serum_dex::state::MarketState = bytemuck::from_bytes(
            &market_external_bytes[5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
        );
        // unpack the data, to avoid unaligned references
        let bids = market_external.bids;
        let asks = market_external.asks;
        let event_q = market_external.event_q;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            open_orders,
            serum_market: self.serum_market,
            serum_program: serum_market.serum_program,
            serum_market_external: serum_market.serum_market_external,
            market_bids: from_serum_style_pubkey(&bids),
            market_asks: from_serum_style_pubkey(&asks),
            market_event_queue: from_serum_style_pubkey(&event_q),
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct Serum3SettleFundsV2Instruction {
    pub account: Pubkey,
    pub owner: TestKeypair,

    pub serum_market: Pubkey,
    pub fees_to_dao: bool,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3SettleFundsV2Instruction {
    type Accounts = mango_v4::accounts::Serum3SettleFundsV2;
    type Instruction = mango_v4::instruction::Serum3SettleFundsV2;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            fees_to_dao: self.fees_to_dao,
        };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3_orders(serum_market.market_index)
            .unwrap()
            .open_orders;
        let quote_info =
            get_mint_info_by_token_index(&account_loader, &account, serum_market.quote_token_index)
                .await;
        let base_info =
            get_mint_info_by_token_index(&account_loader, &account, serum_market.base_token_index)
                .await;

        let market_external_bytes = account_loader
            .load_bytes(&serum_market.serum_market_external)
            .await
            .unwrap();
        let market_external: &serum_dex::state::MarketState = bytemuck::from_bytes(
            &market_external_bytes[5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
        );
        // unpack the data, to avoid unaligned references
        let coin_vault = market_external.coin_vault;
        let pc_vault = market_external.pc_vault;
        let vault_signer = serum_dex::state::gen_vault_signer_key(
            market_external.vault_signer_nonce,
            &serum_market.serum_market_external,
            &serum_market.serum_program,
        )
        .unwrap();

        let accounts = Self::Accounts {
            v1: mango_v4::accounts::Serum3SettleFunds {
                group: account.fixed.group,
                account: self.account,
                open_orders,
                quote_bank: quote_info.first_bank(),
                quote_vault: quote_info.first_vault(),
                base_bank: base_info.first_bank(),
                base_vault: base_info.first_vault(),
                serum_market: self.serum_market,
                serum_program: serum_market.serum_program,
                serum_market_external: serum_market.serum_market_external,
                market_base_vault: from_serum_style_pubkey(&coin_vault),
                market_quote_vault: from_serum_style_pubkey(&pc_vault),
                market_vault_signer: vault_signer,
                owner: self.owner.pubkey(),
                token_program: Token::id(),
            },
            v2: mango_v4::accounts::Serum3SettleFundsV2Extra {
                quote_oracle: quote_info.oracle,
                base_oracle: base_info.oracle,
            },
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct Serum3LiqForceCancelOrdersInstruction {
    pub account: Pubkey,
    pub serum_market: Pubkey,
    pub limit: u8,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3LiqForceCancelOrdersInstruction {
    type Accounts = mango_v4::accounts::Serum3LiqForceCancelOrders;
    type Instruction = mango_v4::instruction::Serum3LiqForceCancelOrders;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction { limit: self.limit };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3_orders(serum_market.market_index)
            .unwrap()
            .open_orders;
        let quote_info =
            get_mint_info_by_token_index(&account_loader, &account, serum_market.quote_token_index)
                .await;
        let base_info =
            get_mint_info_by_token_index(&account_loader, &account, serum_market.base_token_index)
                .await;

        let market_external_bytes = account_loader
            .load_bytes(&serum_market.serum_market_external)
            .await
            .unwrap();
        let market_external: &serum_dex::state::MarketState = bytemuck::from_bytes(
            &market_external_bytes[5..5 + std::mem::size_of::<serum_dex::state::MarketState>()],
        );
        // unpack the data, to avoid unaligned references
        let bids = market_external.bids;
        let asks = market_external.asks;
        let event_q = market_external.event_q;
        let coin_vault = market_external.coin_vault;
        let pc_vault = market_external.pc_vault;
        let vault_signer = serum_dex::state::gen_vault_signer_key(
            market_external.vault_signer_nonce,
            &serum_market.serum_market_external,
            &serum_market.serum_program,
        )
        .unwrap();

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            false,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            open_orders,
            quote_bank: quote_info.first_bank(),
            quote_vault: quote_info.first_vault(),
            base_bank: base_info.first_bank(),
            base_vault: base_info.first_vault(),
            serum_market: self.serum_market,
            serum_program: serum_market.serum_program,
            serum_market_external: serum_market.serum_market_external,
            market_bids: from_serum_style_pubkey(&bids),
            market_asks: from_serum_style_pubkey(&asks),
            market_event_queue: from_serum_style_pubkey(&event_q),
            market_base_vault: from_serum_style_pubkey(&coin_vault),
            market_quote_vault: from_serum_style_pubkey(&pc_vault),
            market_vault_signer: vault_signer,
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct TokenForceCloseBorrowsWithTokenInstruction {
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub liqor_owner: TestKeypair,

    pub asset_token_index: TokenIndex,
    pub asset_bank_index: usize,
    pub liab_token_index: TokenIndex,
    pub liab_bank_index: usize,
    pub max_liab_transfer: u64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenForceCloseBorrowsWithTokenInstruction {
    type Accounts = mango_v4::accounts::TokenForceCloseBorrowsWithToken;
    type Instruction = mango_v4::instruction::TokenForceCloseBorrowsWithToken;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            asset_token_index: self.asset_token_index,
            liab_token_index: self.liab_token_index,
            max_liab_transfer: self.max_liab_transfer,
        };

        let liqee = account_loader
            .load_mango_account(&self.liqee)
            .await
            .unwrap();
        let liqor = account_loader
            .load_mango_account(&self.liqor)
            .await
            .unwrap();
        let health_check_metas = derive_liquidation_remaining_account_metas(
            &account_loader,
            &liqee,
            &liqor,
            self.asset_token_index,
            self.asset_bank_index,
            self.liab_token_index,
            self.liab_bank_index,
        )
        .await;

        let accounts = Self::Accounts {
            group: liqee.fixed.group,
            liqee: self.liqee,
            liqor: self.liqor,
            liqor_owner: self.liqor_owner.pubkey(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.liqor_owner]
    }
}

pub struct TokenForceWithdrawInstruction {
    pub account: Pubkey,
    pub bank: Pubkey,
    pub target: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenForceWithdrawInstruction {
    type Accounts = mango_v4::accounts::TokenForceWithdraw;
    type Instruction = mango_v4::instruction::TokenForceWithdraw;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let bank = account_loader.load::<Bank>(&self.bank).await.unwrap();
        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            false,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            bank: self.bank,
            vault: bank.vault,
            oracle: bank.oracle,
            owner_ata_token_account: self.target,
            alternate_owner_token_account: self.target,
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct TokenLiqWithTokenInstruction {
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub liqor_owner: TestKeypair,

    pub asset_token_index: TokenIndex,
    pub asset_bank_index: usize,
    pub liab_token_index: TokenIndex,
    pub liab_bank_index: usize,
    pub max_liab_transfer: I80F48,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenLiqWithTokenInstruction {
    type Accounts = mango_v4::accounts::TokenLiqWithToken;
    type Instruction = mango_v4::instruction::TokenLiqWithToken;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            asset_token_index: self.asset_token_index,
            liab_token_index: self.liab_token_index,
            max_liab_transfer: self.max_liab_transfer,
        };

        let liqee = account_loader
            .load_mango_account(&self.liqee)
            .await
            .unwrap();
        let liqor = account_loader
            .load_mango_account(&self.liqor)
            .await
            .unwrap();
        let health_check_metas = derive_liquidation_remaining_account_metas(
            &account_loader,
            &liqee,
            &liqor,
            self.asset_token_index,
            self.asset_bank_index,
            self.liab_token_index,
            self.liab_bank_index,
        )
        .await;

        let accounts = Self::Accounts {
            group: liqee.fixed.group,
            liqee: self.liqee,
            liqor: self.liqor,
            liqor_owner: self.liqor_owner.pubkey(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.liqor_owner]
    }
}

pub struct TokenLiqBankruptcyInstruction {
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub liqor_owner: TestKeypair,

    pub max_liab_transfer: I80F48,
    pub liab_mint_info: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenLiqBankruptcyInstruction {
    type Accounts = mango_v4::accounts::TokenLiqBankruptcy;
    type Instruction = mango_v4::instruction::TokenLiqBankruptcy;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            max_liab_transfer: self.max_liab_transfer,
        };

        let liab_mint_info: MintInfo = account_loader.load(&self.liab_mint_info).await.unwrap();
        let liqee = account_loader
            .load_mango_account(&self.liqee)
            .await
            .unwrap();
        let liqor = account_loader
            .load_mango_account(&self.liqor)
            .await
            .unwrap();
        let health_check_metas = derive_liquidation_remaining_account_metas(
            &account_loader,
            &liqee,
            &liqor,
            QUOTE_TOKEN_INDEX,
            0,
            liab_mint_info.token_index,
            0,
        )
        .await;

        let group_key = liqee.fixed.group;
        let group: Group = account_loader.load(&group_key).await.unwrap();

        let quote_mint_info = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                liqee.fixed.group.as_ref(),
                group.insurance_mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let quote_mint_info: MintInfo = account_loader.load(&quote_mint_info).await.unwrap();

        let insurance_vault = Pubkey::find_program_address(
            &[b"InsuranceVault".as_ref(), group_key.as_ref()],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: group_key,
            liqee: self.liqee,
            liqor: self.liqor,
            liqor_owner: self.liqor_owner.pubkey(),
            liab_mint_info: self.liab_mint_info,
            quote_vault: quote_mint_info.first_vault(),
            insurance_vault,
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        let mut bank_ams = liab_mint_info
            .banks()
            .iter()
            .map(|bank| AccountMeta {
                pubkey: *bank,
                is_signer: false,
                is_writable: true,
            })
            .collect::<Vec<_>>();
        instruction.accounts.append(&mut bank_ams);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.liqor_owner]
    }
}

#[derive(Default)]
pub struct PerpCreateMarketInstruction {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub oracle: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_queue: Pubkey,
    pub payer: TestKeypair,
    pub settle_token_index: TokenIndex,
    pub perp_market_index: PerpMarketIndex,
    pub base_decimals: u8,
    pub quote_lot_size: i64,
    pub base_lot_size: i64,
    pub maint_base_asset_weight: f32,
    pub init_base_asset_weight: f32,
    pub maint_base_liab_weight: f32,
    pub init_base_liab_weight: f32,
    pub maint_overall_asset_weight: f32,
    pub init_overall_asset_weight: f32,
    pub base_liquidation_fee: f32,
    pub positive_pnl_liquidation_fee: f32,
    pub maker_fee: f32,
    pub taker_fee: f32,
    pub group_insurance_fund: bool,
    pub fee_penalty: f32,
    pub settle_fee_flat: f32,
    pub settle_fee_amount_threshold: f32,
    pub settle_fee_fraction_low_health: f32,
    pub settle_pnl_limit_factor: f32,
    pub settle_pnl_limit_window_size_ts: u64,
    pub platform_liquidation_fee: f32,
}
impl PerpCreateMarketInstruction {
    pub async fn with_new_book_and_queue(
        solana: &SolanaCookie,
        base: &super::mango_setup::Token,
    ) -> Self {
        PerpCreateMarketInstruction {
            bids: solana
                .create_account_for_type::<BookSide>(&mango_v4::id())
                .await,
            asks: solana
                .create_account_for_type::<BookSide>(&mango_v4::id())
                .await,
            event_queue: solana
                .create_account_for_type::<EventQueue>(&mango_v4::id())
                .await,
            oracle: base.oracle,
            base_decimals: base.mint.decimals,
            ..PerpCreateMarketInstruction::default()
        }
    }
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpCreateMarketInstruction {
    type Accounts = mango_v4::accounts::PerpCreateMarket;
    type Instruction = mango_v4::instruction::PerpCreateMarket;
    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            name: "UUU-PERP".to_string(),
            oracle_config: OracleConfigParams {
                conf_filter: 0.1,
                max_staleness_slots: None,
            },
            settle_token_index: self.settle_token_index,
            perp_market_index: self.perp_market_index,
            quote_lot_size: self.quote_lot_size,
            base_lot_size: self.base_lot_size,
            maint_base_asset_weight: self.maint_base_asset_weight,
            init_base_asset_weight: self.init_base_asset_weight,
            maint_base_liab_weight: self.maint_base_liab_weight,
            init_base_liab_weight: self.init_base_liab_weight,
            maint_overall_asset_weight: self.maint_overall_asset_weight,
            init_overall_asset_weight: self.init_overall_asset_weight,
            base_liquidation_fee: self.base_liquidation_fee,
            maker_fee: self.maker_fee,
            taker_fee: self.taker_fee,
            max_funding: 0.05,
            min_funding: 0.05,
            impact_quantity: 100,
            base_decimals: self.base_decimals,
            group_insurance_fund: self.group_insurance_fund,
            fee_penalty: self.fee_penalty,
            settle_fee_flat: self.settle_fee_flat,
            settle_fee_amount_threshold: self.settle_fee_amount_threshold,
            settle_fee_fraction_low_health: self.settle_fee_fraction_low_health,
            settle_pnl_limit_factor: self.settle_pnl_limit_factor,
            settle_pnl_limit_window_size_ts: self.settle_pnl_limit_window_size_ts,
            positive_pnl_liquidation_fee: self.positive_pnl_liquidation_fee,
            platform_liquidation_fee: self.platform_liquidation_fee,
        };

        let perp_market = Pubkey::find_program_address(
            &[
                b"PerpMarket".as_ref(),
                self.group.as_ref(),
                self.perp_market_index.to_le_bytes().as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            oracle: self.oracle,
            perp_market,
            bids: self.bids,
            asks: self.asks,
            event_queue: self.event_queue,
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin, self.payer]
    }
}

fn perp_edit_instruction_default() -> mango_v4::instruction::PerpEditMarket {
    mango_v4::instruction::PerpEditMarket {
        oracle_opt: None,
        oracle_config_opt: None,
        base_decimals_opt: None,
        maint_base_asset_weight_opt: None,
        init_base_asset_weight_opt: None,
        maint_base_liab_weight_opt: None,
        init_base_liab_weight_opt: None,
        maint_overall_asset_weight_opt: None,
        init_overall_asset_weight_opt: None,
        base_liquidation_fee_opt: None,
        maker_fee_opt: None,
        taker_fee_opt: None,
        min_funding_opt: None,
        max_funding_opt: None,
        impact_quantity_opt: None,
        group_insurance_fund_opt: None,
        fee_penalty_opt: None,
        settle_fee_flat_opt: None,
        settle_fee_amount_threshold_opt: None,
        settle_fee_fraction_low_health_opt: None,
        stable_price_delay_interval_seconds_opt: None,
        stable_price_delay_growth_limit_opt: None,
        stable_price_growth_limit_opt: None,
        settle_pnl_limit_factor_opt: None,
        settle_pnl_limit_window_size_ts_opt: None,
        reduce_only_opt: None,
        reset_stable_price: false,
        positive_pnl_liquidation_fee_opt: None,
        name_opt: None,
        force_close_opt: None,
        platform_liquidation_fee_opt: None,
    }
}

pub struct PerpResetStablePriceModel {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub perp_market: Pubkey,
}

#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpResetStablePriceModel {
    type Accounts = mango_v4::accounts::PerpEditMarket;
    type Instruction = mango_v4::instruction::PerpEditMarket;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();

        let instruction = Self::Instruction {
            reset_stable_price: true,
            ..perp_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            perp_market: self.perp_market,
            oracle: perp_market.oracle,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct PerpSetSettleLimitWindow {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub perp_market: Pubkey,
    pub window_size_ts: u64,
}

#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpSetSettleLimitWindow {
    type Accounts = mango_v4::accounts::PerpEditMarket;
    type Instruction = mango_v4::instruction::PerpEditMarket;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();

        let instruction = Self::Instruction {
            settle_pnl_limit_window_size_ts_opt: Some(self.window_size_ts),
            ..perp_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            perp_market: self.perp_market,
            oracle: perp_market.oracle,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct PerpMakeReduceOnly {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub perp_market: Pubkey,
    pub reduce_only: bool,
    pub force_close: bool,
}

#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpMakeReduceOnly {
    type Accounts = mango_v4::accounts::PerpEditMarket;
    type Instruction = mango_v4::instruction::PerpEditMarket;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();

        let instruction = Self::Instruction {
            reduce_only_opt: Some(self.reduce_only),
            force_close_opt: Some(self.force_close),
            ..perp_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            perp_market: self.perp_market,
            oracle: perp_market.oracle,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct PerpChangeWeights {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub perp_market: Pubkey,
    pub init_overall_asset_weight: f32,
    pub maint_overall_asset_weight: f32,
}

#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpChangeWeights {
    type Accounts = mango_v4::accounts::PerpEditMarket;
    type Instruction = mango_v4::instruction::PerpEditMarket;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();

        let instruction = Self::Instruction {
            init_overall_asset_weight_opt: Some(self.init_overall_asset_weight),
            maint_overall_asset_weight_opt: Some(self.maint_overall_asset_weight),
            ..perp_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            perp_market: self.perp_market,
            oracle: perp_market.oracle,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct PerpCloseMarketInstruction {
    pub admin: TestKeypair,
    pub perp_market: Pubkey,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpCloseMarketInstruction {
    type Accounts = mango_v4::accounts::PerpCloseMarket;
    type Instruction = mango_v4::instruction::PerpCloseMarket;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};
        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();

        let accounts = Self::Accounts {
            group: perp_market.group,
            admin: self.admin.pubkey(),
            perp_market: self.perp_market,
            bids: perp_market.bids,
            asks: perp_market.asks,
            event_queue: perp_market.event_queue,
            token_program: Token::id(),
            sol_destination: self.sol_destination,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct PerpDeactivatePositionInstruction {
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub owner: TestKeypair,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpDeactivatePositionInstruction {
    type Accounts = mango_v4::accounts::PerpDeactivatePosition;
    type Instruction = mango_v4::instruction::PerpDeactivatePosition;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();

        let instruction = Self::Instruction {};
        let accounts = Self::Accounts {
            group: perp_market.group,
            account: self.account,
            perp_market: self.perp_market,
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct PerpPlaceOrderInstruction {
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub owner: TestKeypair,
    pub side: Side,
    pub price_lots: i64,
    pub max_base_lots: i64,
    pub max_quote_lots: i64,
    pub reduce_only: bool,
    pub client_order_id: u64,
    pub self_trade_behavior: SelfTradeBehavior,
    pub limit: u8,
}
impl Default for PerpPlaceOrderInstruction {
    fn default() -> Self {
        Self {
            account: Pubkey::default(),
            perp_market: Pubkey::default(),
            owner: TestKeypair::default(),
            side: Side::Bid,
            price_lots: 0,
            max_base_lots: i64::MAX,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
            self_trade_behavior: SelfTradeBehavior::DecrementTake,
            limit: 10,
        }
    }
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpPlaceOrderInstruction {
    type Accounts = mango_v4::accounts::PerpPlaceOrder;
    type Instruction = mango_v4::instruction::PerpPlaceOrderV2;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            side: self.side,
            price_lots: self.price_lots,
            max_base_lots: self.max_base_lots,
            max_quote_lots: self.max_quote_lots,
            client_order_id: self.client_order_id,
            order_type: PlaceOrderType::Limit,
            self_trade_behavior: self.self_trade_behavior,
            reduce_only: self.reduce_only,
            expiry_timestamp: 0,
            limit: self.limit,
        };

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            false,
            Some(perp_market.perp_market_index),
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            perp_market: self.perp_market,
            bids: perp_market.bids,
            asks: perp_market.asks,
            event_queue: perp_market.event_queue,
            oracle: perp_market.oracle,
            owner: self.owner.pubkey(),
        };
        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct PerpPlaceOrderPeggedInstruction {
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub owner: TestKeypair,
    pub side: Side,
    pub price_offset: i64,
    pub max_base_lots: i64,
    pub max_quote_lots: i64,
    pub client_order_id: u64,
    pub peg_limit: i64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpPlaceOrderPeggedInstruction {
    type Accounts = mango_v4::accounts::PerpPlaceOrder;
    type Instruction = mango_v4::instruction::PerpPlaceOrderPeggedV2;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            side: self.side,
            price_offset_lots: self.price_offset,
            peg_limit: self.peg_limit,
            max_base_lots: self.max_base_lots,
            max_quote_lots: self.max_quote_lots,
            client_order_id: self.client_order_id,
            order_type: PlaceOrderType::Limit,
            reduce_only: false,
            expiry_timestamp: 0,
            self_trade_behavior: SelfTradeBehavior::DecrementTake,
            limit: 10,
            max_oracle_staleness_slots: -1,
        };

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            false,
            Some(perp_market.perp_market_index),
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            perp_market: self.perp_market,
            bids: perp_market.bids,
            asks: perp_market.asks,
            event_queue: perp_market.event_queue,
            oracle: perp_market.oracle,
            owner: self.owner.pubkey(),
        };
        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct PerpCancelOrderInstruction {
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub owner: TestKeypair,
    pub order_id: u128,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpCancelOrderInstruction {
    type Accounts = mango_v4::accounts::PerpCancelOrder;
    type Instruction = mango_v4::instruction::PerpCancelOrder;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            order_id: self.order_id,
        };
        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let accounts = Self::Accounts {
            group: perp_market.group,
            account: self.account,
            perp_market: self.perp_market,
            bids: perp_market.bids,
            asks: perp_market.asks,
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct PerpCancelOrderByClientOrderIdInstruction {
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub owner: TestKeypair,
    pub client_order_id: u64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpCancelOrderByClientOrderIdInstruction {
    type Accounts = mango_v4::accounts::PerpCancelOrderByClientOrderId;
    type Instruction = mango_v4::instruction::PerpCancelOrderByClientOrderId;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            client_order_id: self.client_order_id,
        };
        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let accounts = Self::Accounts {
            group: perp_market.group,
            account: self.account,
            perp_market: self.perp_market,
            bids: perp_market.bids,
            asks: perp_market.asks,
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct PerpCancelAllOrdersInstruction {
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub owner: TestKeypair,
    pub limit: u8,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpCancelAllOrdersInstruction {
    type Accounts = mango_v4::accounts::PerpCancelAllOrders;
    type Instruction = mango_v4::instruction::PerpCancelAllOrders;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction { limit: self.limit };
        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let accounts = Self::Accounts {
            group: perp_market.group,
            account: self.account,
            perp_market: self.perp_market,
            bids: perp_market.bids,
            asks: perp_market.asks,
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct PerpConsumeEventsInstruction {
    pub perp_market: Pubkey,
    pub mango_accounts: Vec<Pubkey>,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpConsumeEventsInstruction {
    type Accounts = mango_v4::accounts::PerpConsumeEvents;
    type Instruction = mango_v4::instruction::PerpConsumeEvents;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction { limit: 10 };

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let accounts = Self::Accounts {
            group: perp_market.group,
            perp_market: self.perp_market,
            event_queue: perp_market.event_queue,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction
            .accounts
            .extend(self.mango_accounts.iter().map(|ma| AccountMeta {
                pubkey: *ma,
                is_signer: false,
                is_writable: true,
            }));
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct PerpUpdateFundingInstruction {
    pub perp_market: Pubkey,
    pub bank: Pubkey,
    pub oracle: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpUpdateFundingInstruction {
    type Accounts = mango_v4::accounts::PerpUpdateFunding;
    type Instruction = mango_v4::instruction::PerpUpdateFunding;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};
        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let accounts = Self::Accounts {
            group: perp_market.group,
            perp_market: self.perp_market,
            bids: perp_market.bids,
            asks: perp_market.asks,
            oracle: self.oracle,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct PerpSettlePnlInstruction {
    pub settler: Pubkey,
    pub settler_owner: TestKeypair,
    pub account_a: Pubkey,
    pub account_b: Pubkey,
    pub perp_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpSettlePnlInstruction {
    type Accounts = mango_v4::accounts::PerpSettlePnl;
    type Instruction = mango_v4::instruction::PerpSettlePnl;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let account_a = account_loader
            .load_mango_account(&self.account_a)
            .await
            .unwrap();
        let account_b = account_loader
            .load_mango_account(&self.account_b)
            .await
            .unwrap();
        let health_check_metas = derive_liquidation_remaining_account_metas(
            &account_loader,
            &account_a,
            &account_b,
            TokenIndex::MAX,
            0,
            TokenIndex::MAX,
            0,
        )
        .await;
        let settle_mint_info = get_mint_info_by_token_index(
            &account_loader,
            &account_a,
            perp_market.settle_token_index,
        )
        .await;

        let accounts = Self::Accounts {
            group: perp_market.group,
            settler: self.settler,
            settler_owner: self.settler_owner.pubkey(),
            perp_market: self.perp_market,
            account_a: self.account_a,
            account_b: self.account_b,
            oracle: perp_market.oracle,
            settle_bank: settle_mint_info.first_bank(),
            settle_oracle: settle_mint_info.oracle,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.settler_owner]
    }
}

pub struct PerpForceClosePositionInstruction {
    pub account_a: Pubkey,
    pub account_b: Pubkey,
    pub perp_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpForceClosePositionInstruction {
    type Accounts = mango_v4::accounts::PerpForceClosePosition;
    type Instruction = mango_v4::instruction::PerpForceClosePosition;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();

        let accounts = Self::Accounts {
            group: perp_market.group,
            perp_market: self.perp_market,
            account_a: self.account_a,
            account_b: self.account_b,
            oracle: perp_market.oracle,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct PerpSettleFeesInstruction {
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub max_settle_amount: u64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpSettleFeesInstruction {
    type Accounts = mango_v4::accounts::PerpSettleFees;
    type Instruction = mango_v4::instruction::PerpSettleFees;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            max_settle_amount: self.max_settle_amount,
        };

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            false,
            Some(perp_market.perp_market_index),
        )
        .await;
        let settle_mint_info =
            get_mint_info_by_token_index(&account_loader, &account, perp_market.settle_token_index)
                .await;

        let accounts = Self::Accounts {
            group: perp_market.group,
            perp_market: self.perp_market,
            account: self.account,
            oracle: perp_market.oracle,
            settle_bank: settle_mint_info.first_bank(),
            settle_oracle: settle_mint_info.oracle,
        };
        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct PerpLiqForceCancelOrdersInstruction {
    pub account: Pubkey,
    pub perp_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpLiqForceCancelOrdersInstruction {
    type Accounts = mango_v4::accounts::PerpLiqForceCancelOrders;
    type Instruction = mango_v4::instruction::PerpLiqForceCancelOrders;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction { limit: 10 };

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            false,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            perp_market: self.perp_market,
            account: self.account,
            bids: perp_market.bids,
            asks: perp_market.asks,
        };
        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct PerpLiqBaseOrPositivePnlInstruction {
    pub liqor: Pubkey,
    pub liqor_owner: TestKeypair,
    pub liqee: Pubkey,
    pub perp_market: Pubkey,
    pub max_base_transfer: i64,
    pub max_pnl_transfer: u64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpLiqBaseOrPositivePnlInstruction {
    type Accounts = mango_v4::accounts::PerpLiqBaseOrPositivePnl;
    type Instruction = mango_v4::instruction::PerpLiqBaseOrPositivePnl;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            max_base_transfer: self.max_base_transfer,
            max_pnl_transfer: self.max_pnl_transfer,
        };

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let group_key = perp_market.group;
        let liqor = account_loader
            .load_mango_account(&self.liqor)
            .await
            .unwrap();
        let liqee = account_loader
            .load_mango_account(&self.liqee)
            .await
            .unwrap();
        let health_check_metas = derive_liquidation_remaining_account_metas(
            &account_loader,
            &liqee,
            &liqor,
            TokenIndex::MAX,
            0,
            TokenIndex::MAX,
            0,
        )
        .await;

        let settle_mint_info =
            get_mint_info_by_token_index(&account_loader, &liqee, perp_market.settle_token_index)
                .await;

        let accounts = Self::Accounts {
            group: group_key,
            perp_market: self.perp_market,
            oracle: perp_market.oracle,
            liqor: self.liqor,
            liqor_owner: self.liqor_owner.pubkey(),
            liqee: self.liqee,
            settle_bank: settle_mint_info.first_bank(),
            settle_vault: settle_mint_info.first_vault(),
            settle_oracle: settle_mint_info.oracle,
        };
        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.liqor_owner]
    }
}

pub struct PerpLiqNegativePnlOrBankruptcyInstruction {
    pub liqor: Pubkey,
    pub liqor_owner: TestKeypair,
    pub liqee: Pubkey,
    pub perp_market: Pubkey,
    pub max_liab_transfer: u64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpLiqNegativePnlOrBankruptcyInstruction {
    type Accounts = mango_v4::accounts::PerpLiqNegativePnlOrBankruptcyV2;
    type Instruction = mango_v4::instruction::PerpLiqNegativePnlOrBankruptcyV2;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            max_liab_transfer: self.max_liab_transfer,
        };

        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let group_key = perp_market.group;
        let liqor = account_loader
            .load_mango_account(&self.liqor)
            .await
            .unwrap();
        let liqee = account_loader
            .load_mango_account(&self.liqee)
            .await
            .unwrap();
        let health_check_metas = derive_liquidation_remaining_account_metas(
            &account_loader,
            &liqee,
            &liqor,
            TokenIndex::MAX,
            0,
            TokenIndex::MAX,
            0,
        )
        .await;

        let group = account_loader.load::<Group>(&group_key).await.unwrap();
        let settle_mint_info =
            get_mint_info_by_token_index(&account_loader, &liqee, perp_market.settle_token_index)
                .await;
        let insurance_mint_info =
            get_mint_info_by_token_index(&account_loader, &liqee, QUOTE_TOKEN_INDEX).await;

        let accounts = Self::Accounts {
            group: group_key,
            liqor: self.liqor,
            liqor_owner: self.liqor_owner.pubkey(),
            liqee: self.liqee,
            perp_market: self.perp_market,
            oracle: perp_market.oracle,
            settle_bank: settle_mint_info.first_bank(),
            settle_vault: settle_mint_info.first_vault(),
            settle_oracle: settle_mint_info.oracle,
            insurance_vault: group.insurance_vault,
            insurance_bank: insurance_mint_info.first_bank(),
            insurance_bank_vault: insurance_mint_info.first_vault(),
            insurance_oracle: insurance_mint_info.oracle,
            token_program: Token::id(),
        };
        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.liqor_owner]
    }
}

pub struct BenchmarkInstruction {}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for BenchmarkInstruction {
    type Accounts = mango_v4::accounts::Benchmark;
    type Instruction = mango_v4::instruction::Benchmark;
    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};
        let accounts = Self::Accounts {
            dummy: Pubkey::new_unique(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}
pub struct TokenUpdateIndexAndRateInstruction {
    pub mint_info: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenUpdateIndexAndRateInstruction {
    type Accounts = mango_v4::accounts::TokenUpdateIndexAndRate;
    type Instruction = mango_v4::instruction::TokenUpdateIndexAndRate;
    async fn to_instruction(
        &self,
        loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let mint_info: MintInfo = loader.load(&self.mint_info).await.unwrap();

        let accounts = Self::Accounts {
            group: mint_info.group,
            mint_info: self.mint_info,
            oracle: mint_info.oracle,
            instructions: solana_program::sysvar::instructions::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        let mut bank_ams = mint_info
            .banks()
            .iter()
            .map(|bank| AccountMeta {
                pubkey: *bank,
                is_signer: false,
                is_writable: true,
            })
            .collect::<Vec<_>>();
        instruction.accounts.append(&mut bank_ams);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct ComputeAccountDataInstruction {
    pub account: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for ComputeAccountDataInstruction {
    type Accounts = mango_v4::accounts::ComputeAccountData;
    type Instruction = mango_v4::instruction::ComputeAccountData;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            false,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct HealthRegionBeginInstruction {
    pub account: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for HealthRegionBeginInstruction {
    type Accounts = mango_v4::accounts::HealthRegionBegin;
    type Instruction = mango_v4::instruction::HealthRegionBegin;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            false,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            instructions: solana_program::sysvar::instructions::id(),
            account: self.account,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct HealthRegionEndInstruction {
    pub account: Pubkey,
    pub affected_bank: Option<Pubkey>,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for HealthRegionEndInstruction {
    type Accounts = mango_v4::accounts::HealthRegionEnd;
    type Instruction = mango_v4::instruction::HealthRegionEnd;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            self.affected_bank,
            false,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            account: self.account,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct AltSetInstruction {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub address_lookup_table: Pubkey,
    pub index: u8,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for AltSetInstruction {
    type Accounts = mango_v4::accounts::AltSet;
    type Instruction = mango_v4::instruction::AltSet;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction { index: self.index };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            address_lookup_table: self.address_lookup_table,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

pub struct AltExtendInstruction {
    pub group: Pubkey,
    pub admin: TestKeypair,
    pub payer: TestKeypair,
    pub address_lookup_table: Pubkey,
    pub index: u8,
    pub new_addresses: Vec<Pubkey>,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for AltExtendInstruction {
    type Accounts = mango_v4::accounts::AltExtend;
    type Instruction = mango_v4::instruction::AltExtend;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            index: self.index,
            new_addresses: self.new_addresses.clone(),
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            payer: self.payer.pubkey(),
            address_lookup_table: self.address_lookup_table,
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin, self.payer]
    }
}

#[derive(Clone)]
pub struct TokenConditionalSwapCreateInstruction {
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub buy_mint: Pubkey,
    pub sell_mint: Pubkey,
    pub max_buy: u64,
    pub max_sell: u64,
    pub price_lower_limit: f64,
    pub price_upper_limit: f64,
    pub price_premium_rate: f64,
    pub allow_creating_deposits: bool,
    pub allow_creating_borrows: bool,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenConditionalSwapCreateInstruction {
    type Accounts = mango_v4::accounts::TokenConditionalSwapCreate;
    type Instruction = mango_v4::instruction::TokenConditionalSwapCreateV2;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            max_buy: self.max_buy,
            max_sell: self.max_sell,
            expiry_timestamp: u64::MAX,
            price_lower_limit: self.price_lower_limit,
            price_upper_limit: self.price_upper_limit,
            price_premium_rate: self.price_premium_rate,
            allow_creating_deposits: self.allow_creating_deposits,
            allow_creating_borrows: self.allow_creating_borrows,
            display_price_style: TokenConditionalSwapDisplayPriceStyle::SellTokenPerBuyToken,
            intention: TokenConditionalSwapIntention::Unknown,
        };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();

        let buy_mint_info_address = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                account.fixed.group.as_ref(),
                self.buy_mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let sell_mint_info_address = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                account.fixed.group.as_ref(),
                self.sell_mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let buy_mint_info: MintInfo = account_loader.load(&buy_mint_info_address).await.unwrap();
        let sell_mint_info: MintInfo = account_loader.load(&sell_mint_info_address).await.unwrap();

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            authority: self.owner.pubkey(),
            buy_bank: buy_mint_info.first_bank(),
            sell_bank: sell_mint_info.first_bank(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

#[derive(Clone)]
pub struct TokenConditionalSwapCreateLinearAuctionInstruction {
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub buy_mint: Pubkey,
    pub sell_mint: Pubkey,
    pub max_buy: u64,
    pub max_sell: u64,
    pub price_start: f64,
    pub price_end: f64,
    pub allow_creating_deposits: bool,
    pub allow_creating_borrows: bool,
    pub start_timestamp: u64,
    pub duration_seconds: u64,
    pub expiry_timestamp: u64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenConditionalSwapCreateLinearAuctionInstruction {
    type Accounts = mango_v4::accounts::TokenConditionalSwapCreate;
    type Instruction = mango_v4::instruction::TokenConditionalSwapCreateLinearAuction;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            max_buy: self.max_buy,
            max_sell: self.max_sell,
            expiry_timestamp: self.expiry_timestamp,
            price_start: self.price_start,
            price_end: self.price_end,
            allow_creating_deposits: self.allow_creating_deposits,
            allow_creating_borrows: self.allow_creating_borrows,
            display_price_style: TokenConditionalSwapDisplayPriceStyle::SellTokenPerBuyToken,
            start_timestamp: self.start_timestamp,
            duration_seconds: self.duration_seconds,
        };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();

        let buy_mint_info_address = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                account.fixed.group.as_ref(),
                self.buy_mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let sell_mint_info_address = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                account.fixed.group.as_ref(),
                self.sell_mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let buy_mint_info: MintInfo = account_loader.load(&buy_mint_info_address).await.unwrap();
        let sell_mint_info: MintInfo = account_loader.load(&sell_mint_info_address).await.unwrap();

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            authority: self.owner.pubkey(),
            buy_bank: buy_mint_info.first_bank(),
            sell_bank: sell_mint_info.first_bank(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

#[derive(Clone)]
pub struct TokenConditionalSwapCreatePremiumAuctionInstruction {
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub buy_mint: Pubkey,
    pub sell_mint: Pubkey,
    pub max_buy: u64,
    pub max_sell: u64,
    pub price_lower_limit: f64,
    pub price_upper_limit: f64,
    pub max_price_premium_rate: f64,
    pub allow_creating_deposits: bool,
    pub allow_creating_borrows: bool,
    pub duration_seconds: u64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenConditionalSwapCreatePremiumAuctionInstruction {
    type Accounts = mango_v4::accounts::TokenConditionalSwapCreate;
    type Instruction = mango_v4::instruction::TokenConditionalSwapCreatePremiumAuction;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            max_buy: self.max_buy,
            max_sell: self.max_sell,
            expiry_timestamp: u64::MAX,
            price_lower_limit: self.price_lower_limit,
            price_upper_limit: self.price_upper_limit,
            max_price_premium_rate: self.max_price_premium_rate,
            allow_creating_deposits: self.allow_creating_deposits,
            allow_creating_borrows: self.allow_creating_borrows,
            display_price_style: TokenConditionalSwapDisplayPriceStyle::SellTokenPerBuyToken,
            intention: TokenConditionalSwapIntention::Unknown,
            duration_seconds: self.duration_seconds,
        };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();

        let buy_mint_info_address = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                account.fixed.group.as_ref(),
                self.buy_mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let sell_mint_info_address = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                account.fixed.group.as_ref(),
                self.sell_mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let buy_mint_info: MintInfo = account_loader.load(&buy_mint_info_address).await.unwrap();
        let sell_mint_info: MintInfo = account_loader.load(&sell_mint_info_address).await.unwrap();

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            authority: self.owner.pubkey(),
            buy_bank: buy_mint_info.first_bank(),
            sell_bank: sell_mint_info.first_bank(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

#[derive(Clone)]
pub struct TokenConditionalSwapCancelInstruction {
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub index: u8,
    pub id: u64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenConditionalSwapCancelInstruction {
    type Accounts = mango_v4::accounts::TokenConditionalSwapCancel;
    type Instruction = mango_v4::instruction::TokenConditionalSwapCancel;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            token_conditional_swap_index: self.index,
            token_conditional_swap_id: self.id,
        };

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let tcs = account.token_conditional_swap_by_id(self.id).unwrap().1;

        let buy_mint_info =
            get_mint_info_by_token_index(&account_loader, &account, tcs.buy_token_index).await;
        let sell_mint_info =
            get_mint_info_by_token_index(&account_loader, &account, tcs.sell_token_index).await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
            authority: self.owner.pubkey(),
            buy_bank: buy_mint_info.first_bank(),
            sell_bank: sell_mint_info.first_bank(),
        };

        let instruction = make_instruction(program_id, &accounts, &instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

#[derive(Clone)]
pub struct TokenConditionalSwapTriggerInstruction {
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub liqor_owner: TestKeypair,
    pub index: u8,
    pub max_buy_token_to_liqee: u64,
    pub max_sell_token_to_liqor: u64,
    pub min_buy_token: u64,
    pub min_taker_price: f32,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenConditionalSwapTriggerInstruction {
    type Accounts = mango_v4::accounts::TokenConditionalSwapTrigger;
    type Instruction = mango_v4::instruction::TokenConditionalSwapTriggerV2;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let liqee = account_loader
            .load_mango_account(&self.liqee)
            .await
            .unwrap();
        let liqor = account_loader
            .load_mango_account(&self.liqor)
            .await
            .unwrap();

        let tcs = liqee
            .token_conditional_swap_by_index(self.index.into())
            .unwrap()
            .clone();

        let instruction = Self::Instruction {
            token_conditional_swap_index: self.index,
            token_conditional_swap_id: tcs.id,
            max_buy_token_to_liqee: self.max_buy_token_to_liqee,
            max_sell_token_to_liqor: self.max_sell_token_to_liqor,
            min_buy_token: self.min_buy_token,
            min_taker_price: self.min_taker_price,
        };

        let health_check_metas = derive_liquidation_remaining_account_metas(
            &account_loader,
            &liqee,
            &liqor,
            tcs.buy_token_index,
            0,
            tcs.sell_token_index,
            0,
        )
        .await;

        let accounts = Self::Accounts {
            group: liqee.fixed.group,
            liqee: self.liqee,
            liqor: self.liqor,
            liqor_authority: self.liqor_owner.pubkey(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.liqor_owner]
    }
}

#[derive(Clone)]
pub struct TokenConditionalSwapStartInstruction {
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub liqor_owner: TestKeypair,
    pub index: u8,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenConditionalSwapStartInstruction {
    type Accounts = mango_v4::accounts::TokenConditionalSwapStart;
    type Instruction = mango_v4::instruction::TokenConditionalSwapStart;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let liqee = account_loader
            .load_mango_account(&self.liqee)
            .await
            .unwrap();

        let tcs = liqee
            .token_conditional_swap_by_index(self.index.into())
            .unwrap()
            .clone();

        let sell_mint_info =
            get_mint_info_by_token_index(&account_loader, &liqee, tcs.sell_token_index).await;

        let instruction = Self::Instruction {
            token_conditional_swap_index: self.index,
            token_conditional_swap_id: tcs.id,
        };

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &liqee,
            Some(sell_mint_info.first_bank()),
            true,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            group: liqee.fixed.group,
            liqee: self.liqee,
            liqor: self.liqor,
            liqor_authority: self.liqor_owner.pubkey(),
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.liqor_owner]
    }
}

#[derive(Clone)]
pub struct TokenChargeCollateralFeesInstruction {
    pub account: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for TokenChargeCollateralFeesInstruction {
    type Accounts = mango_v4::accounts::TokenChargeCollateralFees;
    type Instruction = mango_v4::instruction::TokenChargeCollateralFees;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();

        let instruction = Self::Instruction {};

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            true,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.fixed.group,
            account: self.account,
        };

        let mut instruction = make_instruction(program_id, &accounts, &instruction);
        instruction.accounts.extend(health_check_metas.into_iter());
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}
