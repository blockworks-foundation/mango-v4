#![allow(dead_code)]

use std::{mem, sync::Arc};

use bytemuck::from_bytes;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{instruction::Instruction, signature::Signer};

use super::*;

pub struct ListingKeys {
    market_key: TestKeypair,
    req_q_key: TestKeypair,
    event_q_key: TestKeypair,
    bids_key: TestKeypair,
    asks_key: TestKeypair,
    vault_signer_pk: Pubkey,
    vault_signer_nonce: u64,
}

#[derive(Clone, Debug)]
pub struct SpotMarketCookie {
    pub market: Pubkey,
    pub req_q: Pubkey,
    pub event_q: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub coin_vault: Pubkey,
    pub pc_vault: Pubkey,
    pub vault_signer_key: Pubkey,
    pub coin_mint: MintCookie,
    pub pc_mint: MintCookie,
    pub coin_fee_account: Pubkey,
    pub pc_fee_account: Pubkey,
}

pub struct SerumCookie {
    pub solana: Arc<solana::SolanaCookie>,
    pub program_id: Pubkey,
}

impl SerumCookie {
    pub fn create_dex_account(&self, unpadded_len: usize) -> (TestKeypair, Instruction) {
        let serum_program_id = self.program_id;
        let key = TestKeypair::new();
        let len = unpadded_len + 12;
        let rent = self.solana.rent.minimum_balance(len);
        let create_account_instr = solana_sdk::system_instruction::create_account(
            &self.solana.context.borrow().payer.pubkey(),
            &key.pubkey(),
            rent,
            len as u64,
            &serum_program_id,
        );
        return (key, create_account_instr);
    }

    fn gen_listing_params(
        &self,
        _coin_mint: &Pubkey,
        _pc_mint: &Pubkey,
    ) -> (ListingKeys, Vec<Instruction>) {
        let serum_program_id = self.program_id;
        // let payer_pk = &self.context.payer.pubkey();

        let (market_key, create_market) = self.create_dex_account(376);
        let (req_q_key, create_req_q) = self.create_dex_account(640);
        let (event_q_key, create_event_q) = self.create_dex_account(1 << 20);
        let (bids_key, create_bids) = self.create_dex_account(1 << 16);
        let (asks_key, create_asks) = self.create_dex_account(1 << 16);

        let (vault_signer_pk, vault_signer_nonce) =
            create_signer_key_and_nonce(&serum_program_id, &market_key.pubkey());

        let info = ListingKeys {
            market_key,
            req_q_key,
            event_q_key,
            bids_key,
            asks_key,
            vault_signer_pk,
            vault_signer_nonce,
        };
        let instructions = vec![
            create_market,
            create_req_q,
            create_event_q,
            create_bids,
            create_asks,
        ];
        return (info, instructions);
    }

    pub async fn list_spot_market(
        &self,
        coin_mint: &MintCookie,
        pc_mint: &MintCookie,
    ) -> SpotMarketCookie {
        let serum_program_id = self.program_id;
        let coin_mint_pk = coin_mint.pubkey;
        let pc_mint_pk = pc_mint.pubkey;
        let (listing_keys, mut instructions) = self.gen_listing_params(&coin_mint_pk, &pc_mint_pk);
        let ListingKeys {
            market_key,
            req_q_key,
            event_q_key,
            bids_key,
            asks_key,
            vault_signer_pk,
            vault_signer_nonce,
        } = listing_keys;

        let coin_vault = self
            .solana
            .create_token_account(&vault_signer_pk, coin_mint_pk)
            .await;
        let pc_vault = self
            .solana
            .create_token_account(&listing_keys.vault_signer_pk, pc_mint_pk)
            .await;

        let init_market_instruction = serum_dex::instruction::initialize_market(
            &market_key.pubkey(),
            &serum_program_id,
            &coin_mint_pk,
            &pc_mint_pk,
            &coin_vault,
            &pc_vault,
            None,
            None,
            None,
            &bids_key.pubkey(),
            &asks_key.pubkey(),
            &req_q_key.pubkey(),
            &event_q_key.pubkey(),
            coin_mint.base_lot as u64,
            coin_mint.quote_lot as u64,
            vault_signer_nonce,
            100,
        )
        .unwrap();

        instructions.push(init_market_instruction);

        let signers = vec![
            market_key,
            req_q_key,
            event_q_key,
            bids_key,
            asks_key,
            req_q_key,
            event_q_key,
        ];

        self.solana
            .process_transaction(&instructions, Some(&signers))
            .await
            .unwrap();

        let fee_account_owner = Pubkey::new_unique();
        let coin_fee_account = self
            .solana
            .create_token_account(&fee_account_owner, coin_mint.pubkey)
            .await;
        let pc_fee_account = self
            .solana
            .create_token_account(&fee_account_owner, coin_mint.pubkey)
            .await;

        SpotMarketCookie {
            market: market_key.pubkey(),
            req_q: req_q_key.pubkey(),
            event_q: event_q_key.pubkey(),
            bids: bids_key.pubkey(),
            asks: asks_key.pubkey(),
            coin_vault: coin_vault,
            pc_vault: pc_vault,
            vault_signer_key: vault_signer_pk,
            coin_mint: coin_mint.clone(),
            pc_mint: pc_mint.clone(),
            coin_fee_account,
            pc_fee_account,
        }
    }

    pub async fn consume_spot_events(
        &self,
        spot_market_cookie: &SpotMarketCookie,
        open_orders: &[Pubkey],
    ) {
        let mut sorted_oos = open_orders.to_vec();
        sorted_oos.sort_by_key(|key| serum_dex::state::ToAlignedBytes::to_aligned_bytes(key));

        let instructions = [serum_dex::instruction::consume_events(
            &self.program_id,
            sorted_oos.iter().collect(),
            &spot_market_cookie.market,
            &spot_market_cookie.event_q,
            &spot_market_cookie.coin_fee_account,
            &spot_market_cookie.pc_fee_account,
            10,
        )
        .unwrap()];
        self.solana
            .process_transaction(&instructions, None)
            .await
            .unwrap();
    }

    fn strip_dex_padding(data: &[u8]) -> &[u8] {
        assert!(data.len() >= 12);
        &data[5..data.len() - 7]
    }

    pub async fn load_open_orders(&self, open_orders: Pubkey) -> serum_dex::state::OpenOrders {
        let data = self.solana.get_account_data(open_orders).await.unwrap();
        let slice = Self::strip_dex_padding(&data);
        assert_eq!(slice.len(), mem::size_of::<serum_dex::state::OpenOrders>());
        from_bytes::<serum_dex::state::OpenOrders>(slice).clone()
    }
}
