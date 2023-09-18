use super::*;

#[tokio::test]
async fn test_stale_oracle_deposit_withdraw() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(100_000); // bad oracles log a lot
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..3];
    let payer_token_accounts = &context.users[1].token_accounts[0..3];

    //
    // SETUP: Create a group, account, register tokens
    //

    let mango_setup::GroupWithTokens { group, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // fill vaults, so we can borrow
    let _vault_account = create_funded_account(
        &solana,
        group,
        owner,
        2,
        &context.users[1],
        mints,
        100000,
        0,
    )
    .await;

    // Create account with token0 deposits
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints[0..1],
        100,
        0,
    )
    .await;

    // Create some token1 borrows
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 10,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_token_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Make oracles invalid by increasing deviation
    send_tx(
        solana,
        StubOracleSetTestInstruction {
            group,
            mint: mints[0].pubkey,
            admin,
            price: 1.0,
            last_update_slot: 0,
            deviation: 100.0,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        StubOracleSetTestInstruction {
            group,
            mint: mints[1].pubkey,
            admin,
            price: 1.0,
            last_update_slot: 0,
            deviation: 100.0,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        StubOracleSetTestInstruction {
            group,
            mint: mints[2].pubkey,
            admin,
            price: 1.0,
            last_update_slot: 0,
            deviation: 100.0,
        },
    )
    .await
    .unwrap();

    // Can't activate a token position for a bad oracle
    assert!(send_tx(
        solana,
        TokenDepositInstruction {
            amount: 11,
            reduce_only: false,
            account,
            owner,
            token_account: payer_token_accounts[2],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .is_err());

    // Verify that creating a new borrow won't work
    assert!(send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_token_accounts[2],
            bank_index: 0,
        },
    )
    .await
    .is_err());

    // Repay token1 borrows
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 11,
            reduce_only: true,
            account,
            owner,
            token_account: payer_token_accounts[1],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Withdraw token0 deposits
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 100,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_token_accounts[0],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    Ok(())
}
