#![allow(dead_code)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::{self, SysvarId};
use anchor_spl::token::{Token, TokenAccount};
use fixed::types::I80F48;
use itertools::Itertools;
use mango_v4::instructions::{
    InterestRateParams, Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side,
};
use solana_program::instruction::Instruction;
use solana_program_test::BanksClientError;
use solana_sdk::instruction;
use solana_sdk::signature::{Keypair, Signer};

use std::str::FromStr;

use super::solana::SolanaCookie;
use mango_v4::state::*;

#[async_trait::async_trait(?Send)]
pub trait ClientAccountLoader {
    async fn load_bytes(&self, pubkey: &Pubkey) -> Option<Vec<u8>>;
    async fn load<T: AccountDeserialize>(&self, pubkey: &Pubkey) -> Option<T> {
        let bytes = self.load_bytes(pubkey).await?;
        AccountDeserialize::try_deserialize(&mut &bytes[..]).ok()
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
) -> std::result::Result<CI::Accounts, BanksClientError> {
    let (accounts, instruction) = ix.to_instruction(solana).await;
    let signers = ix.signers();
    let instructions = vec![instruction];
    solana
        .process_transaction(&instructions, Some(&signers[..]))
        .await?;
    Ok(accounts)
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
    account: &MangoAccount,
    mint: Pubkey,
) -> MintInfo {
    let mint_info_pk = Pubkey::find_program_address(
        &[account.group.as_ref(), b"MintInfo".as_ref(), mint.as_ref()],
        &mango_v4::id(),
    )
    .0;
    account_loader.load(&mint_info_pk).await.unwrap()
}

async fn get_mint_info_by_token_index(
    account_loader: &impl ClientAccountLoader,
    account: &MangoAccount,
    token_index: TokenIndex,
) -> MintInfo {
    let bank_pk = Pubkey::find_program_address(
        &[
            account.group.as_ref(),
            b"Bank".as_ref(),
            &token_index.to_le_bytes(),
        ],
        &mango_v4::id(),
    )
    .0;
    let bank: Bank = account_loader.load(&bank_pk).await.unwrap();
    get_mint_info_by_mint(account_loader, account, bank.mint).await
}

// all the accounts that instructions like deposit/withdraw need to compute account health
async fn derive_health_check_remaining_account_metas(
    account_loader: &impl ClientAccountLoader,
    account: &MangoAccount,
    affected_bank: Option<Pubkey>,
    writable_banks: bool,
) -> Vec<AccountMeta> {
    // figure out all the banks/oracles that need to be passed for the health check
    let mut banks = vec![];
    let mut oracles = vec![];
    for position in account.tokens.iter_active() {
        let mint_info =
            get_mint_info_by_token_index(account_loader, account, position.token_index).await;
        // TODO: ALTs are unavailable
        // let lookup_table = account_loader
        //     .load_bytes(&mint_info.address_lookup_table)
        //     .await
        //     .unwrap();
        // let addresses = mango_v4::address_lookup_table::addresses(&lookup_table);
        // banks.push(addresses[mint_info.address_lookup_table_bank_index as usize]);
        // oracles.push(addresses[mint_info.address_lookup_table_oracle_index as usize]);
        banks.push(mint_info.bank);
        oracles.push(mint_info.oracle);
    }
    if let Some(affected_bank) = affected_bank {
        if banks.iter().find(|&&v| v == affected_bank).is_none() {
            // If there is not yet an active position for the token, we need to pass
            // the bank/oracle for health check anyway.
            let new_position = account
                .tokens
                .values
                .iter()
                .position(|p| !p.is_active())
                .unwrap();
            banks.insert(new_position, affected_bank);
            let affected_bank: Bank = account_loader.load(&affected_bank).await.unwrap();
            oracles.insert(new_position, affected_bank.oracle);
        }
    }

    let serum_oos = account.serum3.iter_active().map(|&s| s.open_orders);

    banks
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
        .collect()
}

async fn derive_liquidation_remaining_account_metas(
    account_loader: &impl ClientAccountLoader,
    liqee: &MangoAccount,
    liqor: &MangoAccount,
    asset_token_index: TokenIndex,
    liab_token_index: TokenIndex,
) -> Vec<AccountMeta> {
    let mut banks = vec![];
    let mut oracles = vec![];
    let token_indexes = liqee
        .tokens
        .iter_active()
        .chain(liqor.tokens.iter_active())
        .map(|ta| ta.token_index)
        .unique();
    for token_index in token_indexes {
        let mint_info = get_mint_info_by_token_index(account_loader, liqee, token_index).await;
        let writable_bank = token_index == asset_token_index || token_index == liab_token_index;
        // TODO: ALTs are unavailable
        // let lookup_table = account_loader
        //     .load_bytes(&mint_info.address_lookup_table)
        //     .await
        //     .unwrap();
        // let addresses = mango_v4::address_lookup_table::addresses(&lookup_table);
        // banks.push((
        //     addresses[mint_info.address_lookup_table_bank_index as usize],
        //     writable_bank,
        // ));
        // oracles.push(addresses[mint_info.address_lookup_table_oracle_index as usize]);
        banks.push((mint_info.bank, writable_bank));
        oracles.push(mint_info.oracle);
    }

    let serum_oos = liqee
        .serum3
        .iter_active()
        .chain(liqor.serum3.iter_active())
        .map(|&s| s.open_orders);

    banks
        .iter()
        .map(|(pubkey, is_writable)| AccountMeta {
            pubkey: *pubkey,
            is_writable: *is_writable,
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
        .collect()
}

fn from_serum_style_pubkey(d: &[u64; 4]) -> Pubkey {
    Pubkey::new(bytemuck::cast_slice(d as &[_]))
}

pub async fn account_position(solana: &SolanaCookie, account: Pubkey, bank: Pubkey) -> i64 {
    let account_data: MangoAccount = solana.get_account(account).await;
    let bank_data: Bank = solana.get_account(bank).await;
    let native = account_data
        .tokens
        .find(bank_data.token_index)
        .unwrap()
        .native(&bank_data);
    native.round().to_num::<i64>()
}

//
// a struct for each instruction along with its
// ClientInstruction impl
//

pub struct MarginTradeInstruction<'keypair> {
    pub account: Pubkey,
    pub owner: &'keypair Keypair,
    pub mango_token_vault: Pubkey,
    pub margin_trade_program_id: Pubkey,
    pub deposit_account: Pubkey,
    pub deposit_account_owner: Pubkey,
    pub margin_trade_program_ix_cpi_data: Vec<u8>,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for MarginTradeInstruction<'keypair> {
    type Accounts = mango_v4::accounts::MarginTrade;
    type Instruction = mango_v4::instruction::MarginTrade;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();

        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();

        let instruction = Self::Instruction {
            banks_len: account.tokens.iter_active().count(),
            cpi_data: self.margin_trade_program_ix_cpi_data.clone(),
        };

        let accounts = Self::Accounts {
            group: account.group,
            account: self.account,
            owner: self.owner.pubkey(),
        };

        let health_check_metas =
            derive_health_check_remaining_account_metas(&account_loader, &account, None, true)
                .await;

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());
        instruction.accounts.push(AccountMeta {
            pubkey: self.margin_trade_program_id,
            is_writable: false,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: self.mango_token_vault,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: self.deposit_account,
            is_writable: true,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: self.deposit_account_owner,
            is_writable: false,
            is_signer: false,
        });
        instruction.accounts.push(AccountMeta {
            pubkey: spl_token::ID,
            is_writable: false,
            is_signer: false,
        });

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct WithdrawInstruction<'keypair> {
    pub amount: u64,
    pub allow_borrow: bool,

    pub account: Pubkey,
    pub owner: &'keypair Keypair,
    pub token_account: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for WithdrawInstruction<'keypair> {
    type Accounts = mango_v4::accounts::Withdraw;
    type Instruction = mango_v4::instruction::Withdraw;
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
        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();
        let mint_info = Pubkey::find_program_address(
            &[
                account.group.as_ref(),
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
            Some(mint_info.bank),
            false,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.group,
            account: self.account,
            owner: self.owner.pubkey(),
            bank: mint_info.bank,
            vault: mint_info.vault,
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

pub struct DepositInstruction<'keypair> {
    pub amount: u64,

    pub account: Pubkey,
    pub token_account: Pubkey,
    pub token_authority: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for DepositInstruction<'keypair> {
    type Accounts = mango_v4::accounts::Deposit;
    type Instruction = mango_v4::instruction::Deposit;
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
        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();
        let mint_info = Pubkey::find_program_address(
            &[
                account.group.as_ref(),
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
            Some(mint_info.bank),
            false,
        )
        .await;

        let accounts = Self::Accounts {
            group: account.group,
            account: self.account,
            bank: mint_info.bank,
            vault: mint_info.vault,
            token_account: self.token_account,
            token_authority: self.token_authority.pubkey(),
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.token_authority]
    }
}

pub struct RegisterTokenInstruction<'keypair> {
    pub token_index: TokenIndex,
    pub decimals: u8,
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
    pub address_lookup_table: Pubkey,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for RegisterTokenInstruction<'keypair> {
    type Accounts = mango_v4::accounts::RegisterToken;
    type Instruction = mango_v4::instruction::RegisterToken;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            name: "some_ticker".to_string(),
            token_index: self.token_index,
            interest_rate_params: InterestRateParams {
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
            ],
            &program_id,
        )
        .0;
        let vault = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"Vault".as_ref(),
                &self.token_index.to_le_bytes(),
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
            // TODO: ALTs are unavailable
            //address_lookup_table: self.address_lookup_table,
            payer: self.payer.pubkey(),
            token_program: Token::id(),
            system_program: System::id(),
            // TODO: ALTs are unavailable
            //address_lookup_table_program: mango_v4::address_lookup_table::id(),
            rent: sysvar::rent::Rent::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin, self.payer]
    }
}

pub struct SetStubOracle<'keypair> {
    pub mint: Pubkey,
    pub group: Pubkey,
    pub admin: &'keypair Keypair,
    pub payer: &'keypair Keypair,
    pub price: &'static str,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for SetStubOracle<'keypair> {
    type Accounts = mango_v4::accounts::SetStubOracle;
    type Instruction = mango_v4::instruction::SetStubOracle;

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

pub struct CreateStubOracle<'keypair> {
    pub group: Pubkey,
    pub mint: Pubkey,
    pub admin: &'keypair Keypair,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for CreateStubOracle<'keypair> {
    type Accounts = mango_v4::accounts::CreateStubOracle;
    type Instruction = mango_v4::instruction::CreateStubOracle;

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

pub struct CreateGroupInstruction<'keypair> {
    pub admin: &'keypair Keypair,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for CreateGroupInstruction<'keypair> {
    type Accounts = mango_v4::accounts::CreateGroup;
    type Instruction = mango_v4::instruction::CreateGroup;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let group = Pubkey::find_program_address(
            &[b"Group".as_ref(), self.admin.pubkey().as_ref()],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group,
            admin: self.admin.pubkey(),
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

pub struct CreateAccountInstruction<'keypair> {
    pub account_num: u8,

    pub group: Pubkey,
    pub owner: &'keypair Keypair,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for CreateAccountInstruction<'keypair> {
    type Accounts = mango_v4::accounts::CreateAccount;
    type Instruction = mango_v4::instruction::CreateAccount;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = mango_v4::instruction::CreateAccount {
            account_num: self.account_num,
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

        let accounts = mango_v4::accounts::CreateAccount {
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

pub struct CloseAccountInstruction<'keypair> {
    pub account: Pubkey,
    pub owner: &'keypair Keypair,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for CloseAccountInstruction<'keypair> {
    type Accounts = mango_v4::accounts::CloseAccount;
    type Instruction = mango_v4::instruction::CloseAccount;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let accounts = Self::Accounts {
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

        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3
            .find(serum_market.market_index)
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

        let health_check_metas =
            derive_health_check_remaining_account_metas(&account_loader, &account, None, false)
                .await;

        let accounts = Self::Accounts {
            group: account.group,
            account: self.account,
            open_orders,
            quote_bank: quote_info.bank,
            quote_vault: quote_info.vault,
            base_bank: base_info.bank,
            base_vault: base_info.vault,
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

        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3
            .find(serum_market.market_index)
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
            group: account.group,
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

        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3
            .find(serum_market.market_index)
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
            group: account.group,
            account: self.account,
            open_orders,
            quote_bank: quote_info.bank,
            quote_vault: quote_info.vault,
            base_bank: base_info.bank,
            base_vault: base_info.vault,
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

        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();
        let serum_market: Serum3Market = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum3
            .find(serum_market.market_index)
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

        let health_check_metas =
            derive_health_check_remaining_account_metas(&account_loader, &account, None, false)
                .await;

        let accounts = Self::Accounts {
            group: account.group,
            account: self.account,
            open_orders,
            quote_bank: quote_info.bank,
            quote_vault: quote_info.vault,
            base_bank: base_info.bank,
            base_vault: base_info.vault,
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
    pub liab_token_index: TokenIndex,
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

        let liqee: MangoAccount = account_loader.load(&self.liqee).await.unwrap();
        let liqor: MangoAccount = account_loader.load(&self.liqor).await.unwrap();
        let health_check_metas = derive_liquidation_remaining_account_metas(
            &account_loader,
            &liqee,
            &liqor,
            self.asset_token_index,
            self.liab_token_index,
        )
        .await;

        let accounts = Self::Accounts {
            group: liqee.group,
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
    pub quote_token_index: TokenIndex,
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
            perp_market_index: self.perp_market_index,
            base_token_index_opt: Option::from(self.base_token_index),
            quote_token_index: self.quote_token_index,
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
        _loader: impl ClientAccountLoader + 'async_trait,
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

        let instruction = make_instruction(program_id, &accounts, instruction);
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
pub struct UpdateIndexInstruction {
    pub bank: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for UpdateIndexInstruction {
    type Accounts = mango_v4::accounts::UpdateIndex;
    type Instruction = mango_v4::instruction::UpdateIndex;
    async fn to_instruction(
        &self,
        _loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};
        let accounts = Self::Accounts { bank: self.bank };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![]
    }
}
