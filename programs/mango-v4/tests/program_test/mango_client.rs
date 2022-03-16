#![allow(dead_code)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::{self, SysvarId};
use anchor_spl::dex::serum_dex;
use anchor_spl::token::{Token, TokenAccount};
use fixed::types::I80F48;
use solana_program::instruction::Instruction;
use solana_sdk::instruction;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transport::TransportError;
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
) -> std::result::Result<CI::Accounts, TransportError> {
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
    for position in account.token_account_map.iter_active() {
        let mint_info =
            get_mint_info_by_token_index(account_loader, account, position.token_index).await;
        let lookup_table = account_loader
            .load_bytes(&mint_info.address_lookup_table)
            .await
            .unwrap();
        let addresses = mango_v4::address_lookup_table::addresses(&lookup_table);
        banks.push(addresses[mint_info.address_lookup_table_bank_index as usize]);
        oracles.push(addresses[mint_info.address_lookup_table_oracle_index as usize]);
    }
    if let Some(affected_bank) = affected_bank {
        if banks.iter().find(|&&v| v == affected_bank).is_none() {
            // If there is not yet an active position for the token, we need to pass
            // the bank/oracle for health check anyway.
            let new_position = account
                .token_account_map
                .values
                .iter()
                .position(|p| !p.is_active())
                .unwrap();
            banks.insert(new_position, affected_bank);
            let affected_bank: Bank = account_loader.load(&affected_bank).await.unwrap();
            oracles.insert(new_position, affected_bank.oracle);
        }
    }

    let serum_oos = account
        .serum_account_map
        .iter_active()
        .map(|&s| s.open_orders);

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

fn from_serum_style_pubkey(d: &[u64; 4]) -> Pubkey {
    Pubkey::new(bytemuck::cast_slice(d as &[_]))
}

pub async fn account_position(solana: &SolanaCookie, account: Pubkey, bank: Pubkey) -> i64 {
    let account_data: MangoAccount = solana.get_account(account).await;
    let bank_data: Bank = solana.get_account(bank).await;
    let native = account_data
        .token_account_map
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
            banks_len: account.token_account_map.iter_active().count(),
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
    pub maint_asset_weight: f32,
    pub init_asset_weight: f32,
    pub maint_liab_weight: f32,
    pub init_liab_weight: f32,

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
            token_index: self.token_index,
            maint_asset_weight: self.maint_asset_weight,
            init_asset_weight: self.init_asset_weight,
            maint_liab_weight: self.maint_liab_weight,
            init_liab_weight: self.init_liab_weight,
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
        let oracle = Pubkey::find_program_address(
            &[b"StubOracle".as_ref(), self.mint.as_ref()],
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
            address_lookup_table: self.address_lookup_table,
            payer: self.payer.pubkey(),
            token_program: Token::id(),
            system_program: System::id(),
            address_lookup_table_program: mango_v4::address_lookup_table::id(),
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

        let oracle = Pubkey::find_program_address(
            &[b"StubOracle".as_ref(), self.mint.as_ref()],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts { oracle };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![]
    }
}

pub struct CreateStubOracle<'keypair> {
    pub mint: Pubkey,
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
            &[b"StubOracle".as_ref(), self.mint.as_ref()],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            oracle,
            token_mint: self.mint,
            payer: self.payer.pubkey(),
            system_program: System::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.payer]
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

pub struct RegisterSerumMarketInstruction<'keypair> {
    pub group: Pubkey,
    pub admin: &'keypair Keypair,
    pub payer: &'keypair Keypair,

    pub serum_program: Pubkey,
    pub serum_market_external: Pubkey,

    pub market_index: SerumMarketIndex,
    pub base_token_index: TokenIndex,
    pub quote_token_index: TokenIndex,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for RegisterSerumMarketInstruction<'keypair> {
    type Accounts = mango_v4::accounts::RegisterSerumMarket;
    type Instruction = mango_v4::instruction::RegisterSerumMarket;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            market_index: self.market_index,
            base_token_index: self.base_token_index,
            quote_token_index: self.quote_token_index,
        };

        let serum_market = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"SerumMarket".as_ref(),
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

pub struct CreateSerumOpenOrdersInstruction<'keypair> {
    pub account: Pubkey,
    pub serum_market: Pubkey,
    pub owner: &'keypair Keypair,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for CreateSerumOpenOrdersInstruction<'keypair> {
    type Accounts = mango_v4::accounts::CreateSerumOpenOrders;
    type Instruction = mango_v4::instruction::CreateSerumOpenOrders;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {};

        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();
        let serum_market: SerumMarket = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = Pubkey::find_program_address(
            &[
                self.account.as_ref(),
                b"SerumOO".as_ref(),
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

pub struct PlaceSerumOrderInstruction<'keypair> {
    pub side: u8,
    pub limit_price: u64,
    pub max_base_qty: u64,
    pub max_native_quote_qty_including_fees: u64,
    pub self_trade_behavior: u8,
    pub order_type: u8,
    pub client_order_id: u64,
    pub limit: u16,

    pub account: Pubkey,
    pub owner: &'keypair Keypair,

    pub serum_market: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for PlaceSerumOrderInstruction<'keypair> {
    type Accounts = mango_v4::accounts::PlaceSerumOrder;
    type Instruction = mango_v4::instruction::PlaceSerumOrder;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            order: mango_v4::instructions::NewOrderInstructionData(
                anchor_spl::dex::serum_dex::instruction::NewOrderInstructionV3 {
                    side: self.side.try_into().unwrap(),
                    limit_price: self.limit_price.try_into().unwrap(),
                    max_coin_qty: self.max_base_qty.try_into().unwrap(),
                    max_native_pc_qty_including_fees: self
                        .max_native_quote_qty_including_fees
                        .try_into()
                        .unwrap(),
                    self_trade_behavior: self.self_trade_behavior.try_into().unwrap(),
                    order_type: self.order_type.try_into().unwrap(),
                    client_order_id: self.client_order_id,
                    limit: self.limit,
                },
            ),
        };

        let account: MangoAccount = account_loader.load(&self.account).await.unwrap();
        let serum_market: SerumMarket = account_loader.load(&self.serum_market).await.unwrap();
        let open_orders = account
            .serum_account_map
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
            owner: self.owner.pubkey(),
            token_program: Token::id(),
            rent: sysvar::rent::Rent::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction.accounts.extend(health_check_metas.into_iter());

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}
