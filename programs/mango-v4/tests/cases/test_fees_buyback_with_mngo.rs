use super::*;

#[tokio::test]
async fn test_fees_buyback_with_mngo() -> Result<(), TransportError> {
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

    let deposit_amount = 100_000_000;
    let account_0 = create_funded_account(
        solana,
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
        solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;
    let account_2 = create_funded_account(
        solana,
        group,
        owner,
        2,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;

    //
    // Create a perp market
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
            maker_fee: -0.01,
            taker_fee: 0.02,
            settle_pnl_limit_factor: -1.0,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(solana, &tokens[0]).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::from(1))
    };

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
            max_base_lots: 20,
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
            max_base_lots: 20,
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

    // Change the mngo and quote price to verify that they are taken into account
    set_bank_stub_oracle_price(solana, group, &tokens[0], admin, 2.0).await;
    set_bank_stub_oracle_price(solana, group, &tokens[1], admin, 3.0).await;

    //
    // Test: Account buyback fees accrued with mngo
    //
    send_tx(
        solana,
        GroupEditFeeParameters {
            group,
            admin,
            fees_mngo_token_index: 1 as TokenIndex,
            fees_swap_mango_account: account_2,
            fees_mngo_bonus_factor: 1.2,
        },
    )
    .await
    .unwrap();

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    let before_fees_accrued = mango_account_1.buyback_fees_accrued_current;
    let fees_token_position_before =
        mango_account_1.tokens[0].native(&solana.get_account::<Bank>(tokens[0].bank).await);
    let mngo_token_position_before =
        mango_account_1.tokens[1].native(&solana.get_account::<Bank>(tokens[1].bank).await);
    send_tx(
        solana,
        AccountBuybackFeesWithMngo {
            owner,
            account: account_1,
            mngo_bank: tokens[1].bank,
            fees_bank: tokens[0].bank,
        },
    )
    .await
    .unwrap();
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    let after_fees_accrued = solana
        .get_account::<MangoAccount>(account_1)
        .await
        .buyback_fees_accrued_current;
    let fees_token_position_after =
        mango_account_1.tokens[0].native(&solana.get_account::<Bank>(tokens[0].bank).await);
    let mngo_token_position_after =
        mango_account_1.tokens[1].native(&solana.get_account::<Bank>(tokens[1].bank).await);

    assert_eq!(before_fees_accrued - after_fees_accrued, 19);

    // token[1] swapped at discount for token[0]
    assert!(assert_equal(
        fees_token_position_after - fees_token_position_before,
        19.0 / 2.0,
        0.1
    ));
    assert!(assert_equal(
        mngo_token_position_after - mngo_token_position_before,
        -19.0 / 3.0 / 1.2,
        0.1
    ));

    Ok(())
}
