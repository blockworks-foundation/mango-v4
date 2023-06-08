use super::*;

#[tokio::test]
async fn test_token_stop_loss() -> Result<(), TransportError> {
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
            price_threshold_type: TokenStopLossPriceThresholdType::PriceOverThreshold,
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
        max_buy: 1000,
        max_sell: 1000,
        price_threshold: 1.0,
        price_threshold_type: TokenStopLossPriceThresholdType::PriceOverThreshold,
        price_premium_bps: 100,
        allow_creating_deposits: true,
        allow_creating_borrows: true,
    };
    send_tx(
        solana,
        TokenStopLossCreateInstruction {
            max_buy: 1001,
            ..tsl_ix
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenStopLossCreateInstruction {
            max_buy: 1002,
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
        1001
    );
    assert_eq!(
        account_data.token_stop_loss_by_index(1).unwrap().max_buy,
        1002
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
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenStopLossCreateInstruction {
            max_buy: 1003,
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
        1003
    );
    assert_eq!(
        account_data.token_stop_loss_by_index(1).unwrap().max_buy,
        1002
    );

    Ok(())
}
