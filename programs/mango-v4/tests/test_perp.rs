#![cfg(all(feature = "test-bpf"))]

use anchor_lang::prelude::Pubkey;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use mango_v4::state::*;
use program_test::*;
use solana_program_test::*;
use solana_sdk::transport::TransportError;

use mango_setup::*;

mod program_test;

fn compare_fixed_f64(value: I80F48, expected: f64, max_error: f64) -> bool {
    let ok = (value.to_num::<f64>() - expected).abs() < max_error;
    if !ok {
        println!("comparison failed: value: {value}, expected: {expected}");
    }
    ok
}

#[tokio::test]
async fn test_perp() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
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

    let deposit_amount = 1000;
    let account_0 = create_funded_account(
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
    let account_1 = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;

    //
    // TEST: Create a perp market
    //
    let mango_v4::accounts::PerpCreateMarket {
        perp_market,
        asks,
        bids,
        event_queue,
        ..
    } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            oracle: tokens[0].oracle,
            asks: context
                .solana
                .create_account_for_type::<BookSide>(&mango_v4::id())
                .await,
            bids: context
                .solana
                .create_account_for_type::<BookSide>(&mango_v4::id())
                .await,
            event_queue: {
                context
                    .solana
                    .create_account_for_type::<EventQueue>(&mango_v4::id())
                    .await
            },
            payer,
            perp_market_index: 0,
            base_token_index: tokens[0].index,
            base_token_decimals: tokens[0].mint.decimals,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_asset_weight: 0.975,
            init_asset_weight: 0.95,
            maint_liab_weight: 1.025,
            init_liab_weight: 1.05,
            liquidation_fee: 0.012,
            maker_fee: -0.0001,
            taker_fee: 0.0002,
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48!(1))
    };

    //
    // Place and cancel order with order_id
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    let order_id_to_cancel = solana
        .get_account::<MangoAccount>(account_0)
        .await
        .perp_open_orders[0]
        .order_id;
    send_tx(
        solana,
        PerpCancelOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            owner,
            order_id: order_id_to_cancel,
        },
    )
    .await
    .unwrap();

    assert_no_perp_orders(solana, account_0).await;

    //
    // Place and cancel order with client_order_id
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 1,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    send_tx(
        solana,
        PerpCancelOrderByClientOrderIdInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            owner,
            client_order_id: 1,
        },
    )
    .await
    .unwrap();

    assert_no_perp_orders(solana, account_0).await;

    //
    // Place and cancel all orders
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 2,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 3,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 4,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    send_tx(
        solana,
        PerpCancelAllOrdersInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            owner,
        },
    )
    .await
    .unwrap();

    assert_no_perp_orders(solana, account_0).await;

    //
    // Place a bid, corresponding ask, and consume event
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 5,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_1,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 6,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_1).await;

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            group,
            perp_market,
            event_queue,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].base_position_lots(), 1);
    assert!(compare_fixed_f64(
        mango_account_0.perps[0].quote_position_native(),
        -99.99,
        0.001
    ));

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].base_position_lots(), -1);
    assert!(compare_fixed_f64(
        mango_account_1.perps[0].quote_position_native(),
        99.98,
        0.001
    ));

    //
    // TEST: closing perp positions
    //

    // Can't close yet, active positions
    assert!(send_tx(
        solana,
        PerpClosePositionInstruction {
            account: account_0,
            perp_market,
            owner,
        },
    )
    .await
    .is_err());
    solana.advance_by_slots(1).await;

    // Trade again to bring base_position_lots to 0
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 7,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_1,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 8,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_1).await;

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            group,
            perp_market,
            event_queue,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].base_position_lots(), 0);
    assert!(compare_fixed_f64(
        mango_account_0.perps[0].quote_position_native(),
        0.02,
        0.001
    ));

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].base_position_lots(), 0);
    assert!(compare_fixed_f64(
        mango_account_1.perps[0].quote_position_native(),
        -0.04,
        0.001
    ));

    // settle pnl and fees to bring quote_position_native fully to 0
    send_tx(
        solana,
        PerpSettlePnlInstruction {
            group,
            account_a: account_0,
            account_b: account_1,
            perp_market,
            oracle: tokens[0].oracle,
            quote_bank: tokens[0].bank,
            max_settle_amount: I80F48::MAX,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        PerpSettleFeesInstruction {
            group,
            account: account_1,
            perp_market,
            oracle: tokens[0].oracle,
            quote_bank: tokens[0].bank,
            max_settle_amount: I80F48::MAX,
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].quote_position_native(), 0);

    // Now closing works!
    send_tx(
        solana,
        PerpClosePositionInstruction {
            account: account_0,
            perp_market,
            owner,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        PerpClosePositionInstruction {
            account: account_1,
            perp_market,
            owner,
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].market_index, PerpMarketIndex::MAX);
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].market_index, PerpMarketIndex::MAX);

    //
    // TEST: market closing (testing only)
    //
    send_tx(
        solana,
        PerpCloseMarketInstruction {
            group,
            admin,
            perp_market,
            asks,
            bids,
            event_queue,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    Ok(())
}

async fn assert_no_perp_orders(solana: &SolanaCookie, account_0: Pubkey) {
    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;

    for oo in mango_account_0.perp_open_orders.iter() {
        assert!(oo.order_id == 0);
        assert!(oo.order_side == Side::Bid);
        assert!(oo.client_order_id == 0);
        assert!(oo.order_market == FREE_ORDER_SLOT);
    }
}
