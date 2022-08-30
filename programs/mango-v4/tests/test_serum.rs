#![cfg(feature = "test-bpf")]
#![allow(dead_code)]

use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transport::TransportError};

use mango_v4::instructions::{
    OpenOrdersSlim, Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side,
};
use mango_v4::state::Serum3Orders;
use program_test::*;

mod program_test;

use mango_setup::*;
use std::sync::Arc;

struct SerumOrderPlacer {
    solana: Arc<SolanaCookie>,
    serum: Arc<SerumCookie>,
    account: Pubkey,
    owner: Keypair,
    serum_market: Pubkey,
    open_orders: Pubkey,
    next_client_order_id: u64,
}

impl SerumOrderPlacer {
    fn inc_client_order_id(&mut self) -> u64 {
        let id = self.next_client_order_id;
        self.next_client_order_id += 1;
        id
    }

    async fn find_order_id_for_client_order_id(&self, client_order_id: u64) -> Option<(u128, u64)> {
        let open_orders = self.serum.load_open_orders(self.open_orders).await;
        for i in 0..128 {
            if open_orders.free_slot_bits & (1u128 << i) != 0 {
                continue;
            }
            if open_orders.client_order_ids[i] == client_order_id {
                return Some((open_orders.orders[i], client_order_id));
            }
        }
        None
    }

    async fn bid(&mut self, limit_price: f64, max_base: u64) -> Option<(u128, u64)> {
        let client_order_id = self.inc_client_order_id();
        send_tx(
            &self.solana,
            Serum3PlaceOrderInstruction {
                side: Serum3Side::Bid,
                limit_price: (limit_price * 100.0 / 10.0) as u64, // in quote_lot (10) per base lot (100)
                max_base_qty: max_base / 100,                     // in base lot (100)
                max_native_quote_qty_including_fees: (limit_price * (max_base as f64)) as u64,
                self_trade_behavior: Serum3SelfTradeBehavior::AbortTransaction,
                order_type: Serum3OrderType::Limit,
                client_order_id,
                limit: 10,
                account: self.account,
                owner: &self.owner,
                serum_market: self.serum_market,
            },
        )
        .await
        .unwrap();
        self.find_order_id_for_client_order_id(client_order_id)
            .await
    }

    async fn ask(&mut self, limit_price: f64, max_base: u64) -> Option<(u128, u64)> {
        let client_order_id = self.inc_client_order_id();
        send_tx(
            &self.solana,
            Serum3PlaceOrderInstruction {
                side: Serum3Side::Ask,
                limit_price: (limit_price * 100.0 / 10.0) as u64, // in quote_lot (10) per base lot (100)
                max_base_qty: max_base / 100,                     // in base lot (100)
                max_native_quote_qty_including_fees: (limit_price * (max_base as f64)) as u64,
                self_trade_behavior: Serum3SelfTradeBehavior::AbortTransaction,
                order_type: Serum3OrderType::Limit,
                client_order_id,
                limit: 10,
                account: self.account,
                owner: &self.owner,
                serum_market: self.serum_market,
            },
        )
        .await
        .unwrap();
        self.find_order_id_for_client_order_id(client_order_id)
            .await
    }

    async fn cancel(&self, order_id: u128) {
        let side = {
            let open_orders = self.serum.load_open_orders(self.open_orders).await;
            let orders = open_orders.orders;
            let idx = orders.iter().position(|&v| v == order_id).unwrap();
            if open_orders.is_bid_bits & (1u128 << idx) == 0 {
                Serum3Side::Ask
            } else {
                Serum3Side::Bid
            }
        };
        send_tx(
            &self.solana,
            Serum3CancelOrderInstruction {
                side,
                order_id,
                account: self.account,
                owner: &self.owner,
                serum_market: self.serum_market,
            },
        )
        .await
        .unwrap();
    }

    async fn settle(&self) {
        // to avoid multiple settles looking like the same tx
        self.solana.advance_by_slots(1).await;
        send_tx(
            &self.solana,
            Serum3SettleFundsInstruction {
                account: self.account,
                owner: &self.owner,
                serum_market: self.serum_market,
            },
        )
        .await
        .unwrap();
    }

    async fn mango_serum_orders(&self) -> Serum3Orders {
        let account_data = get_mango_account(&self.solana, self.account).await;
        let orders = account_data
            .all_serum3_orders()
            .find(|s| s.open_orders == self.open_orders)
            .unwrap();
        orders.clone()
    }

    async fn open_orders(&self) -> OpenOrdersSlim {
        OpenOrdersSlim::from_oo(&self.serum.load_open_orders(self.open_orders).await)
    }
}

#[tokio::test]
async fn test_serum_basics() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(95_000); // Serum3PlaceOrder needs 92.8k
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..2];

    //
    // SETUP: Create a group and an account
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;
    let base_token = &tokens[0];
    let quote_token = &tokens[1];

    //
    // SETUP: Create serum market
    //
    let serum_market_cookie = context
        .serum
        .list_spot_market(&base_token.mint, &quote_token.mint)
        .await;

    //
    // TEST: Register a serum market
    //
    let serum_market = send_tx(
        solana,
        Serum3RegisterMarketInstruction {
            group,
            admin,
            serum_program: context.serum.program_id,
            serum_market_external: serum_market_cookie.market,
            market_index: 0,
            base_bank: base_token.bank,
            quote_bank: quote_token.bank,
            payer,
        },
    )
    .await
    .unwrap()
    .serum_market;

    //
    // SETUP: Create account
    //
    let deposit_amount = 1000;
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;

    //
    // TEST: Create an open orders account
    //
    let open_orders = send_tx(
        solana,
        Serum3CreateOpenOrdersInstruction {
            account,
            serum_market,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .open_orders;

    let account_data = get_mango_account(solana, account).await;
    assert_eq!(
        account_data
            .active_serum3_orders()
            .map(|v| (v.open_orders, v.market_index))
            .collect::<Vec<_>>(),
        [(open_orders, 0)]
    );

    let mut order_placer = SerumOrderPlacer {
        solana: solana.clone(),
        serum: context.serum.clone(),
        account,
        owner: owner.clone(),
        serum_market,
        open_orders,
        next_client_order_id: 0,
    };

    //
    // TEST: Place an order
    //
    let (order_id, _) = order_placer.bid(1.0, 100).await.unwrap();
    check_prev_instruction_post_health(&solana, account).await;

    let native0 = account_position(solana, account, base_token.bank).await;
    let native1 = account_position(solana, account, quote_token.bank).await;
    assert_eq!(native0, 1000);
    assert_eq!(native1, 900);

    let account_data = get_mango_account(solana, account).await;
    let serum_orders = account_data.serum3_orders_by_raw_index(0);
    assert_eq!(serum_orders.base_borrows_without_fee, 0);
    assert_eq!(serum_orders.quote_borrows_without_fee, 0);

    assert!(order_id != 0);

    //
    // TEST: Cancel the order
    //
    order_placer.cancel(order_id).await;

    //
    // TEST: Settle, moving the freed up funds back
    //
    order_placer.settle().await;

    let native0 = account_position(solana, account, base_token.bank).await;
    let native1 = account_position(solana, account, quote_token.bank).await;
    assert_eq!(native0, 1000);
    assert_eq!(native1, 1000);

    // Process events such that the OutEvent deactivates the closed order on open_orders
    context
        .serum
        .consume_spot_events(&serum_market_cookie, &[open_orders])
        .await;

    // close oo account
    send_tx(
        solana,
        Serum3CloseOpenOrdersInstruction {
            account,
            serum_market,
            owner,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    // deregister serum3 market
    send_tx(
        solana,
        Serum3DeregisterMarketInstruction {
            group,
            admin,
            serum_market_external: serum_market_cookie.market,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    Ok(())
}

#[tokio::test]
async fn test_serum_loan_origination_fees() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(95_000); // Serum3PlaceOrder needs 92.8k
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..3];

    //
    // SETUP: Create a group and an account
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;
    let base_token = &tokens[0];
    let base_bank = base_token.bank;
    let quote_token = &tokens[1];
    let quote_bank = quote_token.bank;

    //
    // SETUP: Create serum market
    //
    let serum_market_cookie = context
        .serum
        .list_spot_market(&base_token.mint, &quote_token.mint)
        .await;

    //
    // SETUP: Register a serum market
    //
    let serum_market = send_tx(
        solana,
        Serum3RegisterMarketInstruction {
            group,
            admin,
            serum_program: context.serum.program_id,
            serum_market_external: serum_market_cookie.market,
            market_index: 0,
            base_bank: base_token.bank,
            quote_bank: quote_token.bank,
            payer,
        },
    )
    .await
    .unwrap()
    .serum_market;

    //
    // SETUP: Create accounts
    //
    let deposit_amount = 180000;
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;
    let account2 = create_funded_account(
        &solana,
        group,
        owner,
        2,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;
    // to have enough funds in the vaults
    create_funded_account(
        &solana,
        group,
        owner,
        3,
        &context.users[1],
        mints,
        10000000,
        0,
    )
    .await;

    let open_orders = send_tx(
        solana,
        Serum3CreateOpenOrdersInstruction {
            account,
            serum_market,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .open_orders;

    let open_orders2 = send_tx(
        solana,
        Serum3CreateOpenOrdersInstruction {
            account: account2,
            serum_market,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .open_orders;

    let mut order_placer = SerumOrderPlacer {
        solana: solana.clone(),
        serum: context.serum.clone(),
        account,
        owner: owner.clone(),
        serum_market,
        open_orders,
        next_client_order_id: 0,
    };
    let mut order_placer2 = SerumOrderPlacer {
        solana: solana.clone(),
        serum: context.serum.clone(),
        account: account2,
        owner: owner.clone(),
        serum_market,
        open_orders: open_orders2,
        next_client_order_id: 100000,
    };

    //
    // TEST: Placing and canceling an order does not take loan origination fees even if borrows are needed
    //
    {
        let (bid_order_id, _) = order_placer.bid(1.0, 200000).await.unwrap();
        let (ask_order_id, _) = order_placer.ask(2.0, 200000).await.unwrap();

        let o = order_placer.mango_serum_orders().await;
        assert_eq!(o.base_borrows_without_fee, 19999); // rounded
        assert_eq!(o.quote_borrows_without_fee, 19999);

        order_placer.cancel(bid_order_id).await;
        order_placer.cancel(ask_order_id).await;

        let o = order_placer.mango_serum_orders().await;
        assert_eq!(o.base_borrows_without_fee, 19999); // unchanged
        assert_eq!(o.quote_borrows_without_fee, 19999);

        // placing new, slightly larger orders increases the borrow_without_fee amount only by a small amount
        let (bid_order_id, _) = order_placer.bid(1.0, 210000).await.unwrap();
        let (ask_order_id, _) = order_placer.ask(2.0, 300000).await.unwrap();

        let o = order_placer.mango_serum_orders().await;
        assert_eq!(o.base_borrows_without_fee, 119998); // rounded
        assert_eq!(o.quote_borrows_without_fee, 29998);

        order_placer.cancel(bid_order_id).await;
        order_placer.cancel(ask_order_id).await;

        // returns all the funds
        order_placer.settle().await;

        let o = order_placer.mango_serum_orders().await;
        assert_eq!(o.base_borrows_without_fee, 0);
        assert_eq!(o.quote_borrows_without_fee, 0);

        assert_eq!(
            account_position(solana, account, quote_bank).await,
            deposit_amount as i64
        );
        assert_eq!(
            account_position(solana, account, base_bank).await,
            deposit_amount as i64
        );

        // consume all the out events from the cancels
        context
            .serum
            .consume_spot_events(&serum_market_cookie, &[open_orders])
            .await;
    }

    let without_serum_taker_fee = |amount: i64| (amount as f64 * (1.0 - 0.0022)).trunc() as i64;
    let serum_maker_rebate = |amount: i64| (amount as f64 * 0.0003).round() as i64;
    let loan_origination_fee = |amount: i64| (amount as f64 * 0.0005).trunc() as i64;

    //
    // TEST: Order execution and settling charges borrow fee
    //
    {
        let deposit_amount = deposit_amount as i64;
        let bid_amount = 200000;
        let ask_amount = 210000;
        let fill_amount = 200000;

        // account2 has an order on the book
        order_placer2.bid(1.0, bid_amount as u64).await.unwrap();

        // account takes
        order_placer.ask(1.0, ask_amount as u64).await.unwrap();
        order_placer.settle().await;

        let o = order_placer.mango_serum_orders().await;
        // parts of the order ended up on the book an may cause loan origination fees later
        assert_eq!(
            o.base_borrows_without_fee,
            (ask_amount - fill_amount) as u64
        );
        assert_eq!(o.quote_borrows_without_fee, 0);

        assert_eq!(
            account_position(solana, account, quote_bank).await,
            deposit_amount + without_serum_taker_fee(fill_amount)
        );
        assert_eq!(
            account_position(solana, account, base_bank).await,
            deposit_amount - ask_amount - loan_origination_fee(fill_amount - deposit_amount)
        );

        // check account2 balances too
        context
            .serum
            .consume_spot_events(&serum_market_cookie, &[open_orders, open_orders2])
            .await;
        order_placer2.settle().await;

        let o = order_placer2.mango_serum_orders().await;
        assert_eq!(o.base_borrows_without_fee, 0);
        assert_eq!(o.quote_borrows_without_fee, 0);

        assert_eq!(
            account_position(solana, account2, base_bank).await,
            deposit_amount + fill_amount
        );
        assert_eq!(
            account_position(solana, account2, quote_bank).await,
            deposit_amount - fill_amount - loan_origination_fee(fill_amount - deposit_amount)
                + serum_maker_rebate(fill_amount)
        );
    }

    Ok(())
}
