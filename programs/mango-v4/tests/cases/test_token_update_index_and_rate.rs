use super::*;

#[tokio::test]
async fn test_token_update_index_and_rate() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];

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

    // deposit some funds, to the vaults aren't empty
    create_funded_account(&solana, group, owner, 0, &context.users[1], mints, 10000, 0).await;
    let withdraw_account = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        &mints[1..2],
        100000,
        0,
    )
    .await;

    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 5000,
            allow_borrow: true,
            account: withdraw_account,
            owner,
            token_account: context.users[0].token_accounts[0],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    let bank_before = solana.get_account::<Bank>(tokens[0].bank).await;

    let time_before = solana.clock().await.unix_timestamp;
    solana.advance_clock().await;
    let time_after = solana.clock().await.unix_timestamp;

    send_tx(
        solana,
        TokenUpdateIndexAndRateInstruction {
            mint_info: tokens[0].mint_info,
        },
    )
    .await
    .unwrap();

    let bank_after = solana.get_account::<Bank>(tokens[0].bank).await;
    dbg!(bank_after);
    dbg!(bank_after);

    let utilization = 0.5; // 10000 deposits / 5000 borrows
    let diff_ts = (time_after - time_before) as f64;
    let year = 31536000.0;
    let loan_fee_rate = 0.0005;
    let dynamic_rate = 0.07 + 0.9 * (utilization - 0.4) / (0.8 - 0.4);
    let interest_change = 5000.0 * (dynamic_rate + loan_fee_rate) * diff_ts / year;
    let fee_change = 5000.0 * loan_fee_rate * diff_ts / year;

    assert!(assert_equal(
        bank_after.native_borrows() - bank_before.native_borrows(),
        interest_change,
        0.1
    ));
    assert!(assert_equal(
        bank_after.native_deposits() - bank_before.native_deposits(),
        interest_change,
        0.1
    ));
    assert!(assert_equal(
        bank_after.collected_fees_native - bank_before.collected_fees_native,
        fee_change,
        0.1
    ));
    assert!(assert_equal(bank_after.avg_utilization, utilization, 0.01));

    Ok(())
}

#[tokio::test]
async fn test_token_rates_migrate() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let payer = context.users[1].key;
    let mints = &context.mints[0..1];

    let mango_setup::GroupWithTokens { tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        zero_token_is_quote: true,
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let start_time = solana.clock_timestamp().await;

    // Change the bank to have the old on-chain state without curveScaling
    let mut bank_before = solana.get_account::<Bank>(tokens[0].bank).await;
    bank_before.interest_curve_scaling = 0.0;
    bank_before.interest_target_utilization = 0.0;
    bank_before.adjustment_factor = I80F48::ZERO; // so we don't need to compute expected rate changes
    solana.set_account(tokens[0].bank, &bank_before).await;

    // Update index and rate is allowed every hour
    solana.set_clock_timestamp(start_time + 3601).await;

    send_tx(
        solana,
        TokenUpdateIndexAndRateInstruction {
            mint_info: tokens[0].mint_info,
        },
    )
    .await
    .unwrap();

    let bank_after = solana.get_account::<Bank>(tokens[0].bank).await;

    assert!(assert_equal_fixed_f64(bank_after.rate0, 0.07 / 3.0, 0.0001));
    assert!(assert_equal_fixed_f64(bank_after.rate1, 0.9 / 3.0, 0.0001));
    assert!(assert_equal_fixed_f64(bank_after.max_rate, 0.5, 0.0001));
    assert!(assert_equal_f64_f64(
        bank_after.interest_curve_scaling,
        3.0,
        0.0001
    ));
    assert!(assert_equal_f64_f64(
        bank_after.interest_target_utilization as f64,
        0.4,
        0.0001
    ));

    Ok(())
}
