#![allow(dead_code)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::{self, SysvarId};
use anchor_spl::token::{Token, TokenAccount};
use fixed::types::I80F48;
use itertools::Itertools;
use mango_v4::instructions::{
    InterestRateParams, Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side,
};
use mango_v4::state::{MangoAccount, MangoAccountValue};
use solana_program::instruction::Instruction;
use solana_program_test::BanksClientError;
use solana_sdk::instruction;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transport::TransportError;
use std::str::FromStr;
use std::sync::Arc;

use super::solana::SolanaCookie;
use super::utils::clone_keypair;
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

// TODO: report error outwards etc
pub async fn send_tx<CI: ClientInstruction>(
    solana: &SolanaCookie,
    ix: CI,
) -> std::result::Result<CI::Accounts, TransportError> {
    let (accounts, instruction) = ix.to_instruction(solana).await;
    let signers = ix.signers();
    let instructions = vec![instruction];
    solana
        .process_transaction(&instructions, Some(&signers[..]))
        .await?;
    Ok(accounts)
}

/// Build a transaction from multiple instructions
pub struct ClientTransaction {
    solana: Arc<SolanaCookie>,
    instructions: Vec<instruction::Instruction>,
    signers: Vec<Keypair>,
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
        self.signers
            .extend(ix.signers().iter().map(|k| clone_keypair(k)));
        accounts
    }

    pub fn add_instruction_direct(&mut self, ix: instruction::Instruction) {
        self.instructions.push(ix);
    }

    pub fn add_signer(&mut self, keypair: &Keypair) {
        self.signers.push(clone_keypair(keypair));
    }

    pub async fn send(&self) -> std::result::Result<(), BanksClientError> {
        let signer_refs = self.signers.iter().map(|k| k).collect::<Vec<&Keypair>>();
        self.solana
            .process_transaction(&self.instructions, Some(&signer_refs[..]))
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
    fn signers(&self) -> Vec<&Keypair>;
}

fn make_instruction(
    program_id: Pubkey,
    accounts: &impl anchor_lang::ToAccountMetas,
    data: impl anchor_lang::InstructionData,
) -> instruction::Instruction {
    instruction::Instruction {
        program_id,
        accounts: anchor_lang::ToAccountMetas::to_account_metas(accounts, None),
        data: anchor_lang::InstructionData::data(&data),
    }
}

async fn get_mint_info_by_mint(
    account_loader: &impl ClientAccountLoader,
    account: &MangoAccountValue,
    mint: Pubkey,
) -> MintInfo {
    let mint_info_pk = Pubkey::find_program_address(
        &[
            account.fixed.group.as_ref(),
            b"MintInfo".as_ref(),
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
            account.fixed.group.as_ref(),
            b"Bank".as_ref(),
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
            group.as_ref(),
            b"PerpMarket".as_ref(),
            &perp_market_index.to_le_bytes(),
        ],
        &mango_v4::id(),
    )
    .0
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
            .token_get_mut_or_create(bank.token_index)
            .unwrap();
    }
    if let Some(affected_perp_market_index) = affected_perp_market_index {
        adjusted_account
            .perp_get_account_mut_or_create(affected_perp_market_index)
            .unwrap();
    }

    // figure out all the banks/oracles that need to be passed for the health check
    let mut banks = vec![];
    let mut oracles = vec![];
    for position in adjusted_account.token_iter_active() {
        let mint_info =
            get_mint_info_by_token_index(account_loader, account, position.token_index).await;
        banks.push(mint_info.first_bank());
        oracles.push(mint_info.oracle);
    }

    let perp_markets = adjusted_account
        .perp_iter_active_accounts()
        .map(|perp| get_perp_market_address_by_index(account.fixed.group, perp.market_index));

    let serum_oos = account.serum3_iter_active().map(|&s| s.open_orders);

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
        .token_iter_active()
        .chain(liqor.token_iter_active())
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

    let perp_markets = liqee
        .perp_iter_active_accounts()
        .chain(liqee.perp_iter_active_accounts())
        .map(|perp| get_perp_market_address_by_index(liqee.fixed.group, perp.market_index))
        .unique();

    let serum_oos = liqee
        .serum3_iter_active()
        .chain(liqor.serum3_iter_active())
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
        .chain(perp_markets.map(to_account_meta))
        .chain(serum_oos.map(to_account_meta))
        .collect()
}

fn from_serum_style_pubkey(d: &[u64; 4]) -> Pubkey {
    Pubkey::new(bytemuck::cast_slice(d as &[_]))
}

pub async fn get_mango_account(solana: &SolanaCookie, account: Pubkey) -> MangoAccountValue {
    let bytes = solana.get_account_data(account).await.unwrap();
    MangoAccountValue::from_bytes(&bytes[8..]).unwrap()
}

pub async fn account_position(solana: &SolanaCookie, account: Pubkey, bank: Pubkey) -> i64 {
    let account_data = get_mango_account(solana, account).await;
    let bank_data: Bank = solana.get_account(bank).await;
    let native = account_data
        .token_find(bank_data.token_index)
        .unwrap()
        .native(&bank_data);
    native.round().to_num::<i64>()
}

pub async fn account_position_closed(solana: &SolanaCookie, account: Pubkey, bank: Pubkey) -> bool {
    let account_data = get_mango_account(solana, account).await;
    let bank_data: Bank = solana.get_account(bank).await;
    account_data.token_find(bank_data.token_index).is_none()
}

pub async fn account_position_f64(solana: &SolanaCookie, account: Pubkey, bank: Pubkey) -> f64 {
    let account_data = get_mango_account(solana, account).await;
    let bank_data: Bank = solana.get_account(bank).await;
    let native = account_data
        .token_find(bank_data.token_index)
        .unwrap()
        .native(&bank_data);
    native.to_num::<f64>()
}

//
// a struct for each instruction along with its
// ClientInstruction impl
//

pub struct FlashLoanBeginInstruction {
    pub group: Pubkey,
    pub mango_token_bank: Pubkey,
    pub mango_token_vault: Pubkey,
    pub target_token_account: Pubkey,
    pub withdraw_amount: u64,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for FlashLoanBeginInstruction {
    type Accounts = mango_v4::accounts::FlashLoanBegin;
    type Instruction = mango_v4::instruction::FlashLoanBegin;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let accounts = Self::Accounts {
            group: self.group,
            token_program: Token::id(),
            instructions: solana_program::sysvar::instructions::id(),
        };

        let instruction = Self::Instruction {
            loan_amounts: vec![self.withdraw_amount],
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.push(AccountMeta {
            pubkey: self.mango_token_bank,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: self.mango_token_vault,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: self.target_token_account,
            is_writable: true,
            is_signer: false,
        });

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![]
    }
}

pub struct FlashLoanEndInstruction<'keypair> {
    pub account: Pubkey,
    pub owner: &'keypair Keypair,
    pub mango_token_bank: Pubkey,
    pub mango_token_vault: Pubkey,
    pub target_token_account: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for FlashLoanEndInstruction<'keypair> {
    type Accounts = mango_v4::accounts::FlashLoanEnd;
    type Instruction = mango_v4::instruction::FlashLoanEnd;
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
            Some(self.mango_token_bank),
            true,
            None,
        )
        .await;

        let accounts = Self::Accounts {
            account: self.account,
            owner: self.owner.pubkey(),
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());
        instruction.accounts.push(AccountMeta {
            pubkey: self.mango_token_vault,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: self.target_token_account,
            is_writable: true,
            is_signer: false,
        });

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct TokenWithdrawInstruction<'keypair> {
    pub amount: u64,
    pub allow_borrow: bool,

    pub account: Pubkey,
    pub owner: &'keypair Keypair,
    pub token_account: Pubkey,
    pub bank_index: usize,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for TokenWithdrawInstruction<'keypair> {
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
                account.fixed.group.as_ref(),
                b"MintInfo".as_ref(),
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
            token_account: self.token_account,
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct TokenDepositInstruction {
    pub amount: u64,

    pub account: Pubkey,
    pub token_account: Pubkey,
    pub token_authority: Keypair,
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
        };

        // load account so we know its mint
        let token_account: TokenAccount = account_loader.load(&self.token_account).await.unwrap();
        let account = account_loader
            .load_mango_account(&self.account)
            .await
            .unwrap();
        let mint_info = Pubkey::find_program_address(
            &[
                account.fixed.group.as_ref(),
                b"MintInfo".as_ref(),
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
            token_account: self.token_account,
            token_authority: self.token_authority.pubkey(),
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![&self.token_authority]
    }
}

pub struct TokenRegisterInstruction<'keypair> {
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

    pub group: Pubkey,
    pub admin: &'keypair Keypair,
    pub mint: Pubkey,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for TokenRegisterInstruction<'keypair> {
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
            oracle_config: OracleConfig {
                conf_filter: I80F48::from_num::<f32>(0.10),
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
        };

        let bank = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"Bank".as_ref(),
                &self.token_index.to_le_bytes(),
                &0u32.to_le_bytes(),
            ],
            &program_id,
        )
        .0;
        let vault = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"Vault".as_ref(),
                &self.token_index.to_le_bytes(),
                &0u32.to_le_bytes(),
            ],
            &program_id,
        )
        .0;
        let mint_info = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"MintInfo".as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        // TODO: remove copy pasta of pda derivation, use reference
        let oracle = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"StubOracle".as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            mint: self.mint,
            bank,
            vault,
            mint_info,
            oracle,
            payer: self.payer.pubkey(),
            token_program: Token::id(),
            system_program: System::id(),
            rent: sysvar::rent::Rent::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin, self.payer]
    }
}

pub struct TokenAddBankInstruction<'keypair> {
    pub token_index: TokenIndex,
    pub bank_num: u32,

    pub group: Pubkey,
    pub admin: &'keypair Keypair,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for TokenAddBankInstruction<'keypair> {
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
                self.group.as_ref(),
                b"Bank".as_ref(),
                &self.token_index.to_le_bytes(),
                &0u32.to_le_bytes(),
            ],
            &program_id,
        )
        .0;
        let bank = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"Bank".as_ref(),
                &self.token_index.to_le_bytes(),
                &self.bank_num.to_le_bytes(),
            ],
            &program_id,
        )
        .0;
        let vault = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"Vault".as_ref(),
                &self.token_index.to_le_bytes(),
                &self.bank_num.to_le_bytes(),
            ],
            &program_id,
        )
        .0;

        let existing_bank_data: Bank = account_loader.load(&existing_bank).await.unwrap();
        let mint = existing_bank_data.mint;

        let mint_info = Pubkey::find_program_address(
            &[self.group.as_ref(), b"MintInfo".as_ref(), mint.as_ref()],
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin, self.payer]
    }
}

pub struct TokenDeregisterInstruction<'keypair> {
    pub admin: &'keypair Keypair,
    pub payer: &'keypair Keypair,
    pub group: Pubkey,
    pub mint_info: Pubkey,
    pub banks: Vec<Pubkey>,
    pub vaults: Vec<Pubkey>,
    pub dust_vault: Pubkey,
    pub token_index: TokenIndex,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for TokenDeregisterInstruction<'keypair> {
    type Accounts = mango_v4::accounts::TokenDeregister;
    type Instruction = mango_v4::instruction::TokenDeregister;

    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            token_index: self.token_index,
        };

        let accounts = Self::Accounts {
            admin: self.admin.pubkey(),
            group: self.group,
            mint_info: self.mint_info,
            dust_vault: self.dust_vault,
            sol_destination: self.sol_destination,
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);

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

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin]
    }
}

pub struct StubOracleSetInstruction<'keypair> {
    pub mint: Pubkey,
    pub group: Pubkey,
    pub admin: &'keypair Keypair,
    pub payer: &'keypair Keypair,
    pub price: &'static str,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for StubOracleSetInstruction<'keypair> {
    type Accounts = mango_v4::accounts::StubOracleSet;
    type Instruction = mango_v4::instruction::StubOracleSet;

    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            price: I80F48::from_str(self.price).unwrap(),
        };
        // TODO: remove copy pasta of pda derivation, use reference
        let oracle = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"StubOracle".as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            oracle,
            group: self.group,
            admin: self.admin.pubkey(),
            payer: self.payer.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.payer, self.admin]
    }
}

pub struct StubOracleCreate<'keypair> {
    pub group: Pubkey,
    pub mint: Pubkey,
    pub admin: &'keypair Keypair,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for StubOracleCreate<'keypair> {
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

        let oracle = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"StubOracle".as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            oracle,
            token_mint: self.mint,
            admin: self.admin.pubkey(),
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.payer, self.admin]
    }
}

pub struct StubOracleCloseInstruction<'keypair> {
    pub group: Pubkey,
    pub mint: Pubkey,
    pub admin: &'keypair Keypair,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for StubOracleCloseInstruction<'keypair> {
    type Accounts = mango_v4::accounts::StubOracleClose;
    type Instruction = mango_v4::instruction::StubOracleClose;

    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let oracle = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"StubOracle".as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            oracle,
            sol_destination: self.sol_destination,
            token_program: Token::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin]
    }
}

pub struct GroupCreateInstruction<'keypair> {
    pub creator: &'keypair Keypair,
    pub payer: &'keypair Keypair,
    pub insurance_mint: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for GroupCreateInstruction<'keypair> {
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
            &[group.as_ref(), b"InsuranceVault".as_ref()],
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.creator, self.payer]
    }
}

pub struct GroupCloseInstruction<'keypair> {
    pub admin: &'keypair Keypair,
    pub group: Pubkey,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for GroupCloseInstruction<'keypair> {
    type Accounts = mango_v4::accounts::GroupClose;
    type Instruction = mango_v4::instruction::GroupClose;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let insurance_vault = Pubkey::find_program_address(
            &[self.group.as_ref(), b"InsuranceVault".as_ref()],
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin]
    }
}

pub struct AccountCreateInstruction<'keypair> {
    pub account_num: u32,
    pub account_size: AccountSize,
    pub group: Pubkey,
    pub owner: &'keypair Keypair,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for AccountCreateInstruction<'keypair> {
    type Accounts = mango_v4::accounts::AccountCreate;
    type Instruction = mango_v4::instruction::AccountCreate;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = mango_v4::instruction::AccountCreate {
            account_num: self.account_num,
            account_size: self.account_size,
            name: "my_mango_account".to_string(),
        };

        let account = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"MangoAccount".as_ref(),
                self.owner.pubkey().as_ref(),
                &self.account_num.to_le_bytes(),
            ],
            &program_id,
        )
        .0;

        let accounts = mango_v4::accounts::AccountCreate {
            group: self.group,
            owner: self.owner.pubkey(),
            account,
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner, self.payer]
    }
}

pub struct AccountExpandInstruction<'keypair> {
    pub account_num: u32,
    pub group: Pubkey,
    pub owner: &'keypair Keypair,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for AccountExpandInstruction<'keypair> {
    type Accounts = mango_v4::accounts::AccountExpand;
    type Instruction = mango_v4::instruction::AccountExpand;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = mango_v4::instruction::AccountExpand {};

        let account = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"MangoAccount".as_ref(),
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner, self.payer]
    }
}

pub struct AccountEditInstruction<'keypair> {
    pub account_num: u32,
    pub group: Pubkey,
    pub owner: &'keypair Keypair,
    pub name: String,
    pub delegate: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for AccountEditInstruction<'keypair> {
    type Accounts = mango_v4::accounts::AccountEdit;
    type Instruction = mango_v4::instruction::AccountEdit;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = mango_v4::instruction::AccountEdit {
            name_opt: Option::from(self.name.to_string()),
            delegate_opt: Option::from(self.delegate),
        };

        let account = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"MangoAccount".as_ref(),
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct AccountCloseInstruction<'keypair> {
    pub group: Pubkey,
    pub account: Pubkey,
    pub owner: &'keypair Keypair,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for AccountCloseInstruction<'keypair> {
    type Accounts = mango_v4::accounts::AccountClose;
    type Instruction = mango_v4::instruction::AccountClose;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let accounts = Self::Accounts {
            group: self.group,
            owner: self.owner.pubkey(),
            account: self.account,
            sol_destination: self.sol_destination,
            token_program: Token::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct Serum3RegisterMarketInstruction<'keypair> {
    pub group: Pubkey,
    pub admin: &'keypair Keypair,
    pub payer: &'keypair Keypair,

    pub serum_program: Pubkey,
    pub serum_market_external: Pubkey,

    pub base_bank: Pubkey,
    pub quote_bank: Pubkey,

    pub market_index: Serum3MarketIndex,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for Serum3RegisterMarketInstruction<'keypair> {
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
        };

        let serum_market = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"Serum3Market".as_ref(),
                self.serum_market_external.as_ref(),
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
            base_bank: self.base_bank,
            quote_bank: self.quote_bank,
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin, self.payer]
    }
}

pub struct Serum3DeregisterMarketInstruction<'keypair> {
    pub group: Pubkey,
    pub admin: &'keypair Keypair,
    pub serum_market_external: Pubkey,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for Serum3DeregisterMarketInstruction<'keypair> {
    type Accounts = mango_v4::accounts::Serum3DeregisterMarket;
    type Instruction = mango_v4::instruction::Serum3DeregisterMarket;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let serum_market = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"Serum3Market".as_ref(),
                self.serum_market_external.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            serum_market,
            sol_destination: self.sol_destination,
            token_program: Token::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin]
    }
}

pub struct Serum3CreateOpenOrdersInstruction<'keypair> {
    pub account: Pubkey,
    pub serum_market: Pubkey,
    pub owner: &'keypair Keypair,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for Serum3CreateOpenOrdersInstruction<'keypair> {
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
                self.account.as_ref(),
                b"Serum3OO".as_ref(),
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner, self.payer]
    }
}

pub struct Serum3CloseOpenOrdersInstruction<'keypair> {
    pub account: Pubkey,
    pub serum_market: Pubkey,
    pub owner: &'keypair Keypair,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for Serum3CloseOpenOrdersInstruction<'keypair> {
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
                self.account.as_ref(),
                b"Serum3OO".as_ref(),
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct Serum3PlaceOrderInstruction<'keypair> {
    pub side: Serum3Side,
    pub limit_price: u64,
    pub max_base_qty: u64,
    pub max_native_quote_qty_including_fees: u64,
    pub self_trade_behavior: Serum3SelfTradeBehavior,
    pub order_type: Serum3OrderType,
    pub client_order_id: u64,
    pub limit: u16,

    pub account: Pubkey,
    pub owner: &'keypair Keypair,

    pub serum_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for Serum3PlaceOrderInstruction<'keypair> {
    type Accounts = mango_v4::accounts::Serum3PlaceOrder;
    type Instruction = mango_v4::instruction::Serum3PlaceOrder;
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
            .serum3_find(serum_market.market_index)
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
            market_request_queue: from_serum_style_pubkey(&req_q),
            market_base_vault: from_serum_style_pubkey(&coin_vault),
            market_quote_vault: from_serum_style_pubkey(&pc_vault),
            market_vault_signer: vault_signer,
            owner: self.owner.pubkey(),
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct Serum3CancelOrderInstruction<'keypair> {
    pub side: Serum3Side,
    pub order_id: u128,

    pub account: Pubkey,
    pub owner: &'keypair Keypair,

    pub serum_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for Serum3CancelOrderInstruction<'keypair> {
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
            .serum3_find(serum_market.market_index)
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct Serum3CancelAllOrdersInstruction<'keypair> {
    pub limit: u8,
    pub account: Pubkey,
    pub owner: &'keypair Keypair,
    pub serum_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for Serum3CancelAllOrdersInstruction<'keypair> {
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
            .serum3_find(serum_market.market_index)
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct Serum3SettleFundsInstruction<'keypair> {
    pub account: Pubkey,
    pub owner: &'keypair Keypair,

    pub serum_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for Serum3SettleFundsInstruction<'keypair> {
    type Accounts = mango_v4::accounts::Serum3SettleFunds;
    type Instruction = mango_v4::instruction::Serum3SettleFunds;
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
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3_find(serum_market.market_index)
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
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
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
            .serum3_find(serum_market.market_index)
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![]
    }
}

pub struct LiqTokenWithTokenInstruction<'keypair> {
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub liqor_owner: &'keypair Keypair,

    pub asset_token_index: TokenIndex,
    pub asset_bank_index: usize,
    pub liab_token_index: TokenIndex,
    pub liab_bank_index: usize,
    pub max_liab_transfer: I80F48,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for LiqTokenWithTokenInstruction<'keypair> {
    type Accounts = mango_v4::accounts::LiqTokenWithToken;
    type Instruction = mango_v4::instruction::LiqTokenWithToken;
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.liqor_owner]
    }
}

pub struct LiqTokenBankruptcyInstruction<'keypair> {
    pub liqee: Pubkey,
    pub liqor: Pubkey,
    pub liqor_owner: &'keypair Keypair,

    pub liab_token_index: TokenIndex,
    pub max_liab_transfer: I80F48,
    pub liab_mint_info: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for LiqTokenBankruptcyInstruction<'keypair> {
    type Accounts = mango_v4::accounts::LiqTokenBankruptcy;
    type Instruction = mango_v4::instruction::LiqTokenBankruptcy;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            liab_token_index: self.liab_token_index,
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
            self.liab_token_index,
            0,
        )
        .await;

        let group_key = liqee.fixed.group;
        let group: Group = account_loader.load(&group_key).await.unwrap();

        let quote_mint_info = Pubkey::find_program_address(
            &[
                liqee.fixed.group.as_ref(),
                b"MintInfo".as_ref(),
                group.insurance_mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let quote_mint_info: MintInfo = account_loader.load(&quote_mint_info).await.unwrap();

        let insurance_vault = Pubkey::find_program_address(
            &[group_key.as_ref(), b"InsuranceVault".as_ref()],
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.liqor_owner]
    }
}

pub struct PerpCreateMarketInstruction<'keypair> {
    pub group: Pubkey,
    pub admin: &'keypair Keypair,
    pub oracle: Pubkey,
    pub asks: Pubkey,
    pub bids: Pubkey,
    pub event_queue: Pubkey,
    pub payer: &'keypair Keypair,
    pub perp_market_index: PerpMarketIndex,
    pub base_token_index: TokenIndex,
    pub base_token_decimals: u8,
    pub quote_lot_size: i64,
    pub base_lot_size: i64,
    pub maint_asset_weight: f32,
    pub init_asset_weight: f32,
    pub maint_liab_weight: f32,
    pub init_liab_weight: f32,
    pub liquidation_fee: f32,
    pub maker_fee: f32,
    pub taker_fee: f32,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for PerpCreateMarketInstruction<'keypair> {
    type Accounts = mango_v4::accounts::PerpCreateMarket;
    type Instruction = mango_v4::instruction::PerpCreateMarket;
    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            name: "UUU-PERP".to_string(),
            oracle_config: OracleConfig {
                conf_filter: I80F48::from_num::<f32>(0.10),
            },
            perp_market_index: self.perp_market_index,
            base_token_index_opt: Option::from(self.base_token_index),
            quote_lot_size: self.quote_lot_size,
            base_lot_size: self.base_lot_size,
            maint_asset_weight: self.maint_asset_weight,
            init_asset_weight: self.init_asset_weight,
            maint_liab_weight: self.maint_liab_weight,
            init_liab_weight: self.init_liab_weight,
            liquidation_fee: self.liquidation_fee,
            maker_fee: self.maker_fee,
            taker_fee: self.taker_fee,
            max_funding: 0.05,
            min_funding: 0.05,
            impact_quantity: 100,
            base_token_decimals: self.base_token_decimals,
        };

        let perp_market = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"PerpMarket".as_ref(),
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
            asks: self.asks,
            bids: self.bids,
            event_queue: self.event_queue,
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin, self.payer]
    }
}

pub struct PerpCloseMarketInstruction<'keypair> {
    pub group: Pubkey,
    pub admin: &'keypair Keypair,
    pub perp_market: Pubkey,
    pub asks: Pubkey,
    pub bids: Pubkey,
    pub event_queue: Pubkey,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for PerpCloseMarketInstruction<'keypair> {
    type Accounts = mango_v4::accounts::PerpCloseMarket;
    type Instruction = mango_v4::instruction::PerpCloseMarket;
    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            perp_market: self.perp_market,
            asks: self.asks,
            bids: self.bids,
            event_queue: self.event_queue,
            token_program: Token::id(),
            sol_destination: self.sol_destination,
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin]
    }
}

pub struct PerpPlaceOrderInstruction<'keypair> {
    pub group: Pubkey,
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub asks: Pubkey,
    pub bids: Pubkey,
    pub event_queue: Pubkey,
    pub oracle: Pubkey,
    pub owner: &'keypair Keypair,
    pub side: Side,
    pub price_lots: i64,
    pub max_base_lots: i64,
    pub max_quote_lots: i64,
    pub client_order_id: u64,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for PerpPlaceOrderInstruction<'keypair> {
    type Accounts = mango_v4::accounts::PerpPlaceOrder;
    type Instruction = mango_v4::instruction::PerpPlaceOrder;
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
            order_type: OrderType::Limit,
            expiry_timestamp: 0,
            limit: 1,
        };
        let accounts = Self::Accounts {
            group: self.group,
            account: self.account,
            perp_market: self.perp_market,
            asks: self.asks,
            bids: self.bids,
            event_queue: self.event_queue,
            oracle: self.oracle,
            owner: self.owner.pubkey(),
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct PerpCancelOrderInstruction<'keypair> {
    pub group: Pubkey,
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub asks: Pubkey,
    pub bids: Pubkey,
    pub owner: &'keypair Keypair,
    pub order_id: i128,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for PerpCancelOrderInstruction<'keypair> {
    type Accounts = mango_v4::accounts::PerpCancelOrder;
    type Instruction = mango_v4::instruction::PerpCancelOrder;
    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            order_id: self.order_id,
        };
        let accounts = Self::Accounts {
            group: self.group,
            account: self.account,
            perp_market: self.perp_market,
            asks: self.asks,
            bids: self.bids,
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct PerpCancelOrderByClientOrderIdInstruction<'keypair> {
    pub group: Pubkey,
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub asks: Pubkey,
    pub bids: Pubkey,
    pub owner: &'keypair Keypair,
    pub client_order_id: u64,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for PerpCancelOrderByClientOrderIdInstruction<'keypair> {
    type Accounts = mango_v4::accounts::PerpCancelOrderByClientOrderId;
    type Instruction = mango_v4::instruction::PerpCancelOrderByClientOrderId;
    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            client_order_id: self.client_order_id,
        };
        let accounts = Self::Accounts {
            group: self.group,
            account: self.account,
            perp_market: self.perp_market,
            asks: self.asks,
            bids: self.bids,
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct PerpCancelAllOrdersInstruction<'keypair> {
    pub group: Pubkey,
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub asks: Pubkey,
    pub bids: Pubkey,
    pub owner: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for PerpCancelAllOrdersInstruction<'keypair> {
    type Accounts = mango_v4::accounts::PerpCancelAllOrders;
    type Instruction = mango_v4::instruction::PerpCancelAllOrders;
    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction { limit: 5 };
        let accounts = Self::Accounts {
            group: self.group,
            account: self.account,
            perp_market: self.perp_market,
            asks: self.asks,
            bids: self.bids,
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct PerpConsumeEventsInstruction {
    pub group: Pubkey,
    pub perp_market: Pubkey,
    pub event_queue: Pubkey,
    pub mango_accounts: Vec<Pubkey>,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpConsumeEventsInstruction {
    type Accounts = mango_v4::accounts::PerpConsumeEvents;
    type Instruction = mango_v4::instruction::PerpConsumeEvents;
    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction { limit: 10 };
        let accounts = Self::Accounts {
            group: self.group,
            perp_market: self.perp_market,
            event_queue: self.event_queue,
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction
            .accounts
            .extend(self.mango_accounts.iter().map(|ma| AccountMeta {
                pubkey: *ma,
                is_signer: false,
                is_writable: true,
            }));
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![]
    }
}

pub struct PerpUpdateFundingInstruction {
    pub perp_market: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub bank: Pubkey,
    pub oracle: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpUpdateFundingInstruction {
    type Accounts = mango_v4::accounts::PerpUpdateFunding;
    type Instruction = mango_v4::instruction::PerpUpdateFunding;
    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};
        let accounts = Self::Accounts {
            perp_market: self.perp_market,
            bids: self.bids,
            asks: self.asks,
            oracle: self.oracle,
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![]
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
        let accounts = Self::Accounts {};

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
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
            mint_info: self.mint_info,
            oracle: mint_info.oracle,
            instructions: solana_program::sysvar::instructions::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

    fn signers(&self) -> Vec<&Keypair> {
        vec![]
    }
}

pub struct ComputeAccountDataInstruction {
    pub account: Pubkey,
    pub health_type: HealthType,
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![]
    }
}
