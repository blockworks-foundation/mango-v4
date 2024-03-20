use super::*;

#[tokio::test]
async fn test_unlendable_basic() -> Result<(), TransportError> {
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

#[tokio::test]
async fn test_unlendable_liq() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint0_account = context.users[1].token_accounts[0];
    let payer_mint1_account = context.users[1].token_accounts[1];

    //
    // SETUP: Create a group, account, register a token (mint0)
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        zero_token_is_quote: true,
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // Drop loan origination to simplify
    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: tokens[1].mint.pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                loan_origination_fee_rate_opt: Some(0.0),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    // funding for vaults
    create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        1_000_000,
        0,
    )
    .await;

    let liqor = create_funded_account(
        &solana,
        group,
        owner,
        2,
        &context.users[1],
        &mints[0..1],
        1_000_000,
        0,
    )
    .await;

    //
    // SETUP: an account with unlendable deposits backing a borrow
    //
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

    send_tx(
        solana,
        TokenCreatePositionInstruction {
            account,
            owner,
            bank: tokens[0].bank,
            allow_lending: false,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 1000,
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

    set_bank_stub_oracle_price(solana, group, &tokens[1], admin, 0.1).await;
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1000,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_mint1_account,
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // First liquidation until init > -1

    set_bank_stub_oracle_price(solana, group, &tokens[1], admin, 0.9).await;
    send_tx(
        solana,
        TokenLiqWithTokenInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            asset_token_index: tokens[0].index,
            liab_token_index: tokens[1].index,
            asset_bank_index: 0,
            liab_bank_index: 0,
            max_liab_transfer: I80F48::from_num(10000.0),
        },
    )
    .await
    .unwrap();

    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens[0].unlendable_deposits, 1000 - 753);
    assert_eq_f64!(
        account_position_f64(solana, account, tokens[1].bank).await,
        -1000.0 + 752.2 / (0.9 * 1.02 * 1.02),
        0.1
    );
    assert_eq_f64!(account_init_health(solana, account).await, -0.5, 0.5);

    // Second liquidation to bankruptcy

    set_bank_stub_oracle_price(solana, group, &tokens[1], admin, 1.5).await;
    send_tx(
        solana,
        TokenLiqWithTokenInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            asset_token_index: tokens[0].index,
            liab_token_index: tokens[1].index,
            asset_bank_index: 0,
            liab_bank_index: 0,
            max_liab_transfer: I80F48::from_num(10000.0),
        },
    )
    .await
    .unwrap();

    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens[0].unlendable_deposits, 0);
    assert!(account_data.tokens[1].indexed_position < 0);

    Ok(())
}
