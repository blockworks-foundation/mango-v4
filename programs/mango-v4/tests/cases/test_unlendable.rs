use super::*;

#[tokio::test]
async fn test_unlendable() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint0_account = context.users[1].token_accounts[0];

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
    let bank = tokens[0].bank;

    let account = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 0,
            token_count: 6,
            serum3_count: 3,
            perp_count: 3,
            perp_oo_count: 3,
            token_conditional_swap_count: 3,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;

    //
    // TEST: opening and closing
    //

    send_tx(
        solana,
        TokenCreatePositionInstruction {
            account,
            owner,
            bank,
            allow_lending: true,
        },
    )
    .await
    .unwrap();

    send_tx_expect_error!(
        solana,
        TokenCreatePositionInstruction {
            account,
            owner,
            bank,
            allow_lending: false,
        },
        MangoError::TokenPositionWithDifferentSettingAlreadyExists,
    );

    send_tx(
        solana,
        TokenClosePositionInstruction {
            account,
            owner,
            bank,
        },
    )
    .await
    .unwrap();

    send_tx_expect_error!(
        solana,
        TokenClosePositionInstruction {
            account,
            owner,
            bank,
        },
        MangoError::TokenPositionDoesNotExist,
    );

    send_tx(
        solana,
        TokenCreatePositionInstruction {
            account,
            owner,
            bank,
            allow_lending: false,
        },
    )
    .await
    .unwrap();

    send_tx_expect_error!(
        solana,
        TokenCreatePositionInstruction {
            account,
            owner,
            bank,
            allow_lending: true,
        },
        MangoError::TokenPositionWithDifferentSettingAlreadyExists,
    );

    //
    // TEST: Deposit and withdraw
    //

    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 100,
            reduce_only: false,
            account,
            owner,
            token_account: payer_mint0_account,
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();
    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens[0].unlendable_deposits, 100);

    // provides health
    let maint_health = account_maint_health(solana, account).await;
    assert_eq_f64!(maint_health, 80.0, 1e-4);

    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 50,
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint0_account,
            bank_index: 0,
        },
    )
    .await
    .unwrap();
    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens[0].unlendable_deposits, 50);

    send_tx_expect_error!(
        solana,
        TokenWithdrawInstruction {
            amount: 51,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_mint0_account,
            bank_index: 0,
        },
        MangoError::UnlendableTokenPositionCannotBeNegative
    );

    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 50,
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint0_account,
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens[0].unlendable_deposits, 0);

    // not auto-closed
    assert!(account_data.tokens[0].is_active());

    Ok(())
}
