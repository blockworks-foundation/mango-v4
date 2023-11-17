#![allow(dead_code)]

use std::sync::Arc;

use solana_sdk::pubkey::Pubkey;

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
pub struct OpenbookMarketCookie {
    pub market: Pubkey,
    pub event_heap: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub quote_vault: Pubkey,
    pub base_vault: Pubkey,
    pub authority: Pubkey,
    pub quote_mint: MintCookie,
    pub base_mint: MintCookie,
}

pub struct OpenbookV2Cookie {
    pub solana: Arc<solana::SolanaCookie>,
    pub program_id: Pubkey,
}

impl OpenbookV2Cookie {
    pub async fn list_spot_market(
        &self,
        quote_mint: &MintCookie,
        base_mint: &MintCookie,
        payer: TestKeypair,
    ) -> OpenbookMarketCookie {
        let collect_fee_admin = TestKeypair::new();
        let close_market_admin = TestKeypair::new();
        let market = TestKeypair::new();

        let res = openbook_client::send_tx(
            self.solana.as_ref(),
            CreateMarketInstruction {
                collect_fee_admin: collect_fee_admin.pubkey(),
                open_orders_admin: None,
                close_market_admin: Some(close_market_admin.pubkey()),
                payer: payer,
                market,
                quote_lot_size: 10,
                base_lot_size: 100,
                maker_fee: -200,
                taker_fee: 400,
                base_mint: base_mint.pubkey,
                quote_mint: quote_mint.pubkey,
                ..CreateMarketInstruction::with_new_book_and_heap(self.solana.as_ref(), None, None)
                    .await
            },
        )
        .await
        .unwrap();

        OpenbookMarketCookie {
            market: market.pubkey(),
            event_heap: res.event_heap,
            bids: res.bids,
            asks: res.asks,
            authority: res.market_authority,
            quote_vault: res.market_quote_vault,
            base_vault: res.market_base_vault,
            quote_mint: *quote_mint,
            base_mint: *base_mint,
        }
    }

    // pub async fn consume_spot_events(
    //     &self,
    //     spot_market_cookie: &OpenbookMarketCookie,
    //     open_orders: &[Pubkey],
    // ) {
    //     let mut sorted_oos = open_orders.to_vec();
    //     sorted_oos.sort_by_key(|key| serum_dex::state::ToAlignedBytes::to_aligned_bytes(key));

    //     let instructions = [serum_dex::instruction::consume_events(
    //         &self.program_id,
    //         sorted_oos.iter().collect(),
    //         &spot_market_cookie.market,
    //         &spot_market_cookie.event_q,
    //         &spot_market_cookie.coin_fee_account,
    //         &spot_market_cookie.pc_fee_account,
    //         10,
    //     )
    //     .unwrap()];
    //     self.solana
    //         .process_transaction(&instructions, None)
    //         .await
    //         .unwrap();
    // }
}
