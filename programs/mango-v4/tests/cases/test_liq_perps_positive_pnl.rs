use super::*;

#[tokio::test]
async fn test_liq_perps_positive_pnl() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(170_000); // PerpLiqBaseOrPositivePnlInstruction takes a lot of CU
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

    let _quote_token = &tokens[0];
    let base_token = &tokens[1];
    let borrow_token = &tokens[2];
    let settle_token = &tokens[3];

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
            settle_token_index: 3,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_base_asset_weight: 0.8,
            init_base_asset_weight: 0.5,
            maint_base_liab_weight: 1.2,
            init_base_liab_weight: 1.5,
            maint_overall_asset_weight: 0.0,
            init_overall_asset_weight: 0.0,
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
            max_base_lots: 10,
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

    // after this order exchange it is changed by
    //   10*10*100*(0.5-1)*1.4 = -7000 for the long account0
    //   10*10*100*(1-1.5)*1.4 = -7000 for the short account1
    // (100 is base lot size)
    assert_eq!(
        account_init_health(solana, account_0).await.round(),
        (10000.0f64 - 1000.5 * 1.4 - 7000.0).round()
    );
    assert_eq!(
        account_init_health(solana, account_1).await.round(),
        10000.0 - 7000.0
    );

    //
    // SETUP: Change the perp oracle to make perp-based health go positive for account_0
    // perp base value goes to 10*21*100*0.5, exceeding the negative quote
    // perp uhupnl is 10*21*100*0.5 - 10*10*100 = 500
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
            max_pnl_transfer: 0,
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
            max_pnl_transfer: 100,
        },
    )
    .await
    .unwrap();

    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 0);
    assert_eq!(liqor_data.perps[0].quote_position_native(), 100);
    assert_eq!(
        account_position(solana, liqor, settle_token.bank).await,
        10000 - 95
    );
    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 10);
    assert_eq!(liqee_data.perps[0].quote_position_native(), -10100);
    assert_eq!(
        account_position(solana, account_0, settle_token.bank).await,
        95
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
            max_pnl_transfer: 600,
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
        account_position(solana, liqor, settle_token.bank).await,
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
        account_position(solana, account_0, settle_token.bank).await,
        95 + 570
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
            max_pnl_transfer: 0,
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

    send_tx(
        solana,
        PerpChangeWeights {
            group,
            admin,
            perp_market,
            init_overall_asset_weight: 0.6,
            maint_overall_asset_weight: 0.8,
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
            max_pnl_transfer: 0,
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
            max_pnl_transfer: u64::MAX,
        },
    )
    .await
    .unwrap();

    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 1);
    let health = account_init_health(solana, account_0).await;
    assert!(health > 0.0);
    assert!(health < 1.0);

    Ok(())
}
