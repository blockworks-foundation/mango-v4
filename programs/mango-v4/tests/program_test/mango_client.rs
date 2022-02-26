use super::solana::SolanaCookie;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::{self, SysvarId};
use anchor_lang::Key;
use anchor_spl::token::{Token, TokenAccount};
use solana_sdk::instruction;
use solana_sdk::signature::{Keypair, Signer};

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
pub async fn send_tx<CI: ClientInstruction>(solana: &SolanaCookie, ix: CI) -> CI::Accounts {
    let (accounts, instruction) = ix.to_instruction(solana).await;
    let signers = ix.signers();
    let instructions = vec![instruction];
    solana
        .process_transaction(&instructions, Some(&signers[..]))
        .await
        .unwrap();
    accounts
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

//
// a struct for each instruction along with its
// ClientInstruction impl
//

pub struct WithdrawInstruction<'keypair> {
    pub amount: u64,
    pub allow_borrow: bool,

    pub group: Pubkey,
    pub account: Pubkey,
    pub owner: &'keypair Keypair,
    pub token_account: Pubkey,

    pub banks: Vec<Pubkey>,
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

        // load account so we know its mint
        let token_account: TokenAccount = account_loader.load(&self.token_account).await.unwrap();

        let bank = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"tokenbank".as_ref(),
                token_account.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let vault = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"tokenvault".as_ref(),
                token_account.mint.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            account: self.account,
            owner: self.owner.pubkey(),
            bank,
            vault,
            token_account: self.token_account,
            token_program: Token::id(),
        };

        let mut instruction = make_instruction(program_id, &accounts, instruction);
        instruction
            .accounts
            .extend(self.banks.iter().map(|&pubkey| AccountMeta {
                pubkey,
                is_writable: false,
                is_signer: false,
            }));

        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner]
    }
}

pub struct DepositInstruction<'keypair> {
    pub amount: u64,

    pub group: Pubkey,
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

        let bank = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"tokenbank".as_ref(),
                token_account.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let vault = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"tokenvault".as_ref(),
                token_account.mint.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            account: self.account,
            bank,
            vault,
            token_account: self.token_account,
            token_authority: self.token_authority.pubkey(),
            token_program: Token::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.token_authority]
    }
}

pub struct RegisterTokenInstruction<'keypair> {
    pub decimals: u8,
    pub maint_asset_weight: f32,
    pub init_asset_weight: f32,
    pub maint_liab_weight: f32,
    pub init_liab_weight: f32,

    pub group: Pubkey,
    pub admin: &'keypair Keypair,
    pub mint: Pubkey,
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
            decimals: self.decimals,
            maint_asset_weight: self.maint_asset_weight,
            init_asset_weight: self.init_asset_weight,
            maint_liab_weight: self.maint_liab_weight,
            init_liab_weight: self.init_liab_weight,
        };

        let bank = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"tokenbank".as_ref(),
                self.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let vault = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"tokenvault".as_ref(),
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
            &[b"group".as_ref(), self.admin.pubkey().as_ref()],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group,
            admin: self.admin.pubkey(),
            payer: self.payer.pubkey(),
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
                b"account".as_ref(),
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
            rent: sysvar::rent::Rent::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner, self.payer]
    }
}
