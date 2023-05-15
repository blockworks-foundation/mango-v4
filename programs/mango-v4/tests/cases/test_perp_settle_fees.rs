use super::*;

#[tokio::test]
async fn test_perp_settle_fees() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..4];

    let initial_token_deposit = 1_000_000;

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

    let _quote_token = &tokens[0];
    let base_token0 = &tokens[1];
    let base_token1 = &tokens[2];
    let _settle_token = &tokens[3];

    let account_0 = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints[0..1],
        initial_token_deposit,
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
        initial_token_deposit,
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
            settle_token_index: 3,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_base_asset_weight: 0.975,
            init_base_asset_weight: 0.95,
            maint_base_liab_weight: 1.025,
            init_base_liab_weight: 1.05,
            base_liquidation_fee: 0.012,
            maker_fee: 0.0002,
            taker_fee: 0.000,
            settle_pnl_limit_factor: 0.2,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &base_token0).await
        },
    )
    .await
    .unwrap();

    //
    // TEST: Create another perp market
    //
    let mango_v4::accounts::PerpCreateMarket {
        perp_market: perp_market_2,
        ..
    } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 1,
            settle_token_index: 3,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_base_asset_weight: 0.975,
            init_base_asset_weight: 0.95,
            maint_base_liab_weight: 1.025,
            init_base_liab_weight: 1.05,
            base_liquidation_fee: 0.012,
            maker_fee: 0.0002,
            taker_fee: 0.000,
            settle_pnl_limit_factor: 0.2,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &base_token1).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::from(1000))
    };

    // Set the initial oracle price
    set_bank_stub_oracle_price(solana, group, &base_token0, admin, 1000.0).await;

    //
    // Place orders and create a position
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

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
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
    assert_eq!(mango_account_0.perps[0].base_position_lots(), 1);
    assert!(assert_equal(
        mango_account_0.perps[0].quote_position_native(),
        -100_020.0,
        0.01
    ));

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].base_position_lots(), -1);
    assert!(assert_equal(
        mango_account_1.perps[0].quote_position_native(),
        100_000.0,
        0.01
    ));

    // Cannot settle position that does not exist
    let result = send_tx(
        solana,
        PerpSettleFeesInstruction {
            account: account_1,
            perp_market: perp_market_2,
            max_settle_amount: u64::MAX,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::PerpPositionDoesNotExist.into(),
        "Cannot settle a position that does not exist".to_string(),
    );

    // max_settle_amount must be greater than zero
    let result = send_tx(
        solana,
        PerpSettleFeesInstruction {
            account: account_1,
            perp_market: perp_market,
            max_settle_amount: 0,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::MaxSettleAmountMustBeGreaterThanZero.into(),
        "max_settle_amount must be greater than zero".to_string(),
    );

    // TODO: Test funding settlement

    {
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        assert_eq!(
            mango_account_0.tokens[0].native(&bank).round(),
            initial_token_deposit,
            "account 0 has expected amount of tokens"
        );
        assert_eq!(
            mango_account_1.tokens[0].native(&bank).round(),
            initial_token_deposit,
            "account 1 has expected amount of tokens"
        );
    }

    // Try and settle with high price
    set_bank_stub_oracle_price(solana, group, base_token0, admin, 1200.0).await;

    // Account must have a loss, should not settle anything and instead return early
    send_tx(
        solana,
        PerpSettleFeesInstruction {
            account: account_0,
            perp_market,
            max_settle_amount: u64::MAX,
        },
    )
    .await
    .unwrap();
    // No change
    {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert!(assert_equal(
            mango_account_0.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from(1200))
                .unwrap(),
            19980.0, // 1*100*(1200-1000) - (20 in fees)
            0.01
        ));
        assert!(assert_equal(
            mango_account_1.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from(1200))
                .unwrap(),
            -20000.0,
            0.01
        ));
    }

    // TODO: Difficult to test health due to fees being so small. Need alternative
    // let result = send_tx(
    //     solana,
    //     PerpSettleFeesInstruction {
    //         group,
    //         account: account_1,
    //         perp_market,
    //         oracle: tokens[0].oracle,
    //         max_settle_amount: I80F48::MAX,
    //     },
    // )
    // .await;

    // assert_mango_error(
    //     &result,
    //     MangoError::HealthMustBePositive.into(),
    //     "Health of losing account must be positive to settle".to_string(),
    // );

    // Change the oracle to a more reasonable price
    set_bank_stub_oracle_price(solana, group, base_token0, admin, 1005.0).await;

    let expected_pnl_0 = I80F48::from(500 - 20);
    let expected_pnl_1 = I80F48::from(-500);

    {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            mango_account_0.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from_num(1005))
                .unwrap()
                .round(),
            expected_pnl_0
        );
        assert_eq!(
            mango_account_1.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from_num(1005))
                .unwrap(),
            expected_pnl_1
        );
    }

    // Check the fees accrued
    let initial_fees = I80F48::from_num(20.0);
    {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            perp_market.fees_accrued.round(),
            initial_fees,
            "Fees from trading have been accrued"
        );
        assert_eq!(
            perp_market.fees_settled.round(),
            0,
            "No fees have been settled yet"
        );
    }

    // Partially execute the settle
    let partial_settle_amount = 10;
    send_tx(
        solana,
        PerpSettleFeesInstruction {
            account: account_1,
            perp_market,
            max_settle_amount: partial_settle_amount,
        },
    )
    .await
    .unwrap();

    {
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;

        assert_eq!(
            mango_account_1.perps[0].base_position_lots(),
            -1,
            "base position unchanged for account 1"
        );

        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000 + partial_settle_amount),
            "quote position increased for losing position by fee settle amount"
        );

        assert_eq!(
            mango_account_1.tokens[1].native(&bank).round(),
            -I80F48::from(partial_settle_amount),
            "account 1 token native position decreased (loss) by max_settle_amount"
        );

        assert_eq!(
            mango_account_1.perp_spot_transfers,
            -(partial_settle_amount as i64),
            "perp_spot_transfers on account 1 updated with loss from settlement"
        );

        assert_eq!(
            perp_market.fees_accrued.round(),
            initial_fees - I80F48::from(partial_settle_amount),
            "Fees accrued have been reduced by partial settle"
        );
        assert_eq!(
            perp_market.fees_settled.round(),
            partial_settle_amount,
            "Fees have been partially settled"
        );
    }

    // Fully execute the settle
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

    {
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;

        assert_eq!(
            mango_account_1.perps[0].base_position_lots(),
            -1,
            "base position unchanged for account 1"
        );

        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000) + initial_fees,
            "quote position increased for losing position by fees settled"
        );

        assert_eq!(
            mango_account_1.tokens[1].native(&bank).round(),
            -initial_fees,
            "account 1 token native position decreased (loss)"
        );

        assert_eq!(
            mango_account_1.perp_spot_transfers, -initial_fees,
            "perp_spot_transfers on account 1 updated with loss from settlement"
        );

        assert_eq!(
            perp_market.fees_accrued.round(),
            0,
            "Fees accrued have been reduced to zero"
        );
        assert_eq!(
            perp_market.fees_settled.round(),
            initial_fees,
            "Fees have been fully settled"
        );
    }

    Ok(())
}
