use super::*;

#[tokio::test]
async fn test_perp_settle_pnl_basic() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(90_000); // the divisions in perp_max_settle are costly!
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..=3];

    let initial_token_deposit = 10_000;

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

    let settler =
        create_funded_account(&solana, group, owner, 251, &context.users[1], &[], 0, 0).await;
    let settler_owner = owner.clone();

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
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[1]).await
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
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[2]).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::from(1000))
    };

    // Set the initial oracle price
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1000.0).await;

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

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;

        assert_eq!(mango_account_0.perps[0].base_position_lots(), 1);
        assert_eq!(mango_account_1.perps[0].base_position_lots(), -1);
        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            -100_020
        );
        assert_eq!(mango_account_1.perps[0].quote_position_native(), 100_000);
    }

    // Cannot settle with yourself
    let result = send_tx(
        solana,
        PerpSettlePnlInstruction {
            settler,
            settler_owner,
            account_a: account_0,
            account_b: account_0,
            perp_market,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::CannotSettleWithSelf.into(),
        "Cannot settle with yourself".to_string(),
    );

    // Cannot settle position that does not exist
    let result = send_tx(
        solana,
        PerpSettlePnlInstruction {
            settler,
            settler_owner,
            account_a: account_0,
            account_b: account_1,
            perp_market: perp_market_2,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::PerpPositionDoesNotExist.into(),
        "Cannot settle a position that does not exist".to_string(),
    );

    // TODO: Test funding settlement

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
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
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1200.0).await;

    // Account a must be the profitable one
    let result = send_tx(
        solana,
        PerpSettlePnlInstruction {
            settler,
            settler_owner,
            account_a: account_1,
            account_b: account_0,
            perp_market,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::ProfitabilityMismatch.into(),
        "Account a must be the profitable one".to_string(),
    );

    // Change the oracle to a more reasonable price
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1005.0).await;

    let expected_pnl_0 = I80F48::from(480); // Less due to fees
    let expected_pnl_1 = I80F48::from(-500);

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            mango_account_0.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from(1005))
                .unwrap()
                .round(),
            expected_pnl_0
        );
        assert_eq!(
            mango_account_1.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from(1005))
                .unwrap(),
            expected_pnl_1
        );
    }

    // Change the oracle to a very high price, such that the pnl exceeds the account funding
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1500.0).await;

    let expected_pnl_0 = I80F48::from(50000 - 20);
    let expected_pnl_1 = I80F48::from(-50000);

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            mango_account_0.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from(1500))
                .unwrap()
                .round(),
            expected_pnl_0
        );
        assert_eq!(
            mango_account_1.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from(1500))
                .unwrap(),
            expected_pnl_1
        );
    }

    //
    // SETUP: Add some non-settle token, so the account's health has more contributions
    //
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 1001,
            reduce_only: false,
            account: account_1,
            owner,
            token_account: context.users[1].token_accounts[2],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Settle as much PNL as account_1's health allows:
    // The account perp_settle_health is 0.8 * 10000.0 + 0.8 * 1001.0,
    // meaning we can bring it to zero by settling 10000 * 0.8 / 0.8 + 1001 * 0.8 / 1.2 = 10667.333
    // because then we'd be left with -667.333 * 1.2 + 0.8 * 1000 = 0
    let expected_total_settle = I80F48::from(10667);
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

    {
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;

        assert_eq!(
            mango_account_0.perps[0].base_position_lots(),
            1,
            "base position unchanged for account 0"
        );
        assert_eq!(
            mango_account_1.perps[0].base_position_lots(),
            -1,
            "base position unchanged for account 1"
        );

        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            I80F48::from(-100_020) - expected_total_settle,
            "quote position reduced for profitable position"
        );
        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000) + expected_total_settle,
            "quote position increased for losing position by opposite of first account"
        );

        assert_eq!(
            mango_account_0.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit) + expected_total_settle,
            "account 0 token native position increased (profit)"
        );
        assert_eq!(
            mango_account_1.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit) - expected_total_settle,
            "account 1 token native position decreased (loss)"
        );

        assert_eq!(
            mango_account_0.perp_spot_transfers, expected_total_settle,
            "net_settled on account 0 updated with profit from settlement"
        );
        assert_eq!(
            mango_account_1.perp_spot_transfers, -expected_total_settle,
            "net_settled on account 1 updated with loss from settlement"
        );
        assert_eq!(
            mango_account_0.perps[0].perp_spot_transfers, expected_total_settle,
            "net_settled on account 0 updated with profit from settlement"
        );
        assert_eq!(
            mango_account_1.perps[0].perp_spot_transfers, -expected_total_settle,
            "net_settled on account 1 updated with loss from settlement"
        );
    }

    // Change the oracle to a reasonable price in other direction
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 995.0).await;

    let expected_pnl_0 = I80F48::from(1 * 100 * 995 - 1 * 100 * 1000 - 20) - expected_total_settle;
    let expected_pnl_1 = I80F48::from(-1 * 100 * 995 + 1 * 100 * 1000) + expected_total_settle;

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            mango_account_0.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from(995))
                .unwrap()
                .round(),
            expected_pnl_0
        );
        assert_eq!(
            mango_account_1.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from(995))
                .unwrap()
                .round(),
            expected_pnl_1
        );
    }

    // Fully execute the settle
    let expected_total_settle = expected_total_settle - expected_pnl_1;
    send_tx(
        solana,
        PerpSettlePnlInstruction {
            settler,
            settler_owner,
            account_a: account_1,
            account_b: account_0,
            perp_market,
        },
    )
    .await
    .unwrap();

    {
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;

        assert_eq!(
            mango_account_0.perps[0].base_position_lots(),
            1,
            "base position unchanged for account 0"
        );
        assert_eq!(
            mango_account_1.perps[0].base_position_lots(),
            -1,
            "base position unchanged for account 1"
        );

        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            I80F48::from(-100_020) - expected_total_settle,
            "quote position increased for losing position"
        );
        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000) + expected_total_settle,
            "quote position reduced for losing position by opposite of first account"
        );

        // 480 was previous settlement
        assert_eq!(
            mango_account_0.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit) + expected_total_settle,
            "account 0 token native position decreased (loss)"
        );
        assert_eq!(
            mango_account_1.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit) - expected_total_settle,
            "account 1 token native position increased (profit)"
        );

        assert_eq!(
            mango_account_0.perp_spot_transfers, expected_total_settle,
            "net_settled on account 0 updated with loss from settlement"
        );
        assert_eq!(
            mango_account_1.perp_spot_transfers, -expected_total_settle,
            "net_settled on account 1 updated with profit from settlement"
        );
    }

    // no more settleable pnl left
    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            mango_account_0.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from(995))
                .unwrap()
                .round(),
            -20 // fees
        );
        assert_eq!(
            mango_account_1.perps[0]
                .unsettled_pnl(&perp_market, I80F48::from(995))
                .unwrap()
                .round(),
            0
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_perp_settle_pnl_fees() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(90_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..=2];

    let initial_token_deposit = 10_000;

    //
    // SETUP: Create a group and accounts
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        zero_token_is_quote: true,
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let settle_bank = tokens[0].bank;

    // ensure vaults are not empty
    create_funded_account(
        &solana,
        group,
        owner,
        250,
        &context.users[1],
        mints,
        100_000,
        0,
    )
    .await;

    let settler =
        create_funded_account(&solana, group, owner, 251, &context.users[1], &[], 0, 0).await;
    let settler_owner = owner.clone();

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
    // SETUP: Create a perp market
    //
    let flat_fee = 1000;
    let fee_low_health = 0.10;
    let mango_v4::accounts::PerpCreateMarket { perp_market, .. } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 0,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_base_asset_weight: 1.0,
            init_base_asset_weight: 1.0,
            maint_base_liab_weight: 1.0,
            init_base_liab_weight: 1.0,
            base_liquidation_fee: 0.0,
            maker_fee: 0.0,
            taker_fee: 0.0,
            settle_fee_flat: flat_fee as f32,
            settle_fee_amount_threshold: 4000.0,
            settle_fee_fraction_low_health: fee_low_health,
            settle_pnl_limit_factor: 0.2,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[1]).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::from(1000))
    };

    // Set the initial oracle price
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1000.0).await;

    //
    // SETUP: Create a perp base position
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

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;

        assert_eq!(mango_account_0.perps[0].base_position_lots(), 1);
        assert_eq!(mango_account_1.perps[0].base_position_lots(), -1);
        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            -100_000
        );
        assert_eq!(mango_account_1.perps[0].quote_position_native(), 100_000);
    }

    //
    // TEST: Settle (health is high)
    //
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1050.0).await;

    let expected_pnl = 5000;

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

    let mut total_settled_pnl = expected_pnl;
    let mut total_fees_paid = flat_fee;
    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            I80F48::from(-100_000 - total_settled_pnl)
        );
        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000 + total_settled_pnl),
        );
        assert_eq!(
            account_position(solana, account_0, settle_bank).await,
            initial_token_deposit as i64 + total_settled_pnl - total_fees_paid
        );
        assert_eq!(
            account_position(solana, account_1, settle_bank).await,
            initial_token_deposit as i64 - total_settled_pnl
        );
        assert_eq!(
            account_position(solana, settler, settle_bank).await,
            total_fees_paid
        );
    }

    //
    // Bring account_0 health low, specifically to
    // init_health = 14000 - 1.4 * 1 * 10700 = -980
    // maint_health = 14000 - 1.2 * 1 * 10700 = 1160
    //
    send_tx(
        solana,
        TokenWithdrawInstruction {
            account: account_0,
            owner,
            token_account: context.users[1].token_accounts[2],
            amount: 1,
            allow_borrow: true,
            bank_index: 0,
        },
    )
    .await
    .unwrap();
    set_bank_stub_oracle_price(solana, group, &tokens[2], admin, 10700.0).await;

    //
    // TEST: Settle (health is low), percent fee
    //
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1070.0).await;

    let expected_pnl = 2000;

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

    total_settled_pnl += expected_pnl;
    total_fees_paid +=
        (expected_pnl as f64 * fee_low_health as f64 * 980.0 / (1160.0 + 980.0)) as i64 + 1;
    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            I80F48::from(-100_000 - total_settled_pnl)
        );
        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000 + total_settled_pnl),
        );
        assert_eq!(
            account_position(solana, account_0, settle_bank).await,
            initial_token_deposit as i64 + total_settled_pnl - total_fees_paid
        );
        assert_eq!(
            account_position(solana, account_1, settle_bank).await,
            initial_token_deposit as i64 - total_settled_pnl
        );
        assert_eq!(
            account_position(solana, settler, settle_bank).await,
            total_fees_paid
        );
    }

    //
    // TEST: Settle (health is low), no fee because pnl too small
    //
    set_perp_stub_oracle_price(solana, group, perp_market, &tokens[1], admin, 1071.0).await;

    let expected_pnl = 100;

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

    total_settled_pnl += expected_pnl;
    total_fees_paid += 0;
    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            I80F48::from(-100_000 - total_settled_pnl)
        );
        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000 + total_settled_pnl),
        );
        assert_eq!(
            account_position(solana, account_0, settle_bank).await,
            initial_token_deposit as i64 + total_settled_pnl - total_fees_paid
        );
        assert_eq!(
            account_position(solana, account_1, settle_bank).await,
            initial_token_deposit as i64 - total_settled_pnl
        );
        assert_eq!(
            account_position(solana, settler, settle_bank).await,
            total_fees_paid
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_perp_pnl_settle_limit() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..=2];

    let initial_token_deposit = 1000_000;

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

    let settler =
        create_funded_account(&solana, group, owner, 251, &context.users[1], &[], 0, 0).await;
    let settler_owner = owner.clone();

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
        initial_token_deposit * 100, // Fund 100x, so that this is not the bound for what account_0 can settle
        0,
    )
    .await;

    //
    // TEST: Create a perp market
    //
    let settle_pnl_limit_factor = 0.8;
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
            maker_fee: 0.0,
            taker_fee: 0.0,
            settle_pnl_limit_factor,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[1]).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::from(1000))
    };

    // Set the initial oracle price
    set_perp_stub_oracle_price(&solana, group, perp_market, &tokens[1], admin, 1000.0).await;

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

    // Manipulate the price (without adjusting stable price)
    let price_factor = 3;
    send_tx(
        solana,
        StubOracleSetInstruction {
            oracle: tokens[1].oracle,
            group,
            admin,
            mint: mints[1].pubkey,
            price: price_factor as f64 * 1000.0,
        },
    )
    .await
    .unwrap();

    //
    // Test 1: settle max possible, limited by unrealized pnl settle limit
    //
    // a has lots of positive unrealized pnl, b has negative unrealized pnl.
    // Since b has very large deposits, b's health will not interfere.
    // The pnl settle limit is relative to the stable price
    let market = solana.get_account::<PerpMarket>(perp_market).await;
    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    let account_1_settle_limits = mango_account_1.perps[0].available_settle_limit(&market);
    assert_eq!(account_1_settle_limits, (-80000, 80000));
    let account_1_settle_limit = I80F48::from(account_1_settle_limits.0.abs());
    assert_eq!(
        account_1_settle_limit,
        (market.settle_pnl_limit_factor()
            * market.stable_price()
            * mango_account_0.perps[0].base_position_native(&market))
        .round()
    );
    let mango_account_1_expected_qpn_after_settle =
        mango_account_1.perps[0].quote_position_native() + account_1_settle_limit.round();
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
    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(
        mango_account_1.perps[0].quote_position_native().round(),
        mango_account_1_expected_qpn_after_settle.round()
    );
    // neither account has any settle limit left
    assert_eq!(
        mango_account_0.perps[0].available_settle_limit(&market).1,
        0
    );
    assert_eq!(
        mango_account_1.perps[0].available_settle_limit(&market).0,
        0
    );

    //
    // Test 2: Once the settle limit is exhausted, we can't settle more
    //
    // we are in the same window, and we settled max. possible in previous attempt
    let result = send_tx(
        solana,
        PerpSettlePnlInstruction {
            settler,
            settler_owner,
            account_a: account_0,
            account_b: account_1,
            perp_market,
        },
    )
    .await;
    assert_mango_error(
        &result,
        MangoError::ProfitabilityMismatch.into(),
        "Account A has no settleable positive pnl left".to_string(),
    );

    //
    // Test 3: realizing the pnl does not allow further settling
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots: 3 * price_lots,
            max_base_lots: 1,
            ..PerpPlaceOrderInstruction::default()
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots: 3 * price_lots,
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
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    // neither account has any settle limit left (check for 1 because of the ceil()ing)
    assert_eq!(
        mango_account_0.perps[0].available_settle_limit(&market).1,
        1
    );
    assert_eq!(
        mango_account_1.perps[0].available_settle_limit(&market).0,
        -1
    );
    // check that realized pnl settle limit was set up correctly
    assert_eq!(
        mango_account_0.perps[0].recurring_settle_pnl_allowance,
        (0.8 * 1.0 * 100.0 * 1000.0) as i64 + 1
    ); // +1 just for rounding

    // settle 1
    let account_1_quote_before = mango_account_1.perps[0].quote_position_native();
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

    // indeed settled 1
    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(
        mango_account_1.perps[0].quote_position_native() - account_1_quote_before,
        I80F48::from(1)
    );

    //
    // Test 4: Move to a new settle window and check the realized pnl settle limit
    //
    // This time account 0's realized pnl settle limit kicks in.
    //
    let account_1_quote_before = mango_account_1.perps[0].quote_position_native();
    let account_0_realized_limit = mango_account_0.perps[0].recurring_settle_pnl_allowance;

    send_tx(
        solana,
        PerpSetSettleLimitWindow {
            group,
            admin,
            perp_market,
            window_size_ts: 10000, // guaranteed to move windows, resetting the limits
        },
    )
    .await
    .unwrap();

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

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    // successful settle of expected amount
    assert_eq!(
        mango_account_1.perps[0].quote_position_native() - account_1_quote_before,
        I80F48::from(account_0_realized_limit)
    );
    // account0's limit gets reduced to the pnl amount left over
    let perp_market_data = solana.get_account::<PerpMarket>(perp_market).await;
    assert_eq!(
        mango_account_0.perps[0].recurring_settle_pnl_allowance,
        mango_account_0.perps[0]
            .unsettled_pnl(&perp_market_data, I80F48::from_num(1.0))
            .unwrap()
    );

    // can't settle again
    assert!(send_tx(
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
    .is_err());

    //
    // Test 5: in a new settle window, the remaining pnl can be settled
    //

    let account_1_quote_before = mango_account_1.perps[0].quote_position_native();
    let account_0_realized_limit = mango_account_0.perps[0].recurring_settle_pnl_allowance;

    send_tx(
        solana,
        PerpSetSettleLimitWindow {
            group,
            admin,
            perp_market,
            window_size_ts: 5000, // guaranteed to move windows, resetting the limits
        },
    )
    .await
    .unwrap();

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

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    // successful settle of expected amount
    assert_eq!(
        mango_account_1.perps[0].quote_position_native() - account_1_quote_before,
        I80F48::from(account_0_realized_limit)
    );
    // account0's limit gets reduced to the realized pnl amount left over
    assert_eq!(mango_account_0.perps[0].recurring_settle_pnl_allowance, 0);
    assert_eq!(
        mango_account_0.perps[0].realized_pnl_for_position_native,
        I80F48::from(0)
    );
    assert_eq!(
        mango_account_1.perps[0].realized_pnl_for_position_native,
        I80F48::from(0)
    );

    Ok(())
}
