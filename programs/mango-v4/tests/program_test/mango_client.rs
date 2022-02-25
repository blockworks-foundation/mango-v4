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
    let (program_id, accounts, instruction) = ix.to_instruction(solana).await;
    let signers = ix.signers();
    let instructions = vec![instruction::Instruction {
        program_id,
        accounts: anchor_lang::ToAccountMetas::to_account_metas(&accounts, None),
        data: anchor_lang::InstructionData::data(&instruction),
    }];
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
    ) -> (Pubkey, Self::Accounts, Self::Instruction);
    fn signers(&self) -> Vec<&Keypair>;
}

//
// a struct for each instruction along with its
// ClientInstruction impl
//

pub struct DepositInstruction<'keypair> {
    pub amount: u64,

    pub group: Pubkey,
    pub account: Pubkey,
    pub deposit_token: Pubkey,
    pub deposit_authority: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for DepositInstruction<'keypair> {
    type Accounts = mango_v4::accounts::Deposit;
    type Instruction = mango_v4::instruction::Deposit;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Pubkey, Self::Accounts, Self::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            amount: self.amount,
        };

        // load deposit_token so we know its mint
        let deposit_token: TokenAccount = account_loader.load(&self.deposit_token).await.unwrap();

        let bank = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"tokenbank".as_ref(),
                deposit_token.mint.as_ref(),
            ],
            &program_id,
        )
        .0;
        let vault = Pubkey::find_program_address(
            &[
                self.group.as_ref(),
                b"tokenvault".as_ref(),
                deposit_token.mint.as_ref(),
            ],
            &program_id,
        )
        .0;

        let accounts = Self::Accounts {
            group: self.group,
            account: self.account,
            bank,
            vault,
            deposit_token: self.deposit_token,
            deposit_authority: self.deposit_authority.pubkey(),
            token_program: Token::id(),
        };

        (program_id, accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.deposit_authority]
    }
}

pub struct RegisterTokenInstruction<'keypair> {
    pub decimals: u8,

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
    ) -> (Pubkey, Self::Accounts, Self::Instruction) {
        let program_id = mango_v4::id();
        let instruction = Self::Instruction {
            decimals: self.decimals,
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

        (program_id, accounts, instruction)
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
    ) -> (Pubkey, Self::Accounts, Self::Instruction) {
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

        (program_id, accounts, instruction)
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
    ) -> (Pubkey, Self::Accounts, Self::Instruction) {
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

        (program_id, accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.owner, self.payer]
    }
}
