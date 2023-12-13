use super::*;

#[tokio::test]
async fn test_perp_fixed() -> Result<(), TransportError> {
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
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
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
    let settler =
        create_funded_account(&solana, group, owner, 251, &context.users[1], &[], 0, 0).await;
    let settler_owner = owner.clone();

    //
    // TEST: Create a perp market
    //
    let mango_v4::accounts::PerpCreateMarket {
        perp_market, bids, ..
    } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 0,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_base_asset_weight: 0.975,
            init_base_asset_weight: 0.95,
            maint_base_liab_weight: 1.025,
            init_base_liab_weight: 1.05,
            base_liquidation_fee: 0.012,
            maker_fee: -0.0001,
            taker_fee: 0.0002,
            settle_pnl_limit_factor: -1.0,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[0]).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::ONE)
    };

    //
    // Place and cancel order with order_id
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    let bids_data = solana.get_account_boxed::<BookSide>(bids).await;
    assert_eq!(bids_data.roots[0].leaf_count, 1);
    let order_id_to_cancel = solana
        .get_account::<MangoAccount>(account_0)
        .await
        .perp_open_orders[0]
        .id;
    send_tx(
        solana,
        PerpCancelOrderInstruction {
            account: account_0,
            perp_market,
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
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            client_order_id: 1,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    send_tx(
        solana,
        PerpCancelOrderByClientOrderIdInstruction {
            account: account_0,
            perp_market,
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
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            client_order_id: 2,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;
    send_tx(
        solana,
        PerpPlaceOrderPeggedInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_offset: -1,
            peg_limit: -1,
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
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            client_order_id: 4,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    send_tx(
        solana,
        PerpCancelAllOrdersInstruction {
            account: account_0,
            perp_market,
            owner,
            limit: 10,
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
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            client_order_id: 5,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            client_order_id: 6,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_1).await;

    // Trying to cancel-all after the order was already taken: has no effect but succeeds
    send_tx(
        solana,
        PerpCancelAllOrdersInstruction {
            account: account_0,
            perp_market,
            owner,
            limit: 10,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].base_position_lots(), 1);
    assert!(assert_equal(
        mango_account_0.perps[0].quote_position_native(),
        -99.99,
        0.001
    ));

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].base_position_lots(), -1);
    assert!(assert_equal(
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
        PerpDeactivatePositionInstruction {
            account: account_0,
            perp_market,
            owner,
        },
    )
    .await
    .is_err());

    // Trade again to bring base_position_lots to 0
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            client_order_id: 7,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            client_order_id: 8,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_1).await;

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].base_position_lots(), 0);
    assert!(assert_equal(
        mango_account_0.perps[0].quote_position_native(),
        0.02,
        0.001
    ));

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].base_position_lots(), 0);
    assert!(assert_equal(
        mango_account_1.perps[0].quote_position_native(),
        -0.04,
        0.001
    ));

    // settle pnl and fees to bring quote_position_native fully to 0
    send_tx(
        solana,
        PerpSettlePnlInstruction {
            settler,
            settler_owner,
            account_a: account_0,
            account_b: account_1,
            perp_market,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        PerpSettleFeesInstruction {
            account: account_1,
            perp_market,
            max_settle_amount: u64::MAX,
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].quote_position_native(), 0);

    // Now closing works!
    send_tx(
        solana,
        PerpDeactivatePositionInstruction {
            account: account_0,
            perp_market,
            owner,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        PerpDeactivatePositionInstruction {
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
            admin,
            perp_market,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    Ok(())
}

#[tokio::test]
async fn test_perp_oracle_peg() -> Result<(), TransportError> {
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
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let deposit_amount = 100000;
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
    // SETUP: Create a perp market
    //
    let mango_v4::accounts::PerpCreateMarket {
        perp_market, bids, ..
    } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 0,
            quote_lot_size: 10,
            base_lot_size: 10000,
            maint_base_asset_weight: 0.975,
            init_base_asset_weight: 0.95,
            maint_base_liab_weight: 1.025,
            init_base_liab_weight: 1.05,
            base_liquidation_fee: 0.012,
            maker_fee: -0.0001,
            taker_fee: 0.0002,
            settle_pnl_limit_factor: 0.2,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[0]).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::ONE)
    };
    assert_eq!(price_lots, 1000);

    //
    // TEST: Place and cancel order with order_id
    //
    send_tx(
        solana,
        PerpPlaceOrderPeggedInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_offset: -1,
            peg_limit: -1,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_0).await;

    let bids_data = solana.get_account_boxed::<BookSide>(bids).await;
    assert_eq!(bids_data.roots[1].leaf_count, 1);
    let perp_order = solana
        .get_account::<MangoAccount>(account_0)
        .await
        .perp_open_orders[0];
    assert_eq!(
        perp_order.side_and_tree(),
        SideAndOrderTree::BidOraclePegged
    );
    send_tx(
        solana,
        PerpCancelOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            order_id: perp_order.id,
        },
    )
    .await
    .unwrap();

    assert_no_perp_orders(solana, account_0).await;

    //
    // TEST: Place a pegged bid, take it with a direct and pegged ask, and consume events
    //
    send_tx(
        solana,
        PerpPlaceOrderPeggedInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_offset: 0,
            peg_limit: -1,
            max_base_lots: 2,
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
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            client_order_id: 6,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_1).await;

    send_tx(
        solana,
        PerpPlaceOrderPeggedInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_offset: 0,
            peg_limit: -1,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 7,
        },
    )
    .await
    .unwrap();
    check_prev_instruction_post_health(&solana, account_1).await;

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].base_position_lots(), 2);
    assert!(assert_equal(
        mango_account_0.perps[0].quote_position_native(),
        -19998.0,
        0.001
    ));

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].base_position_lots(), -2);
    assert!(assert_equal(
        mango_account_1.perps[0].quote_position_native(),
        19996.0,
        0.001
    ));

    //
    // TEST: Place a pegged order and check how it behaves with oracle changes
    //
    send_tx(
        solana,
        PerpPlaceOrderPeggedInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_offset: -1,
            peg_limit: -1,
            max_base_lots: 2,
            max_quote_lots: i64::MAX,
            client_order_id: 5,
        },
    )
    .await
    .unwrap();

    // TEST: an ask at current oracle price does not match
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            client_order_id: 60,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        PerpCancelOrderByClientOrderIdInstruction {
            account: account_1,
            perp_market,
            owner,
            client_order_id: 60,
        },
    )
    .await
    .unwrap();

    // TEST: Change the oracle, now the ask matches
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[0], admin, 1.002).await;
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 2,
            client_order_id: 61,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();
    assert_no_perp_orders(solana, account_0).await;

    // restore the oracle to default
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[0], admin, 1.0).await;

    //
    // TEST: order is cancelled when the price exceeds the peg limit
    //
    send_tx(
        solana,
        PerpPlaceOrderPeggedInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_offset: -1,
            peg_limit: price_lots + 2,
            max_base_lots: 2,
            max_quote_lots: i64::MAX,
            client_order_id: 5,
        },
    )
    .await
    .unwrap();

    // order is still matchable when exactly at the peg limit
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[0], admin, 1.003).await;
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots: price_lots + 2,
            max_base_lots: 1,
            client_order_id: 62,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    assert!(send_tx(
        solana,
        PerpCancelOrderByClientOrderIdInstruction {
            account: account_1,
            perp_market,
            owner,
            client_order_id: 62,
        },
    )
    .await
    .is_err());

    // but once the adjusted price is > the peg limit, it's gone
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[0], admin, 1.004).await;
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots: price_lots + 3,
            max_base_lots: 1,
            client_order_id: 63,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        PerpCancelOrderByClientOrderIdInstruction {
            account: account_1,
            perp_market,
            owner,
            client_order_id: 63,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();
    assert_no_perp_orders(solana, account_0).await;

    Ok(())
}

#[tokio::test]
async fn test_perp_realize_partially() -> Result<(), TransportError> {
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
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
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
    let mango_v4::accounts::PerpCreateMarket { perp_market, .. } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 0,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_base_asset_weight: 0.975,
            init_base_asset_weight: 0.95,
            maint_base_liab_weight: 1.025,
            init_base_liab_weight: 1.05,
            base_liquidation_fee: 0.012,
            maker_fee: 0.0000,
            taker_fee: 0.0000,
            settle_pnl_limit_factor: -1.0,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[1]).await
        },
    )
    .await
    .unwrap();

    let perp_market_data = solana.get_account::<PerpMarket>(perp_market).await;
    let price_lots = perp_market_data.native_price_to_lot(I80F48::from(1000));
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1000.0).await;

    //
    // SETUP: Place a bid, corresponding ask, and consume event
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 2,
            client_order_id: 5,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 2,
            client_order_id: 6,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let perp_0 = mango_account_0.perps[0];
    assert_eq!(perp_0.base_position_lots(), 2);

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    let perp_1 = mango_account_1.perps[0];
    assert_eq!(perp_1.base_position_lots(), -2);

    //
    // SETUP: Sell one lot again at increased price
    //
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1500.0).await;
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots: perp_market_data.native_price_to_lot(I80F48::from_num(1500)),
            max_base_lots: 1,
            client_order_id: 5,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots: perp_market_data.native_price_to_lot(I80F48::from_num(1500)),
            max_base_lots: 1,
            client_order_id: 6,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let perp_0 = mango_account_0.perps[0];
    assert_eq!(perp_0.base_position_lots(), 1);
    assert!(assert_equal(
        perp_0.quote_position_native(),
        -200_000.0 + 150_000.0,
        0.001
    ));
    assert!(assert_equal(
        perp_0.realized_pnl_for_position_native,
        50_000.0,
        0.001
    ));

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    let perp_1 = mango_account_1.perps[0];
    assert_eq!(perp_1.base_position_lots(), -1);
    assert!(assert_equal(
        perp_1.quote_position_native(),
        200_000.0 - 150_000.0,
        0.001
    ));
    assert!(assert_equal(
        perp_1.realized_pnl_for_position_native,
        -50_000.0,
        0.001
    ));

    Ok(())
}

#[tokio::test]
async fn test_perp_reducing_when_liquidatable() -> Result<(), TransportError> {
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
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let deposit_amount = 100000;
    let account_0 = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints[0..1],
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
        &mints[0..1],
        deposit_amount,
        0,
    )
    .await;

    //
    // TEST: Create a perp market
    //
    let mango_v4::accounts::PerpCreateMarket { perp_market, .. } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 0,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_base_asset_weight: 0.975,
            init_base_asset_weight: 0.95,
            maint_base_liab_weight: 1.025,
            init_base_liab_weight: 1.05,
            base_liquidation_fee: 0.012,
            maker_fee: 0.0000,
            taker_fee: 0.0000,
            settle_pnl_limit_factor: -1.0,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[1]).await
        },
    )
    .await
    .unwrap();

    let perp_market_data = solana.get_account::<PerpMarket>(perp_market).await;
    let price_lots = perp_market_data.native_price_to_lot(I80F48::from(1000));
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1000.0).await;

    //
    // SETUP: Place a bid, corresponding ask, and consume event
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 2,
            client_order_id: 5,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 2,
            client_order_id: 6,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let perp_0 = mango_account_0.perps[0];
    assert_eq!(perp_0.base_position_lots(), 2);

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    let perp_1 = mango_account_1.perps[0];
    assert_eq!(perp_1.base_position_lots(), -2);

    //
    // SETUP: Change the price to make the SHORT account liquidatable
    //
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 4000.0).await;
    assert!(account_init_health(solana, account_1).await < 0.0);

    //
    // TEST: Can place an order that reduces the position anyway
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots: perp_market_data.native_price_to_lot(I80F48::from_num(4000)),
            max_base_lots: 1,
            client_order_id: 5,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();

    //
    // TEST: Can NOT place an order that goes too far
    //
    let err = send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots: perp_market_data.native_price_to_lot(I80F48::from_num(4000)),
            max_base_lots: 5,
            client_order_id: 5,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await;
    assert!(err.is_err());

    Ok(())
}

#[tokio::test]
async fn test_perp_compute() -> Result<(), TransportError> {
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
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let deposit_amount = 100_000;
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
    let mango_v4::accounts::PerpCreateMarket { perp_market, .. } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 0,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_base_asset_weight: 0.975,
            init_base_asset_weight: 0.95,
            maint_base_liab_weight: 1.025,
            init_base_liab_weight: 1.05,
            base_liquidation_fee: 0.012,
            maker_fee: 0.0000,
            taker_fee: 0.0000,
            settle_pnl_limit_factor: -1.0,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[1]).await
        },
    )
    .await
    .unwrap();

    let perp_market_data = solana.get_account::<PerpMarket>(perp_market).await;
    let price_lots = perp_market_data.native_price_to_lot(I80F48::from(1000));
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1000.0).await;

    //
    // TEST: check compute per order match
    //

    for limit in 1..6 {
        for bid in price_lots..price_lots + 6 {
            send_tx(
                solana,
                PerpPlaceOrderInstruction {
                    account: account_0,
                    perp_market,
                    owner,
                    side: Side::Bid,
                    price_lots: bid,
                    max_base_lots: 1,
                    client_order_id: 5,
                    ..PerpPlaceOrderInstruction::default()
                },
            )
            .await
            .unwrap();
        }
        let result = send_tx_get_metadata(
            solana,
            PerpPlaceOrderInstruction {
                account: account_1,
                perp_market,
                owner,
                side: Side::Ask,
                price_lots,
                max_base_lots: 10,
                client_order_id: 6,
                limit,
                ..PerpPlaceOrderInstruction::default()
            },
        )
        .await
        .unwrap();
        println!(
            "CU for perp_place_order matching {limit} orders in sequence: {}",
            result.metadata.unwrap().compute_units_consumed
        );

        send_tx(
            solana,
            PerpCancelAllOrdersInstruction {
                account: account_0,
                perp_market,
                owner,
                limit: 10,
            },
        )
        .await
        .unwrap();
        send_tx(
            solana,
            PerpCancelAllOrdersInstruction {
                account: account_1,
                perp_market,
                owner,
                limit: 10,
            },
        )
        .await
        .unwrap();

        send_tx(
            solana,
            PerpConsumeEventsInstruction {
                perp_market,
                mango_accounts: vec![account_0, account_1],
            },
        )
        .await
        .unwrap();
        send_tx(
            solana,
            PerpConsumeEventsInstruction {
                perp_market,
                mango_accounts: vec![account_0, account_1],
            },
        )
        .await
        .unwrap();
        send_tx(
            solana,
            PerpConsumeEventsInstruction {
                perp_market,
                mango_accounts: vec![account_0, account_1],
            },
        )
        .await
        .unwrap();
    }

    //
    // TEST: check compute per order cancel
    //

    for count in 1..6 {
        for bid in price_lots..price_lots + count {
            send_tx(
                solana,
                PerpPlaceOrderInstruction {
                    account: account_0,
                    perp_market,
                    owner,
                    side: Side::Bid,
                    price_lots: bid,
                    max_base_lots: 1,
                    client_order_id: 5,
                    ..PerpPlaceOrderInstruction::default()
                },
            )
            .await
            .unwrap();
        }
        let result = send_tx_get_metadata(
            solana,
            PerpCancelAllOrdersInstruction {
                account: account_0,
                perp_market,
                owner,
                limit: 10,
            },
        )
        .await
        .unwrap();
        println!(
            "CU for perp_cancel_all_orders matching {count} orders: {}",
            result.metadata.unwrap().compute_units_consumed
        );

        send_tx(
            solana,
            PerpConsumeEventsInstruction {
                perp_market,
                mango_accounts: vec![account_0],
            },
        )
        .await
        .unwrap();
    }

    Ok(())
}

#[tokio::test]
async fn test_perp_cancel_with_in_flight_events() -> Result<(), TransportError> {
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
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
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
    // SETUP: Create a perp market
    //
    let mango_v4::accounts::PerpCreateMarket { perp_market, .. } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 0,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_base_asset_weight: 0.975,
            init_base_asset_weight: 0.95,
            maint_base_liab_weight: 1.025,
            init_base_liab_weight: 1.05,
            base_liquidation_fee: 0.012,
            maker_fee: 0.0000,
            taker_fee: 0.0000,
            settle_pnl_limit_factor: -1.0,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[1]).await
        },
    )
    .await
    .unwrap();

    let perp_market_data = solana.get_account::<PerpMarket>(perp_market).await;
    let price_lots = perp_market_data.native_price_to_lot(I80F48::from(1));

    //
    // SETUP: Place a bid, a matching ask, generating a closing fill event
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 2,
            client_order_id: 5,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 2,
            client_order_id: 6,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();

    //
    // TEST: it's possible to cancel, freeing up the user's oo slot
    //

    send_tx(
        solana,
        PerpCancelAllOrdersInstruction {
            account: account_0,
            perp_market,
            owner,
            limit: 10,
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let perp_0 = mango_account_0.perps[0];
    assert_eq!(perp_0.bids_base_lots, 2);
    assert!(!mango_account_0.perp_open_orders[0].is_active());

    //
    // TEST: consuming the event updates the perp account state
    //

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let perp_0 = mango_account_0.perps[0];
    assert_eq!(perp_0.bids_base_lots, 0);

    Ok(())
}

async fn assert_no_perp_orders(solana: &SolanaCookie, account_0: Pubkey) {
    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;

    for oo in mango_account_0.perp_open_orders.iter() {
        assert!(oo.id == 0);
        assert!(oo.side_and_tree() == SideAndOrderTree::BidFixed);
        assert!(oo.client_id == 0);
        assert!(oo.market == FREE_ORDER_SLOT);
    }
}
