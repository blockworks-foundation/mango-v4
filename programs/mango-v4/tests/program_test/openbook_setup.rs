#![allow(dead_code)]

use std::sync::Arc;

use bytemuck::cast_ref;
use itertools::Itertools;
use openbook_client::*;
use openbook_v2::state::{EventHeap, EventType, FillEvent, OpenOrdersAccount, OutEvent};
use solana_sdk::pubkey::Pubkey;

use super::*;

pub struct OpenbookListingKeys {
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
        let market = TestKeypair::new();

        let res = openbook_client::send_openbook_tx(
            self.solana.as_ref(),
            CreateMarketInstruction {
                collect_fee_admin: collect_fee_admin.pubkey(),
                open_orders_admin: None,
                close_market_admin: None,
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

    pub async fn load_open_orders(&self, address: Pubkey) -> OpenOrdersAccount {
        self.solana.get_account::<OpenOrdersAccount>(address).await
    }

    pub async fn consume_spot_events(&self, spot_market_cookie: &OpenbookMarketCookie, limit: u8) {
        let event_heap = self
            .solana
            .get_account_boxed::<EventHeap>(spot_market_cookie.event_heap)
            .await;
        let to_consume = event_heap
            .iter()
            .map(|(event, _slot)| event)
            .take(limit as usize)
            .collect_vec();
        let open_orders_accounts = to_consume
            .into_iter()
            .map(
                |event| match EventType::try_from(event.event_type).unwrap() {
                    EventType::Fill => {
                        let fill: &FillEvent = cast_ref(event);
                        fill.maker
                    }
                    EventType::Out => {
                        let out: &OutEvent = cast_ref(event);
                        out.owner
                    }
                },
            )
            .collect_vec();

        openbook_client::send_openbook_tx(
            self.solana.as_ref(),
            ConsumeEventsInstruction {
                consume_events_admin: None,
                market: spot_market_cookie.market,
                open_orders_accounts,
            },
        )
        .await
        .unwrap();
    }
}
