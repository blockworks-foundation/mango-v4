use super::*;

#[tokio::test]
async fn test_force_close_token() -> Result<(), TransportError> {
    let test_builder = TestContextBuilder::new();
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..4];
    let payer_mint_accounts = &context.users[1].token_accounts[0..4];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let collateral_token = &tokens[0];
    let borrow_token = &tokens[1];

    send_tx(
        solana,
        TokenEditWeights {
            group,
            admin,
            mint: mints[1].pubkey,
            init_asset_weight: 0.6,
            maint_asset_weight: 0.8,
            maint_liab_weight: 1.2,
            init_liab_weight: 1.5, // changed from 1.4
        },
    )
    .await
    .unwrap();

    // deposit some funds, to the vaults aren't empty
    create_funded_account(
        &solana,
        group,
        owner,
        99,
        &context.users[1],
        mints,
        100000,
        0,
    )
    .await;

    let deposit1_amount = 100;
    let liqor = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[0],
        &[mints[0]],
        deposit1_amount,
        0,
    )
    .await;

    //
    // SETUP: Make an account with some collateral and some borrows
    //
    let liqee = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[0],
        &[mints[0]],
        deposit1_amount,
        0,
    )
    .await;

    let borrow1_amount = 10;
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: borrow1_amount,
            allow_borrow: true,
            account: liqee,
            owner,
            token_account: payer_mint_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // test force close is enabled
    //
    assert!(send_tx(
        solana,
        TokenForceCloseBorrowsWithTokenInstruction {
            liqee: liqee,
            liqor: liqor,
            liqor_owner: owner,
            asset_token_index: collateral_token.index,
            liab_token_index: borrow_token.index,
            max_liab_transfer: 10000,
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .is_err());

    // set force close, and reduce only to 1
    send_tx(
        solana,
        TokenMakeReduceOnly {
            admin,
            group,
            mint: mints[1].pubkey,
            reduce_only: 1,
            force_close: false,
        },
    )
    .await
    .unwrap();

    //
    // test liqor needs deposits to be gte than the borrows it wants to liquidate
    //
    assert!(send_tx(
        solana,
        TokenForceCloseBorrowsWithTokenInstruction {
            liqee: liqee,
            liqor: liqor,
            liqor_owner: owner,
            asset_token_index: collateral_token.index,
            liab_token_index: borrow_token.index,
            max_liab_transfer: 10000,
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .is_err());

    //
    // test deposit with reduce only set to 1
    //
    let deposit1_amount = 11;
    assert!(send_tx(
        solana,
        TokenDepositInstruction {
            amount: deposit1_amount,
            reduce_only: false,
            account: liqor,
            owner,
            token_account: payer_mint_accounts[1],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .is_err());

    // set force close, and reduce only to 2
    send_tx(
        solana,
        TokenMakeReduceOnly {
            admin,
            group,
            mint: mints[1].pubkey,
            reduce_only: 2,
            force_close: true,
        },
    )
    .await
    .unwrap();

    //
    // test deposit with reduce only set to 2
    //
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: deposit1_amount,
            reduce_only: false,
            account: liqor,
            owner,
            token_account: payer_mint_accounts[1],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // test force close borrows
    //
    send_tx(
        solana,
        TokenForceCloseBorrowsWithTokenInstruction {
            liqee: liqee,
            liqor: liqor,
            liqor_owner: owner,
            asset_token_index: collateral_token.index,
            liab_token_index: borrow_token.index,
            max_liab_transfer: 10000,
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .unwrap();

    assert!(account_position_closed(solana, liqee, borrow_token.bank).await);
    assert_eq!(
        account_position(solana, liqee, collateral_token.bank).await,
        100 - 10
    );

    Ok(())
}

#[tokio::test]
async fn test_force_close_perp() -> Result<(), TransportError> {
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

    // Market needs to be in force close
    assert!(send_tx(
        solana,
        PerpForceClosePositionInstruction {
            account_a: account_0,
            account_b: account_1,
            perp_market: perp_market,
        },
    )
    .await
    .is_err());

    //
    // Set force close and force close position and verify that base position is 0
    //
    send_tx(
        solana,
        PerpMakeReduceOnly {
            admin,
            group,
            perp_market: perp_market,
            reduce_only: true,
            force_close: true,
        },
    )
    .await
    .unwrap();

    // account_a needs to be long, and account_b needs to be short
    assert!(send_tx(
        solana,
        PerpForceClosePositionInstruction {
            account_a: account_1,
            account_b: account_0,
            perp_market: perp_market,
        },
    )
    .await
    .is_err());

    send_tx(
        solana,
        PerpForceClosePositionInstruction {
            account_a: account_0,
            account_b: account_1,
            perp_market: perp_market,
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].base_position_lots(), 0);
    assert!(assert_equal(
        mango_account_0.perps[0].quote_position_native(),
        0.009,
        0.001
    ));
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].base_position_lots(), 0);
    assert!(assert_equal(
        mango_account_1.perps[0].quote_position_native(),
        -0.0199,
        0.001
    ));

    Ok(())
}

#[tokio::test]
async fn test_force_withdraw_token() -> Result<(), TransportError> {
    let test_builder = TestContextBuilder::new();
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..1];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let token = &tokens[0];

    let deposit_amount = 100;

    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[0],
        mints,
        deposit_amount,
        0,
    )
    .await;

    //
    // TEST: fails when force withdraw isn't enabled
    //
    assert!(send_tx(
        solana,
        TokenForceWithdrawInstruction {
            account,
            bank: token.bank,
            target: context.users[0].token_accounts[0],
        },
    )
    .await
    .is_err());

    // set force withdraw to enabled
    send_tx(
        solana,
        TokenEdit {
            admin,
            group,
            mint: token.mint.pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                maint_asset_weight_opt: Some(0.0),
                reduce_only_opt: Some(1),
                disable_asset_liquidation_opt: Some(true),
                force_withdraw_opt: Some(true),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    //
    // TEST: can't withdraw to foreign address
    //
    assert!(send_tx(
        solana,
        TokenForceWithdrawInstruction {
            account,
            bank: token.bank,
            target: context.users[1].token_accounts[0], // bad address/owner
        },
    )
    .await
    .is_err());

    //
    // TEST: passes and withdraws tokens
    //
    let token_account = context.users[0].token_accounts[0];
    let before_balance = solana.token_account_balance(token_account).await;
    send_tx(
        solana,
        TokenForceWithdrawInstruction {
            account,
            bank: token.bank,
            target: token_account,
        },
    )
    .await
    .unwrap();

    let after_balance = solana.token_account_balance(token_account).await;
    assert_eq!(after_balance, before_balance + deposit_amount);
    assert!(account_position_closed(solana, account, token.bank).await);

    Ok(())
}
