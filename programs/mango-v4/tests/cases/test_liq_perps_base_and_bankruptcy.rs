use super::*;

#[tokio::test]
async fn test_liq_perps_base_and_bankruptcy() -> Result<(), TransportError> {
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
            max_pnl_transfer: 0,
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
            max_pnl_transfer: 0,
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
            max_pnl_transfer: 0,
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
            max_pnl_transfer: 0,
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
            max_pnl_transfer: 0,
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
