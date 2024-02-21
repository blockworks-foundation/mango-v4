#![allow(unused_assignments)]
use super::*;

#[tokio::test]
async fn test_collateral_fees() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // fund the vaults to allow borrowing
    create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        1_000_000,
        0,
    )
    .await;

    let account = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        &mints[0..1],
        1_500, // maint: 0.8 * 1500 = 1200
        0,
    )
    .await;

    let empty_account = create_funded_account(
        &solana,
        group,
        owner,
        2,
        &context.users[1],
        &mints[0..0],
        0,
        0,
    )
    .await;

    let hour = 60 * 60;

    send_tx(
        solana,
        GroupEdit {
            group,
            admin,
            options: mango_v4::instruction::GroupEdit {
                collateral_fee_interval_opt: Some(6 * hour),
                ..group_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: mints[0].pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                collateral_fee_per_day_opt: Some(0.1),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: mints[1].pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                loan_origination_fee_rate_opt: Some(0.0),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    //
    // TEST: It works on empty accounts
    //

    send_tx(
        solana,
        TokenChargeCollateralFeesInstruction {
            account: empty_account,
        },
    )
    .await
    .unwrap();
    let mut last_time = solana.clock_timestamp().await;
    solana.set_clock_timestamp(last_time + 9 * hour).await;

    // send it twice, because the first time will never charge anything
    send_tx(
        solana,
        TokenChargeCollateralFeesInstruction {
            account: empty_account,
        },
    )
    .await
    .unwrap();
    last_time = solana.clock_timestamp().await;

    //
    // TEST: Without borrows, charging collateral fees has no effect
    //

    send_tx(solana, TokenChargeCollateralFeesInstruction { account })
        .await
        .unwrap();
    last_time = solana.clock_timestamp().await;
    solana.set_clock_timestamp(last_time + 9 * hour).await;

    // send it twice, because the first time will never charge anything
    send_tx(solana, TokenChargeCollateralFeesInstruction { account })
        .await
        .unwrap();
    last_time = solana.clock_timestamp().await;

    // no effect
    assert_eq!(
        account_position(solana, account, tokens[0].bank).await,
        1_500
    );

    //
    // TEST: With borrows, there's an effect depending on the time that has passed
    //

    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 500, // maint: -1.2 * 500 = -600 (half of 1200)
            allow_borrow: true,
            account,
            owner,
            token_account: context.users[1].token_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    solana.set_clock_timestamp(last_time + 9 * hour).await;

    send_tx(solana, TokenChargeCollateralFeesInstruction { account })
        .await
        .unwrap();
    last_time = solana.clock_timestamp().await;
    assert!(assert_equal_f64_f64(
        account_position_f64(solana, account, tokens[0].bank).await,
        1500.0 * (1.0 - 0.1 * (9.0 / 24.0) * (600.0 / 1200.0)),
        0.01
    ));
    let last_balance = account_position_f64(solana, account, tokens[0].bank).await;

    //
    // TEST: More borrows
    //

    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 100, // maint: -1.2 * 600 = -720
            allow_borrow: true,
            account,
            owner,
            token_account: context.users[1].token_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    solana.set_clock_timestamp(last_time + 7 * hour).await;

    send_tx(solana, TokenChargeCollateralFeesInstruction { account })
        .await
        .unwrap();
    //last_time = solana.clock_timestamp().await;
    assert!(assert_equal_f64_f64(
        account_position_f64(solana, account, tokens[0].bank).await,
        last_balance * (1.0 - 0.1 * (7.0 / 24.0) * (720.0 / (last_balance * 0.8))),
        0.01
    ));

    Ok(())
}
