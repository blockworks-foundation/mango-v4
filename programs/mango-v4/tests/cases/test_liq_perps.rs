use super::*;

#[tokio::test]
async fn test_liq_perps_force_cancel() -> Result<(), TransportError> {
    let test_builder = TestContextBuilder::new();
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..2];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    //let quote_token = &tokens[0];
    let base_token = &tokens[1];

    // deposit some funds, to the vaults aren't empty
    create_funded_account(&solana, group, owner, 0, &context.users[1], mints, 10000, 0).await;

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
            maint_base_asset_weight: 0.8,
            init_base_asset_weight: 0.6,
            maint_base_liab_weight: 1.2,
            init_base_liab_weight: 1.4,
            base_liquidation_fee: 0.05,
            maker_fee: 0.0,
            taker_fee: 0.0,
            settle_pnl_limit_factor: 0.2,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, base_token).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::ONE)
    };

    //
    // SETUP: Make an account and deposit some quote and base
    //
    let deposit_amount = 1000;
    let account = create_funded_account(
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

    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 1,
            reduce_only: false,
            account,
            owner,
            token_account: payer_mint_accounts[1],
            token_authority: payer,
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Place a perp order
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            // health was 1000 * 0.6 = 600; this order is -14*100*(1.4-1) = -560
            max_base_lots: 14,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Change the oracle to make health go negative
    //
    set_bank_stub_oracle_price(solana, group, base_token, admin, 10.0).await;

    // verify health is bad: can't withdraw
    assert!(send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint_accounts[1],
            bank_index: 0,
        }
    )
    .await
    .is_err());

    //
    // TEST: force cancel orders, making the account healthy again
    //
    send_tx(
        solana,
        PerpLiqForceCancelOrdersInstruction {
            account,
            perp_market,
        },
    )
    .await
    .unwrap();

    // can withdraw again
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    Ok(())
}

#[tokio::test]
async fn test_liq_perps_base_position_and_bankruptcy() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(120_000); // PerpLiqBaseOrPositivePnl takes a lot of CU
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..2];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let GroupWithTokens {
        group,
        tokens,
        insurance_vault,
        ..
    } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        zero_token_is_quote: true,
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // fund the insurance vault
    let insurance_vault_funding = 100;
    {
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction_direct(
            spl_token::instruction::transfer(
                &spl_token::ID,
                &payer_mint_accounts[0],
                &insurance_vault,
                &payer.pubkey(),
                &[&payer.pubkey()],
                insurance_vault_funding,
            )
            .unwrap(),
        );
        tx.add_signer(payer);
        tx.send().await.unwrap();
    }

    let quote_token = &tokens[0];
    let base_token = &tokens[1];

    // deposit some funds, to the vaults aren't empty
    let liqor = create_funded_account(
        &solana,
        group,
        owner,
        250,
        &context.users[1],
        mints,
        10000,
        0,
    )
    .await;
    let settler =
        create_funded_account(&solana, group, owner, 251, &context.users[1], &[], 0, 0).await;
    let settler_owner = owner.clone();

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
            maint_base_asset_weight: 0.8,
            init_base_asset_weight: 0.6,
            maint_base_liab_weight: 1.2,
            init_base_liab_weight: 1.4,
            base_liquidation_fee: 0.05,
            maker_fee: 0.0,
            taker_fee: 0.0,
            group_insurance_fund: true,
            settle_pnl_limit_factor: 0.2,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, base_token).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::ONE)
    };

    //
    // SETUP: Make an two accounts and deposit some quote and base
    //
    let context_ref = &context;
    let make_account = |idx: u32| async move {
        let deposit_amount = 1000;
        let account = create_funded_account(
            &solana,
            group,
            owner,
            idx,
            &context_ref.users[1],
            &mints[0..1],
            deposit_amount,
            0,
        )
        .await;

        account
    };
    let account_0 = make_account(0).await;
    let account_1 = make_account(1).await;

    //
    // SETUP: Trade perps between accounts
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
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
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
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
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

    // health was 1000 before;
    // after this order exchange it is changed by
    //   20*100*(0.6-1) = -800 for the long account0
    //   20*100*(1-1.4) = -800 for the short account1
    // (100 is base lot size)
    assert_eq!(
        account_init_health(solana, account_0).await.round(),
        1000.0 - 800.0
    );
    assert_eq!(
        account_init_health(solana, account_1).await.round(),
        1000.0 - 800.0
    );

    //
    // SETUP: Change the oracle to make health go negative for account_0
    // perp base value decreases from 2000 * 0.6 to 2000 * 0.6 * 0.6, i.e. -480
    //
    set_bank_stub_oracle_price(solana, group, base_token, admin, 0.6).await;
    assert_eq!(
        account_init_health(solana, account_0).await.round(),
        200.0 - 480.0
    );

    //
    // TEST: Liquidate base position with limit
    //
    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_0,
            perp_market,
            max_base_transfer: 10,
            max_quote_transfer: 0,
        },
    )
    .await
    .unwrap();

    let liq_amount = 10.0 * 100.0 * 0.6 * (1.0 - 0.05);
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 10);
    assert!(assert_equal(
        liqor_data.perps[0].quote_position_native(),
        -liq_amount,
        0.1
    ));
    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 10);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        -20.0 * 100.0 + liq_amount,
        0.1
    ));
    assert!(assert_equal(
        liqee_data.perps[0].realized_trade_pnl_native,
        liq_amount - 1000.0,
        0.1
    ));
    // stable price is 1.0, so 0.2 * 1000
    assert_eq!(liqee_data.perps[0].settle_pnl_limit_realized_trade, -201);

    //
    // TEST: Liquidate base position max
    //
    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_0,
            perp_market,
            max_base_transfer: i64::MAX,
            max_quote_transfer: 0,
        },
    )
    .await
    .unwrap();

    let liq_amount_2 = 4.0 * 100.0 * 0.6 * (1.0 - 0.05);
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 10 + 4);
    assert!(assert_equal(
        liqor_data.perps[0].quote_position_native(),
        -liq_amount - liq_amount_2,
        0.1
    ));
    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 6);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        -20.0 * 100.0 + liq_amount + liq_amount_2,
        0.1
    ));

    // verify health is good again
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: false,
            account: account_0,
            owner,
            token_account: payer_mint_accounts[0],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Change the oracle to make health go negative for account_1
    //
    set_bank_stub_oracle_price(solana, group, base_token, admin, 1.3).await;

    // verify health is bad: can't withdraw
    assert!(send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: false,
            account: account_1,
            owner,
            token_account: payer_mint_accounts[0],
            bank_index: 0,
        }
    )
    .await
    .is_err());

    //
    // TEST: Liquidate base position
    //
    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_1,
            perp_market,
            max_base_transfer: -10,
            max_quote_transfer: 0,
        },
    )
    .await
    .unwrap();

    let liq_amount_3 = 10.0 * 100.0 * 1.3 * (1.0 + 0.05);
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 14 - 10);
    assert!(assert_equal(
        liqor_data.perps[0].quote_position_native(),
        -liq_amount - liq_amount_2 + liq_amount_3,
        0.1
    ));
    let liqee_data = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), -10);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        20.0 * 100.0 - liq_amount_3,
        0.1
    ));

    //
    // TEST: Liquidate base position max
    //
    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_1,
            perp_market,
            max_base_transfer: i64::MIN,
            max_quote_transfer: 0,
        },
    )
    .await
    .unwrap();

    let liq_amount_4 = 5.0 * 100.0 * 1.3 * (1.0 + 0.05);
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 4 - 5);
    assert!(assert_equal(
        liqor_data.perps[0].quote_position_native(),
        -liq_amount - liq_amount_2 + liq_amount_3 + liq_amount_4,
        0.1
    ));
    let liqee_data = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), -5);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        20.0 * 100.0 - liq_amount_3 - liq_amount_4,
        0.1
    ));

    // verify health is good again
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: false,
            account: account_1,
            owner,
            token_account: payer_mint_accounts[0],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: liquidate base position to 0, so bankruptcy can be tested
    //
    set_bank_stub_oracle_price(solana, group, base_token, admin, 2.0).await;

    //
    // TEST: Liquidate base position max
    //
    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_1,
            perp_market,
            max_base_transfer: i64::MIN,
            max_quote_transfer: 0,
        },
    )
    .await
    .unwrap();

    let liq_amount_5 = 5.0 * 100.0 * 2.0 * (1.0 + 0.05);
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), -1 - 5);
    assert!(assert_equal(
        liqor_data.perps[0].quote_position_native(),
        -liq_amount - liq_amount_2 + liq_amount_3 + liq_amount_4 + liq_amount_5,
        0.1
    ));
    let liqee_data = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 0);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        20.0 * 100.0 - liq_amount_3 - liq_amount_4 - liq_amount_5,
        0.1
    ));

    //
    // SETUP: We want pnl settling to cause a negative quote position,
    // thus we deposit some base token collateral. To be able to do that,
    // we need to temporarily raise health > 0, deposit, then bring health
    // negative again for the test
    //
    set_bank_stub_oracle_price(solana, group, quote_token, admin, 2.0).await;
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 1,
            reduce_only: false,
            account: account_1,
            owner,
            token_account: payer_mint_accounts[1],
            token_authority: payer,
            bank_index: 0,
        },
    )
    .await
    .unwrap();
    set_bank_stub_oracle_price(solana, group, quote_token, admin, 1.0).await;

    //
    // TEST: Can settle-pnl even though health is negative
    //
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    let perp_market_data = solana.get_account::<PerpMarket>(perp_market).await;
    let liqor_max_settle = liqor_data.perps[0]
        .available_settle_limit(&perp_market_data)
        .1;
    let account_1_quote_before = account_position(solana, account_1, quote_token.bank).await;

    send_tx(
        solana,
        PerpSettlePnlInstruction {
            settler,
            settler_owner,
            account_a: liqor,
            account_b: account_1,
            perp_market,
            settle_bank: tokens[0].bank,
        },
    )
    .await
    .unwrap();

    let liqee_settle_health_before: f64 = 999.0 + 1.0 * 2.0 * 0.8;
    // the liqor's settle limit means we can't settle everything
    let settle_amount = liqee_settle_health_before.min(liqor_max_settle as f64);
    let remaining_pnl = 20.0 * 100.0 - liq_amount_3 - liq_amount_4 - liq_amount_5 + settle_amount;
    assert!(remaining_pnl < 0.0);
    let liqee_data = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 0);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        remaining_pnl,
        0.1
    ));
    assert_eq!(
        account_position(solana, account_1, quote_token.bank).await,
        account_1_quote_before - settle_amount as i64
    );
    assert_eq!(
        account_position(solana, account_1, base_token.bank).await,
        1
    );

    //
    // TEST: Can liquidate/bankruptcy away remaining negative pnl
    //
    let liqee_before = solana.get_account::<MangoAccount>(account_1).await;
    let liqor_before = solana.get_account::<MangoAccount>(liqor).await;
    let liqee_settle_limit_before = liqee_before.perps[0]
        .available_settle_limit(&perp_market_data)
        .0;
    send_tx(
        solana,
        PerpLiqNegativePnlOrBankruptcyInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_1,
            perp_market,
            max_liab_transfer: u64::MAX,
        },
    )
    .await
    .unwrap();
    let liqee_after = solana.get_account::<MangoAccount>(account_1).await;
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    let quote_bank = solana.get_account::<Bank>(tokens[0].bank).await;

    // the amount of spot the liqor received: full insurance fund, plus what was still settleable
    let liq_spot_amount = insurance_vault_funding as f64 + (-liqee_settle_limit_before) as f64;
    // the amount of perp quote transfered
    let liq_perp_quote_amount =
        (insurance_vault_funding as f64) / 1.05 + (-liqee_settle_limit_before) as f64;

    // insurance fund was depleted and the liqor received it
    assert_eq!(solana.token_account_balance(insurance_vault).await, 0);
    assert!(assert_equal(
        liqor_data.tokens[0].native(&quote_bank),
        liqor_before.tokens[0].native(&quote_bank).to_num::<f64>() + liq_spot_amount,
        0.1
    ));

    // liqor took over the max possible negative pnl
    assert!(assert_equal(
        liqor_data.perps[0].quote_position_native(),
        liqor_before.perps[0]
            .quote_position_native()
            .to_num::<f64>()
            - liq_perp_quote_amount,
        0.1
    ));

    // liqee exited liquidation
    assert!(account_init_health(solana, account_1).await >= 0.0);
    assert_eq!(liqee_after.being_liquidated, 0);

    // the remainder got socialized via funding payments
    let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
    let pnl_before = liqee_before.perps[0]
        .unsettled_pnl(&perp_market, I80F48::ONE)
        .unwrap();
    let pnl_after = liqee_after.perps[0]
        .unsettled_pnl(&perp_market, I80F48::ONE)
        .unwrap();
    let socialized_amount = (pnl_after - pnl_before).to_num::<f64>() - liq_perp_quote_amount;
    assert!(assert_equal(
        perp_market.long_funding,
        socialized_amount / 20.0,
        0.1
    ));
    assert!(assert_equal(
        perp_market.short_funding,
        -socialized_amount / 20.0,
        0.1
    ));

    Ok(())
}

#[tokio::test]
async fn test_liq_perps_base_position_overall_weight() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(120_000); // PerpLiqNegativePnlOrBankruptcy takes a lot of CU
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..3];
    let payer_mint_accounts = &context.users[1].token_accounts[0..3];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let GroupWithTokens {
        group,
        tokens,
        insurance_vault,
        ..
    } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        zero_token_is_quote: true,
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // fund the insurance vault
    let insurance_vault_funding = 100;
    {
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction_direct(
            spl_token::instruction::transfer(
                &spl_token::ID,
                &payer_mint_accounts[0],
                &insurance_vault,
                &payer.pubkey(),
                &[&payer.pubkey()],
                insurance_vault_funding,
            )
            .unwrap(),
        );
        tx.add_signer(payer);
        tx.send().await.unwrap();
    }

    let quote_token = &tokens[0];
    let base_token = &tokens[1];
    let borrow_token = &tokens[2];

    // deposit some funds, to the vaults aren't empty
    let liqor = create_funded_account(
        &solana,
        group,
        owner,
        250,
        &context.users[1],
        mints,
        10000,
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
            maint_base_asset_weight: 0.8,
            init_base_asset_weight: 0.5,
            maint_base_liab_weight: 1.2,
            init_base_liab_weight: 1.5,
            maint_pnl_asset_weight: 0.0,
            init_pnl_asset_weight: 0.0,
            base_liquidation_fee: 0.05,
            positive_pnl_liquidation_fee: 0.05,
            maker_fee: 0.0,
            taker_fee: 0.0,
            group_insurance_fund: true,
            settle_pnl_limit_factor: 0.2,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, base_token).await
        },
    )
    .await
    .unwrap();

    set_perp_stub_oracle_price(solana, group, perp_market, &base_token, admin, 10.0).await;
    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::from(10))
    };

    //
    // SETUP: Make an two accounts and deposit some quote and base
    //
    let context_ref = &context;
    let make_account = |idx: u32| async move {
        let deposit_amount = 10000;
        let account = create_funded_account(
            &solana,
            group,
            owner,
            idx,
            &context_ref.users[1],
            &mints[0..1],
            deposit_amount,
            0,
        )
        .await;

        account
    };
    let account_0 = make_account(0).await;
    let account_1 = make_account(1).await;

    //
    // SETUP: Borrow some spot on account_0, so we can later make it liquidatable that way
    // (actually borrowing 1000.5 due to loan origination!)
    //
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1000,
            allow_borrow: true,
            account: account_0,
            owner,
            token_account: payer_mint_accounts[2],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Trade perps between accounts
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 10,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
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
            max_base_lots: 10,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
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

    // after this order exchange it is changed by
    //   10*10*100*(0.5-1) = -5000 for the long account0
    //   10*10*100*(1-1.5) = -5000 for the short account1
    // (100 is base lot size)
    assert_eq!(
        account_init_health(solana, account_0).await.round(),
        (10000.0f64 - 1000.5 * 1.4 - 5000.0).round()
    );
    assert_eq!(
        account_init_health(solana, account_1).await.round(),
        10000.0 - 5000.0
    );

    //
    // SETUP: Change the perp oracle to make perp-based health go positive for account_0
    // perp base value goes to 10*21*100*0.5, exceeding the negative quote
    // unweighted perp health is 10*1*100*0.5 = 500
    // but health doesn't exceed 10k because of the 0 overall weight
    //
    set_perp_stub_oracle_price(solana, group, perp_market, &base_token, admin, 21.0).await;
    assert_eq!(
        account_init_health(solana, account_0).await.round(),
        (10000.0f64 - 1000.5 * 1.4).round()
    );

    //
    // SETUP: Increase the price of the borrow so account_0 becomes liquidatable
    //
    set_bank_stub_oracle_price(solana, group, &borrow_token, admin, 10.0).await;
    assert_eq!(
        account_init_health(solana, account_0).await.round(),
        (10000.0f64 - 10.0 * 1000.5 * 1.4).round()
    );

    //
    // TEST: Can't liquidate base if health wouldn't go up: no effect
    //
    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_0,
            perp_market,
            max_base_transfer: i64::MAX,
            max_quote_transfer: 0,
        },
    )
    .await
    .unwrap();

    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 0);
    assert_eq!(liqor_data.perps[0].quote_position_native(), 0);
    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 10);

    //
    // TEST: Can take over existing positive pnl without eating base position
    //
    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_0,
            perp_market,
            max_base_transfer: i64::MAX,
            max_quote_transfer: 100,
        },
    )
    .await
    .unwrap();

    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 0);
    assert_eq!(liqor_data.perps[0].quote_position_native(), 100);
    assert_eq!(
        account_position(solana, liqor, quote_token.bank).await,
        10000 - 95
    );
    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 10);
    assert_eq!(liqee_data.perps[0].quote_position_native(), -10100);
    assert_eq!(
        account_position(solana, account_0, quote_token.bank).await,
        10000 + 95
    );

    //
    // TEST: Being willing to take over more positive pnl can trigger more base liquidation
    //
    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_0,
            perp_market,
            max_base_transfer: i64::MAX,
            max_quote_transfer: 600,
        },
    )
    .await
    .unwrap();

    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 1);
    assert!(assert_equal(
        liqor_data.perps[0].quote_position_native(),
        100.0 + 600.0 - 2100.0 * 0.95,
        0.1
    ));
    assert_eq!(
        account_position(solana, liqor, quote_token.bank).await,
        10000 - 95 - 570
    );
    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 9);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        -10000.0 - 100.0 - 600.0 + 2100.0 * 0.95,
        0.1
    ));
    assert_eq!(
        account_position(solana, account_0, quote_token.bank).await,
        10000 + 95 + 570
    );

    //
    // TEST: can liquidate to increase perp health until >= 0
    //

    // perp base value goes to 9*19*100*0.5
    // unweighted perp health changes by -9*2*100*0.5 = -900
    // this makes the perp health contribution negative!
    set_perp_stub_oracle_price(solana, group, perp_market, &base_token, admin, 19.0).await;

    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_0,
            perp_market,
            max_base_transfer: i64::MAX,
            max_quote_transfer: 0,
        },
    )
    .await
    .unwrap();

    // liquidated one base lot only, even though health is still negative!
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 2);
    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 8);

    //
    // TEST: if overall perp health weight is >0, we can liquidate the base position further
    //

    // reduce the price some more, so the liq instruction can do some of step1 and step2
    set_perp_stub_oracle_price(solana, group, perp_market, &base_token, admin, 17.0).await;

    send_tx(
        solana,
        PerpChangeWeights {
            group,
            admin,
            perp_market,
            init_pnl_asset_weight: 0.6,
            maint_pnl_asset_weight: 0.8,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_0,
            perp_market,
            max_base_transfer: 3,
            max_quote_transfer: 0,
        },
    )
    .await
    .unwrap();

    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 5);
    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 5);

    //
    // TEST: can bring the account to just above >0 health if desired
    //

    send_tx(
        solana,
        PerpLiqBaseOrPositivePnlInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_0,
            perp_market,
            max_base_transfer: i64::MAX,
            max_quote_transfer: u64::MAX,
        },
    )
    .await
    .unwrap();

    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 2);
    let health = account_init_health(solana, account_0).await;
    assert!(health > 0.0);
    assert!(health < 1.0);

    Ok(())
}

#[tokio::test]
async fn test_liq_perps_bankruptcy() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(200_000); // PerpLiqNegativePnlOrBankruptcy takes a lot of CU
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..3];
    let payer_mint_accounts = &context.users[1].token_accounts[0..3];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let GroupWithTokens {
        group,
        tokens,
        insurance_vault,
        ..
    } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        zero_token_is_quote: true,
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    send_tx(
        solana,
        TokenEditWeights {
            group,
            admin,
            mint: mints[2].pubkey,
            maint_liab_weight: 1.0,
            maint_asset_weight: 1.0,
            init_liab_weight: 1.0,
            init_asset_weight: 1.0,
        },
    )
    .await
    .unwrap();

    let fund_insurance = |amount: u64| async move {
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction_direct(
            spl_token::instruction::transfer(
                &spl_token::ID,
                &payer_mint_accounts[0],
                &insurance_vault,
                &payer.pubkey(),
                &[&payer.pubkey()],
                amount,
            )
            .unwrap(),
        );
        tx.add_signer(payer);
        tx.send().await.unwrap();
    };

    let quote_token = &tokens[0]; // USDC, 1/1 weights, price 1, never changed
    let base_token = &tokens[1]; // used for perp market
    let collateral_token = &tokens[2]; // used for adjusting account health

    // deposit some funds, to the vaults aren't empty
    let liqor = create_funded_account(
        &solana,
        group,
        owner,
        250,
        &context.users[1],
        mints,
        10000,
        0,
    )
    .await;

    // all perp markets used here default to price = 1.0, base_lot_size = 100
    let price_lots = 100;

    let context_ref = &context;
    let mut perp_market_index: PerpMarketIndex = 0;
    let setup_perp_inner = |perp_market_index: PerpMarketIndex,
                            health: i64,
                            pnl: i64,
                            settle_limit: i64| async move {
        // price used later to produce negative pnl with a short:
        // doubling the price leads to -100 pnl
        let adj_price = 1.0 + pnl as f64 / -100.0;
        let adj_price_lots = (price_lots as f64 * adj_price) as i64;

        let mango_v4::accounts::PerpCreateMarket { perp_market, .. } = send_tx(
            solana,
            PerpCreateMarketInstruction {
                group,
                admin,
                payer,
                perp_market_index,
                quote_lot_size: 1,
                base_lot_size: 100,
                maint_base_asset_weight: 0.8,
                init_base_asset_weight: 0.6,
                maint_base_liab_weight: 1.2,
                init_base_liab_weight: 1.4,
                base_liquidation_fee: 0.05,
                maker_fee: 0.0,
                taker_fee: 0.0,
                group_insurance_fund: true,
                // adjust this factur such that we get the desired settle limit in the end
                settle_pnl_limit_factor: (settle_limit as f32 + 0.1).min(0.0)
                    / (-1.0 * 100.0 * adj_price) as f32,
                settle_pnl_limit_window_size_ts: 24 * 60 * 60,
                ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, base_token).await
            },
        )
        .await
        .unwrap();
        set_perp_stub_oracle_price(solana, group, perp_market, &base_token, admin, 1.0).await;
        set_bank_stub_oracle_price(solana, group, &collateral_token, admin, 1.0).await;

        //
        // SETUP: accounts
        //
        let deposit_amount = 1000;
        let helper_account = create_funded_account(
            &solana,
            group,
            owner,
            perp_market_index as u32 * 2,
            &context_ref.users[1],
            &mints[2..3],
            deposit_amount,
            0,
        )
        .await;
        let account = create_funded_account(
            &solana,
            group,
            owner,
            perp_market_index as u32 * 2 + 1,
            &context_ref.users[1],
            &mints[2..3],
            deposit_amount,
            0,
        )
        .await;

        //
        // SETUP: Trade perps between accounts twice to generate pnl, settle_limit
        //
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction(PerpPlaceOrderInstruction {
            account: helper_account,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 0,
            reduce_only: false,
        })
        .await;
        tx.add_instruction(PerpPlaceOrderInstruction {
            account: account,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 0,
            reduce_only: false,
        })
        .await;
        tx.add_instruction(PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account, helper_account],
        })
        .await;
        tx.send().await.unwrap();

        set_perp_stub_oracle_price(solana, group, perp_market, &base_token, admin, adj_price).await;
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction(PerpPlaceOrderInstruction {
            account: helper_account,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots: adj_price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 0,
            reduce_only: false,
        })
        .await;
        tx.add_instruction(PerpPlaceOrderInstruction {
            account: account,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots: adj_price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 0,
            reduce_only: false,
        })
        .await;
        tx.add_instruction(PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account, helper_account],
        })
        .await;
        tx.send().await.unwrap();

        set_perp_stub_oracle_price(solana, group, perp_market, &base_token, admin, 1.0).await;

        // Adjust target health:
        // full health = 1000 * collat price * 1.0 + pnl
        set_bank_stub_oracle_price(
            solana,
            group,
            &collateral_token,
            admin,
            (health - pnl) as f64 / 1000.0,
        )
        .await;

        // Verify we got it right
        let account_data = solana.get_account::<MangoAccount>(account).await;
        assert_eq!(account_data.perps[0].quote_position_native(), pnl);
        assert_eq!(
            account_data.perps[0].settle_pnl_limit_realized_trade,
            settle_limit
        );
        assert_eq!(
            account_init_health(solana, account).await.round(),
            health as f64
        );

        (perp_market, account)
    };
    let mut setup_perp = |health: i64, pnl: i64, settle_limit: i64| {
        let out = setup_perp_inner(perp_market_index, health, pnl, settle_limit);
        perp_market_index += 1;
        out
    };

    let limit_prec = |f: f64| (f * 1000.0).round() / 1000.0;

    let liq_event_amounts = || {
        let settlement = solana
            .program_log_events::<mango_v4::logs::PerpLiqNegativePnlOrBankruptcyLog>()
            .pop()
            .map(|v| limit_prec(I80F48::from_bits(v.settlement).to_num::<f64>()))
            .unwrap_or(0.0);
        let (insur, loss) = solana
            .program_log_events::<mango_v4::logs::PerpLiqBankruptcyLog>()
            .pop()
            .map(|v| {
                (
                    I80F48::from_bits(v.insurance_transfer).to_num::<u64>(),
                    limit_prec(I80F48::from_bits(v.socialized_loss).to_num::<f64>()),
                )
            })
            .unwrap_or((0, 0.0));
        (settlement, insur, loss)
    };

    let liqor_info = |perp_market: Pubkey| async move {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
        let liqor_perp = liqor_data
            .perps
            .iter()
            .find(|p| p.market_index == perp_market.perp_market_index)
            .unwrap()
            .clone();
        (liqor_data, liqor_perp)
    };

    {
        let (perp_market, account) = setup_perp(-28, -50, -10).await;
        let liqor_quote_before = account_position(solana, liqor, quote_token.bank).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: 1,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (1.0, 0, 0.0));

        assert_eq!(
            account_position(solana, account, quote_token.bank).await,
            -1
        );
        assert_eq!(
            account_position(solana, liqor, quote_token.bank).await,
            liqor_quote_before + 1
        );
        let acc_data = solana.get_account::<MangoAccount>(account).await;
        assert_eq!(acc_data.perps[0].quote_position_native(), -49);
        assert_eq!(acc_data.being_liquidated, 1);
        let (_liqor_data, liqor_perp) = liqor_info(perp_market).await;
        assert_eq!(liqor_perp.quote_position_native(), -1);
    }

    {
        let (perp_market, account) = setup_perp(-28, -50, -10).await;
        fund_insurance(2).await;
        let liqor_quote_before = account_position(solana, liqor, quote_token.bank).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: 11,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (10.0, 2, 27.0));

        assert_eq!(
            account_position(solana, account, quote_token.bank).await,
            -10
        );
        assert_eq!(
            account_position(solana, liqor, quote_token.bank).await,
            liqor_quote_before + 12
        );
        let acc_data = solana.get_account::<MangoAccount>(account).await;
        assert!(assert_equal(
            acc_data.perps[0].quote_position_native(),
            -50.0 + 11.0 + 27.0,
            0.1
        ));
        assert_eq!(acc_data.being_liquidated, 0);
        let (_liqor_data, liqor_perp) = liqor_info(perp_market).await;
        assert_eq!(liqor_perp.quote_position_native(), -11);
    }

    {
        let (perp_market, account) = setup_perp(-28, -50, -10).await;
        fund_insurance(5).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: 16,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            liq_event_amounts(),
            (10.0, 5, limit_prec(28.0 - 5.0 / 1.05))
        );
    }

    // no insurance
    {
        let (perp_market, account) = setup_perp(-28, -50, -10).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: u64::MAX,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (10.0, 0, limit_prec(28.0)));
    }

    // no settlement: no settle health
    {
        let (perp_market, account) = setup_perp(-200, -50, -10).await;
        fund_insurance(5).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: u64::MAX,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (0.0, 5, limit_prec(50.0 - 5.0 / 1.05)));
    }

    // no settlement: no settle limit
    {
        let (perp_market, account) = setup_perp(-40, -50, 0).await;
        // no insurance

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: u64::MAX,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (0.0, 0, limit_prec(40.0)));
    }

    // no socialized loss: fully covered by insurance fund
    {
        let (perp_market, account) = setup_perp(-40, -50, -5).await;
        fund_insurance(42).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: u64::MAX,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (5.0, 42, 0.0));
    }

    Ok(())
}
