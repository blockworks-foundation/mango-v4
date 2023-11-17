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

    mango_client::send_tx(
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

    mango_client::send_tx(
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
