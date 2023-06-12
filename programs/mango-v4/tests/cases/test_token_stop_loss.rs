use super::*;

#[tokio::test]
async fn test_token_stop_loss() -> Result<(), TransportError> {
    pub use utils::assert_equal_f64_f64 as assert_equal_f_f;

    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];

    //
    // SETUP: Create a group, account, register a token (mint0)
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let quote_token = &tokens[0];
    let base_token = &tokens[1];

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
    let liqor = create_funded_account(
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
    // TEST: Trying to add a tsl on an account without space will fail
    //
    let tx_result = send_tx(
        solana,
        TokenStopLossCreateInstruction {
            account,
            owner,
            buy_token_index: quote_token.index,
            sell_token_index: base_token.index,
            max_buy: 1000,
            max_sell: 1000,
            price_threshold: 1.0,
            price_limit: 10.0,
            price_premium_bps: 100,
            allow_creating_deposits: true,
            allow_creating_borrows: true,
        },
    )
    .await;
    assert!(tx_result.is_err());

    //
    // TEST: Extending an account to have space for tsl works
    //
    send_tx(
        solana,
        AccountExpandInstruction {
            account_num: 0,
            token_count: 16,
            serum3_count: 8,
            perp_count: 8,
            perp_oo_count: 8,
            token_stop_loss_count: 2,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;
    let account_data = get_mango_account(solana, account).await;
    assert_eq!(account_data.header.token_stop_loss_count, 2);

    //
    // TEST: Can create tsls until all slots are filled
    //
    let tsl_ix = TokenStopLossCreateInstruction {
        account,
        owner,
        buy_token_index: quote_token.index,
        sell_token_index: base_token.index,
        max_buy: 100,
        max_sell: 100,
        price_threshold: 0.9,
        price_limit: 10.0,
        price_premium_bps: 1000,
        allow_creating_deposits: true,
        allow_creating_borrows: true,
    };
    send_tx(
        solana,
        TokenStopLossCreateInstruction {
            max_buy: 101,
            ..tsl_ix
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenStopLossCreateInstruction {
            max_buy: 102,
            price_threshold: 1.1,
            ..tsl_ix
        },
    )
    .await
    .unwrap();
    let tx_result = send_tx(solana, tsl_ix.clone()).await;
    assert!(tx_result.is_err());

    let account_data = get_mango_account(solana, account).await;
    assert_eq!(
        account_data.token_stop_loss_by_index(0).unwrap().max_buy,
        101
    );
    assert_eq!(
        account_data.token_stop_loss_by_index(1).unwrap().max_buy,
        102
    );

    //
    // TEST: Can cancel, and then readd a new one
    //
    send_tx(
        solana,
        TokenStopLossCancelInstruction {
            account,
            owner,
            index: 0,
            id: 0,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenStopLossCreateInstruction {
            max_buy: 103,
            ..tsl_ix
        },
    )
    .await
    .unwrap();
    let tx_result = send_tx(solana, tsl_ix.clone()).await;
    assert!(tx_result.is_err());

    let account_data = get_mango_account(solana, account).await;
    assert_eq!(
        account_data.token_stop_loss_by_index(0).unwrap().max_buy,
        103
    );
    assert_eq!(
        account_data.token_stop_loss_by_index(1).unwrap().max_buy,
        102
    );

    //
    // TEST: can't trigger if price threshold not reached
    //
    let tx_result = send_tx(
        solana,
        TokenStopLossTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 1,
            max_buy_token_to_give: 50,
            max_sell_token_to_receive: 50,
        },
    )
    .await;
    assert!(tx_result.is_err());

    //
    // TEST: trigger partially
    //
    send_tx(
        solana,
        TokenStopLossTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_give: 50,
            max_sell_token_to_receive: 50,
        },
    )
    .await
    .unwrap();

    let liqee_quote = account_position_f64(solana, account, quote_token.bank).await;
    let liqee_base = account_position_f64(solana, account, base_token.bank).await;
    assert!(assert_equal_f_f(
        liqee_quote,
        1000.0 + 46.0, // roughly 50 / 1.1
        0.01
    ));
    assert!(assert_equal_f_f(liqee_base, 1000.0 - 50.0, 0.01));

    let liqor_quote = account_position_f64(solana, liqor, quote_token.bank).await;
    let liqor_base = account_position_f64(solana, liqor, base_token.bank).await;
    assert!(assert_equal_f_f(liqor_quote, 1000.0 - 46.0, 0.01));
    assert!(assert_equal_f_f(liqor_base, 1000.0 + 50.0, 0.01));

    //
    // TEST: trigger fully
    //
    send_tx(
        solana,
        TokenStopLossTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_give: 5000,
            max_sell_token_to_receive: 5000,
        },
    )
    .await
    .unwrap();

    let liqee_quote = account_position_f64(solana, account, quote_token.bank).await;
    let liqee_base = account_position_f64(solana, account, base_token.bank).await;
    assert!(assert_equal_f_f(liqee_quote, 1000.0 + 92.0, 0.01));
    assert!(assert_equal_f_f(liqee_base, 1000.0 - 100.0, 0.01));

    let liqor_quote = account_position_f64(solana, liqor, quote_token.bank).await;
    let liqor_base = account_position_f64(solana, liqor, base_token.bank).await;
    assert!(assert_equal_f_f(liqor_quote, 1000.0 - 92.0, 0.01));
    assert!(assert_equal_f_f(liqor_base, 1000.0 + 100.0, 0.01));

    let account_data = get_mango_account(solana, account).await;
    assert!(!account_data
        .token_stop_loss_by_index(0)
        .unwrap()
        .is_active());

    Ok(())
}