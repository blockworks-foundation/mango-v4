#![allow(dead_code)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::{self, SysvarId};
use anchor_spl::token::{Token, TokenAccount};
use fixed::types::I80F48;
use itertools::Itertools;
use mango_v4::accounts_ix::{
    InterestRateParams, Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side,
};
use mango_v4::state::{MangoAccount, MangoAccountValue};
use solana_program::instruction::Instruction;
use solana_program_test::BanksClientError;
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

    pub async fn send(&self) -> std::result::Result<(), BanksClientError> {
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
        adjusted_account
            .ensure_perp_position(affected_perp_market_index, QUOTE_TOKEN_INDEX)
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

// Verifies that the "post_health: ..." log emitted by the previous instruction
// matches the init health of the account.
pub async fn check_prev_instruction_post_health(solana: &SolanaCookie, account: Pubkey) {
    let logs = solana.program_log();
    let post_health_str = logs
        .iter()
        .find_map(|line| line.strip_prefix("post_init_health: "))
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

pub struct FlashLoanBeginInstruction {
    pub account: Pubkey,
    pub group: Pubkey,
    pub owner: TestKeypair,
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
            account: self.account,
            owner: self.owner.pubkey(),
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
        instruction.accounts.push(AccountMeta {
            pubkey: self.group,
            is_writable: false,
            is_signer: false,
        });

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![]
    }
}

pub struct FlashLoanEndInstruction {
    pub account: Pubkey,
    pub owner: TestKeypair,
    pub mango_token_bank: Pubkey,
    pub mango_token_vault: Pubkey,
    pub target_token_account: Pubkey,
    pub flash_loan_type: mango_v4::accounts_ix::FlashLoanType,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for FlashLoanEndInstruction {
    type Accounts = mango_v4::accounts::FlashLoanEnd;
    type Instruction = mango_v4::instruction::FlashLoanEnd;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            flash_loan_type: self.flash_loan_type,
        };

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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

    pub min_vault_to_deposits_ratio: f64,
    pub net_borrow_limit_per_window_quote: i64,
    pub net_borrow_limit_window_size_ts: u64,

    pub group: Pubkey,
    pub admin: TestKeypair,
    pub mint: Pubkey,
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
            min_vault_to_deposits_ratio: self.min_vault_to_deposits_ratio,
            net_borrow_limit_per_window_quote: self.net_borrow_limit_per_window_quote,
            net_borrow_limit_window_size_ts: self.net_borrow_limit_window_size_ts,
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
        // TODO: remove copy pasta of pda derivation, use reference
        let oracle = Pubkey::find_program_address(
            &[
                b"StubOracle".as_ref(),
                self.group.as_ref(),
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin]
    }
}

fn token_edit_instruction_default() -> mango_v4::instruction::TokenEdit {
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
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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
            reduce_only_opt: Some(true),
            ..token_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            mint_info: mint_info_key,
            oracle: mint_info.oracle,
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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
        // TODO: remove copy pasta of pda derivation, use reference
        let oracle = Pubkey::find_program_address(
            &[
                b"StubOracle".as_ref(),
                self.group.as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            oracle,
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

pub struct StubOracleCreate {
    pub group: Pubkey,
    pub mint: Pubkey,
    pub admin: TestKeypair,
    pub payer: TestKeypair,
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

        let oracle = Pubkey::find_program_address(
            &[
                b"StubOracle".as_ref(),
                self.group.as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            oracle,
            mint: self.mint,
            admin: self.admin.pubkey(),
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.payer, self.admin]
    }
}

pub struct StubOracleCloseInstruction {
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

        let oracle = Pubkey::find_program_address(
            &[
                b"StubOracle".as_ref(),
                self.group.as_ref(),
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.creator, self.payer]
    }
}

fn group_edit_instruction_default() -> mango_v4::instruction::GroupEdit {
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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
    pub group: Pubkey,
    pub owner: TestKeypair,
    pub payer: TestKeypair,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for AccountCreateInstruction {
    type Accounts = mango_v4::accounts::AccountCreate;
    type Instruction = mango_v4::instruction::AccountCreate;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = mango_v4::instruction::AccountCreate {
            account_num: self.account_num,
            token_count: self.token_count,
            serum3_count: self.serum3_count,
            perp_count: self.perp_count,
            perp_oo_count: self.perp_oo_count,
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

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner, self.payer]
    }
}

pub struct AccountExpandInstruction {
    pub account_num: u32,
    pub group: Pubkey,
    pub owner: TestKeypair,
    pub payer: TestKeypair,
    pub token_count: u8,
    pub serum3_count: u8,
    pub perp_count: u8,
    pub perp_oo_count: u8,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for AccountExpandInstruction {
    type Accounts = mango_v4::accounts::AccountExpand;
    type Instruction = mango_v4::instruction::AccountExpand;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = mango_v4::instruction::AccountExpand {
            token_count: self.token_count,
            serum3_count: self.serum3_count,
            perp_count: self.perp_count,
            perp_oo_count: self.perp_oo_count,
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner, self.payer]
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
            name_opt: Option::from(self.name.to_string()),
            delegate_opt: Option::from(self.delegate),
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin, self.payer]
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let health_check_metas = derive_health_check_remaining_account_metas(
            &account_loader,
            &account,
            None,
            false,
            None,
        )
        .await;

        let payer_info = &match self.side {
            Serum3Side::Bid => &quote_info,
            Serum3Side::Ask => &base_info,
        };

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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.owner]
    }
}

pub struct Serum3SettleFundsInstruction {
    pub account: Pubkey,
    pub owner: TestKeypair,

    pub serum_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for Serum3SettleFundsInstruction {
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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
            reduce_only_opt: Some(true),
            ..perp_edit_instruction_default()
        };

        let accounts = Self::Accounts {
            group: self.group,
            admin: self.admin.pubkey(),
            perp_market: self.perp_market,
            oracle: perp_market.oracle,
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for PerpPlaceOrderInstruction {
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
            order_type: PlaceOrderType::Limit,
            reduce_only: self.reduce_only,
            expiry_timestamp: 0,
            limit: 10,
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
        let mut instruction = make_instruction(program_id, &accounts, instruction);
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
    type Instruction = mango_v4::instruction::PerpPlaceOrderPegged;
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
        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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
        let instruction = Self::Instruction { limit: 5 };
        let perp_market: PerpMarket = account_loader.load(&self.perp_market).await.unwrap();
        let accounts = Self::Accounts {
            group: perp_market.group,
            account: self.account,
            perp_market: self.perp_market,
            bids: perp_market.bids,
            asks: perp_market.asks,
            owner: self.owner.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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
    pub settle_bank: Pubkey,
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
        let settle_bank: Bank = account_loader.load(&self.settle_bank).await.unwrap();
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

        let accounts = Self::Accounts {
            group: perp_market.group,
            settler: self.settler,
            settler_owner: self.settler_owner.pubkey(),
            perp_market: self.perp_market,
            account_a: self.account_a,
            account_b: self.account_b,
            oracle: perp_market.oracle,
            settle_bank: self.settle_bank,
            settle_oracle: settle_bank.oracle,
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas);

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.settler_owner]
    }
}

pub struct PerpSettleFeesInstruction {
    pub account: Pubkey,
    pub perp_market: Pubkey,
    pub settle_bank: Pubkey,
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
        let settle_bank: Bank = account_loader.load(&self.settle_bank).await.unwrap();
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
            group: perp_market.group,
            perp_market: self.perp_market,
            account: self.account,
            oracle: perp_market.oracle,
            settle_bank: self.settle_bank,
            settle_oracle: settle_bank.oracle,
        };
        let mut instruction = make_instruction(program_id, &accounts, instruction);
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
        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

        let group = account_loader.load::<Group>(&group_key).await.unwrap();
        let quote_mint_info = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                group_key.as_ref(),
                group.insurance_mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let quote_mint_info: MintInfo = account_loader.load(&quote_mint_info).await.unwrap();

        let accounts = Self::Accounts {
            group: group_key,
            perp_market: self.perp_market,
            oracle: perp_market.oracle,
            liqor: self.liqor,
            liqor_owner: self.liqor_owner.pubkey(),
            liqee: self.liqee,
            settle_bank: quote_mint_info.first_bank(),
            settle_vault: quote_mint_info.first_vault(),
            settle_oracle: quote_mint_info.oracle,
        };
        let mut instruction = make_instruction(program_id, &accounts, instruction);
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
    type Accounts = mango_v4::accounts::PerpLiqNegativePnlOrBankruptcy;
    type Instruction = mango_v4::instruction::PerpLiqNegativePnlOrBankruptcy;
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
        let quote_mint_info = Pubkey::find_program_address(
            &[
                b"MintInfo".as_ref(),
                group_key.as_ref(),
                group.insurance_mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let quote_mint_info: MintInfo = account_loader.load(&quote_mint_info).await.unwrap();

        let accounts = Self::Accounts {
            group: group_key,
            liqor: self.liqor,
            liqor_owner: self.liqor_owner.pubkey(),
            liqee: self.liqee,
            perp_market: self.perp_market,
            oracle: perp_market.oracle,
            settle_bank: quote_mint_info.first_bank(),
            settle_vault: quote_mint_info.first_vault(),
            settle_oracle: quote_mint_info.oracle,
            insurance_vault: group.insurance_vault,
            token_program: Token::id(),
        };
        let mut instruction = make_instruction(program_id, &accounts, instruction);
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
        let accounts = Self::Accounts {};

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

        let mut instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<TestKeypair> {
        vec![self.admin, self.payer]
    }
}
