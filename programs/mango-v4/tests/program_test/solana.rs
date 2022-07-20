use std::cell::RefCell;
use std::sync::{Arc, RwLock};

use anchor_lang::AccountDeserialize;
use anchor_spl::token::TokenAccount;
use solana_program::{program_pack::Pack, rent::*, system_instruction};
use solana_program_test::*;
use solana_sdk::transport::TransportError;
use solana_sdk::{
    account::ReadableAccount,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token::*;

pub struct SolanaCookie {
    pub context: RefCell<ProgramTestContext>,
    pub rent: Rent,
    pub program_log: Arc<RwLock<Vec<String>>>,
}

impl SolanaCookie {
    #[allow(dead_code)]
    pub async fn process_transaction(
        &self,
        instructions: &[Instruction],
        signers: Option<&[&Keypair]>,
    ) -> Result<(), BanksClientError> {
        self.program_log.write().unwrap().clear();

        let mut context = self.context.borrow_mut();

        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&context.payer.pubkey()));

        let mut all_signers = vec![&context.payer];

        if let Some(signers) = signers {
            all_signers.extend_from_slice(signers);
        }

        // This fails when warping is involved - https://gitmemory.com/issue/solana-labs/solana/18201/868325078
        // let recent_blockhash = self.context.banks_client.get_recent_blockhash().await.unwrap();

        transaction.sign(&all_signers, context.last_blockhash);

        context
            .banks_client
            .process_transaction_with_commitment(
                transaction,
                solana_sdk::commitment_config::CommitmentLevel::Processed,
            )
            .await
    }

    pub async fn get_clock(&self) -> solana_program::clock::Clock {
        self.context
            .borrow_mut()
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
    }

    #[allow(dead_code)]
    pub async fn advance_by_slots(&self, slots: u64) {
        let clock = self.get_clock().await;
        self.context
            .borrow_mut()
            .warp_to_slot(clock.slot + slots + 1)
            .unwrap();
    }

    #[allow(dead_code)]

    pub async fn advance_clock(&self) {
        let mut clock = self.get_clock().await;
        let old_ts = clock.unix_timestamp;

        // just advance enough to ensure we get changes over last_updated in various ix
        // if this gets too slow for our tests, remove and replace with manual time offset
        // which is configurable
        while clock.unix_timestamp <= old_ts {
            self.context
                .borrow_mut()
                .warp_to_slot(clock.slot + 50)
                .unwrap();
            clock = self.get_clock().await;
        }
    }

    pub async fn get_newest_slot_from_history(&self) -> u64 {
        self.context
            .borrow_mut()
            .banks_client
            .get_sysvar::<solana_program::slot_history::SlotHistory>()
            .await
            .unwrap()
            .newest()
    }

    pub async fn create_account_from_len(&self, owner: &Pubkey, len: usize) -> Pubkey {
        let key = Keypair::new();
        let rent = self.rent.minimum_balance(len);
        let create_account_instr = solana_sdk::system_instruction::create_account(
            &self.context.borrow().payer.pubkey(),
            &key.pubkey(),
            rent,
            len as u64,
            &owner,
        );
        self.process_transaction(&[create_account_instr], Some(&[&key]))
            .await
            .unwrap();
        key.pubkey()
    }

    pub async fn create_account_for_type<T>(&self, owner: &Pubkey) -> Pubkey {
        let key = Keypair::new();
        let len = 8 + std::mem::size_of::<T>();
        let rent = self.rent.minimum_balance(len);
        let create_account_instr = solana_sdk::system_instruction::create_account(
            &self.context.borrow().payer.pubkey(),
            &key.pubkey(),
            rent,
            len as u64,
            &owner,
        );
        self.process_transaction(&[create_account_instr], Some(&[&key]))
            .await
            .unwrap();
        key.pubkey()
    }

    #[allow(dead_code)]
    pub async fn create_token_account(&self, owner: &Pubkey, mint: Pubkey) -> Pubkey {
        let keypair = Keypair::new();
        let rent = self.rent.minimum_balance(spl_token::state::Account::LEN);

        let instructions = [
            system_instruction::create_account(
                &self.context.borrow().payer.pubkey(),
                &keypair.pubkey(),
                rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &keypair.pubkey(),
                &mint,
                owner,
            )
            .unwrap(),
        ];

        self.process_transaction(&instructions, Some(&[&keypair]))
            .await
            .unwrap();
        return keypair.pubkey();
    }

    // Note: Only one table can be created per authority per slot!
    #[allow(dead_code)]
    pub async fn create_address_lookup_table(
        &self,
        authority: &Keypair,
        payer: &Keypair,
    ) -> Pubkey {
        let (instruction, alt_address) = mango_v4::address_lookup_table::create_lookup_table(
            authority.pubkey(),
            payer.pubkey(),
            self.get_newest_slot_from_history().await,
        );
        self.process_transaction(&[instruction], Some(&[authority, payer]))
            .await
            .unwrap();
        alt_address
    }

    #[allow(dead_code)]
    pub async fn get_account_data(&self, address: Pubkey) -> Option<Vec<u8>> {
        Some(
            self.context
                .borrow_mut()
                .banks_client
                .get_account(address)
                .await
                .unwrap()?
                .data()
                .to_vec(),
        )
    }

    #[allow(dead_code)]
    pub async fn get_account_opt<T: AccountDeserialize>(&self, address: Pubkey) -> Option<T> {
        self.context
            .borrow_mut()
            .banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap();

        let data = self.get_account_data(address).await?;
        let mut data_slice: &[u8] = &data;
        AccountDeserialize::try_deserialize(&mut data_slice).ok()
    }

    #[allow(dead_code)]
    pub async fn get_account<T: AccountDeserialize>(&self, address: Pubkey) -> T {
        self.get_account_opt(address).await.unwrap()
    }

    #[allow(dead_code)]
    pub async fn token_account_balance(&self, address: Pubkey) -> u64 {
        self.get_account::<TokenAccount>(address).await.amount
    }

    #[allow(dead_code)]
    pub fn program_log(&self) -> Vec<String> {
        self.program_log.read().unwrap().clone()
    }
}
