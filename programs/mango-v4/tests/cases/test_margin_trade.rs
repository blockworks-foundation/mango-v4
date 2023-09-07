use super::*;

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_margin_trade() -> Result<(), BanksClientError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(100_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint0_account = context.users[1].token_accounts[0];
    let loan_origination_fee = 0.0005;

    // higher resolution that the loan_origination_fee for one token
    let balance_f64eq = |a: f64, b: f64| utils::assert_equal_f64_f64(a, b, 0.0001);

    //
    // SETUP: Create a group, account, register a token (mint0)
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let bank = tokens[0].bank;
    let vault = tokens[0].vault;

    //
    // provide some funds for tokens, so the test user can borrow
    //
    let provided_amount = 1000;
    create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        provided_amount,
        0,
    )
    .await;

    //
    // create thes test user account
    //

    let account = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 0,
            group,
            owner,
            payer,
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .account;

    //
    // TEST: Deposit funds
    //
    let deposit_amount_initial = 100;
    {
        let start_balance = solana.token_account_balance(payer_mint0_account).await;

        send_tx(
            solana,
            TokenDepositInstruction {
                amount: deposit_amount_initial,
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

        assert_eq!(
            solana.token_account_balance(vault).await,
            provided_amount + deposit_amount_initial
        );
        assert_eq!(
            solana.token_account_balance(payer_mint0_account).await,
            start_balance - deposit_amount_initial
        );
        assert_eq!(
            account_position(solana, account, bank).await,
            deposit_amount_initial as i64,
        );
    }

    //
    // TEST: Margin trade
    //
    let margin_account = payer_mint0_account;
    let margin_account_initial = solana.token_account_balance(margin_account).await;
    let target_token_account = context.users[0].token_accounts[0];
    let withdraw_amount = 2;
    let deposit_amount = 1;
    let send_flash_loan_tx = |solana, withdraw_amount, deposit_amount| async move {
        let mut tx = ClientTransaction::new(solana);
        let loans = vec![FlashLoanPart {
            bank,
            token_account: target_token_account,
            withdraw_amount,
        }];
        tx.add_instruction(FlashLoanBeginInstruction {
            account,
            owner,
            loans: loans.clone(),
        })
        .await;
        if withdraw_amount > 0 {
            tx.add_instruction_direct(
                spl_token::instruction::transfer(
                    &spl_token::ID,
                    &target_token_account,
                    &margin_account,
                    &owner.pubkey(),
                    &[&owner.pubkey()],
                    withdraw_amount,
                )
                .unwrap(),
            );
        }
        if deposit_amount > 0 {
            tx.add_instruction_direct(
                spl_token::instruction::transfer(
                    &spl_token::ID,
                    &margin_account,
                    &target_token_account,
                    &payer.pubkey(),
                    &[&payer.pubkey()],
                    deposit_amount,
                )
                .unwrap(),
            );
            tx.add_signer(payer);
        }
        tx.add_instruction(FlashLoanEndInstruction {
            account,
            owner,
            loans,
            // the test only accesses a single token: not a swap
            flash_loan_type: mango_v4::accounts_ix::FlashLoanType::Unknown,
        })
        .await;
        tx.send().await.unwrap();
    };
    send_flash_loan_tx(solana, withdraw_amount, deposit_amount).await;

    assert_eq!(
        solana.token_account_balance(vault).await,
        provided_amount + deposit_amount_initial - withdraw_amount + deposit_amount
    );
    assert_eq!(
        solana.token_account_balance(margin_account).await,
        margin_account_initial + withdraw_amount - deposit_amount
    );
    // no fee because user had positive balance
    assert!(balance_f64eq(
        account_position_f64(solana, account, bank).await,
        (deposit_amount_initial - withdraw_amount + deposit_amount) as f64
    ));

    //
    // TEST: Bringing the balance to 0 deactivates the token
    //
    let deposit_amount_initial = account_position(solana, account, bank).await;
    let margin_account_initial = solana.token_account_balance(margin_account).await;
    let withdraw_amount = deposit_amount_initial as u64;
    let deposit_amount = 0;
    send_flash_loan_tx(solana, withdraw_amount, deposit_amount).await;
    assert_eq!(solana.token_account_balance(vault).await, provided_amount);
    assert_eq!(
        solana.token_account_balance(margin_account).await,
        margin_account_initial + withdraw_amount
    );
    // Check that position is fully deactivated
    let account_data = get_mango_account(solana, account).await;
    assert_eq!(account_data.active_token_positions().count(), 0);

    //
    // TEST: Activating a token via margin trade
    //
    let margin_account_initial = solana.token_account_balance(margin_account).await;
    let withdraw_amount = 0;
    let deposit_amount = 100;
    send_flash_loan_tx(solana, withdraw_amount, deposit_amount).await;
    assert_eq!(
        solana.token_account_balance(vault).await,
        provided_amount + deposit_amount
    );
    assert_eq!(
        solana.token_account_balance(margin_account).await,
        margin_account_initial - deposit_amount
    );
    assert!(balance_f64eq(
        account_position_f64(solana, account, bank).await,
        deposit_amount as f64
    ));

    //
    // TEST: Try loan fees by withdrawing more than the user balance
    //
    let margin_account_initial = solana.token_account_balance(margin_account).await;
    let deposit_amount_initial = account_position(solana, account, bank).await as u64;
    let withdraw_amount = 500;
    let deposit_amount = 450;
    println!("{}", deposit_amount_initial);
    send_flash_loan_tx(solana, withdraw_amount, deposit_amount).await;
    assert_eq!(
        solana.token_account_balance(vault).await,
        provided_amount + deposit_amount_initial + deposit_amount - withdraw_amount
    );
    assert_eq!(
        solana.token_account_balance(margin_account).await,
        margin_account_initial + withdraw_amount - deposit_amount
    );
    assert!(balance_f64eq(
        account_position_f64(solana, account, bank).await,
        (deposit_amount_initial + deposit_amount - withdraw_amount) as f64
            - (withdraw_amount - deposit_amount_initial) as f64 * loan_origination_fee
    ));

    Ok(())
}

#[tokio::test]
async fn test_flash_loan_deposit_fee() -> Result<(), BanksClientError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(100_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let owner_accounts = context.users[0].token_accounts.clone();
    let payer_accounts = context.users[1].token_accounts.clone();

    // higher resolution that the loan_origination_fee for one token
    let balance_f64eq = |a: f64, b: f64| utils::assert_equal_f64_f64(a, b, 0.0001);

    //
    // SETUP: Create a group, account, register a token (mint0)
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let deposit_fee_rate = 0.042f64;
    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: tokens[1].mint.pubkey,
            options: mango_v4::instruction::TokenEdit {
                flash_loan_deposit_fee_rate_opt: Some(deposit_fee_rate as f32),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    //
    // provide some funds for tokens, so the test user can borrow
    //
    let provided_amount = 10000;
    create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        provided_amount,
        0,
    )
    .await;

    //
    // create thes test user account
    //

    let initial_deposit = 5000;
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints[0..1],
        initial_deposit,
        0,
    )
    .await;

    //
    // TEST: flash loan swap
    //
    let initial_owner_balance0 = solana.token_account_balance(owner_accounts[0]).await;
    let initial_owner_balance1 = solana.token_account_balance(owner_accounts[1]).await;
    let initial_payer_balance0 = solana.token_account_balance(payer_accounts[0]).await;
    let initial_payer_balance1 = solana.token_account_balance(payer_accounts[1]).await;

    let withdraw_amount = 1000;
    let deposit_amount = 1000;
    {
        let mut tx = ClientTransaction::new(solana);
        let loans = vec![
            FlashLoanPart {
                bank: tokens[0].bank,
                token_account: owner_accounts[0],
                withdraw_amount,
            },
            FlashLoanPart {
                bank: tokens[1].bank,
                token_account: owner_accounts[1],
                withdraw_amount: 0,
            },
        ];
        tx.add_instruction(FlashLoanBeginInstruction {
            account,
            owner,
            loans: loans.clone(),
        })
        .await;
        if withdraw_amount > 0 {
            tx.add_instruction_direct(
                spl_token::instruction::transfer(
                    &spl_token::ID,
                    &owner_accounts[0],
                    &payer_accounts[0],
                    &owner.pubkey(),
                    &[&owner.pubkey()],
                    withdraw_amount,
                )
                .unwrap(),
            );
        }
        if deposit_amount > 0 {
            tx.add_instruction_direct(
                spl_token::instruction::transfer(
                    &spl_token::ID,
                    &payer_accounts[1],
                    &owner_accounts[1],
                    &payer.pubkey(),
                    &[&payer.pubkey()],
                    deposit_amount,
                )
                .unwrap(),
            );
            tx.add_signer(payer);
        }
        tx.add_instruction(FlashLoanEndInstruction {
            account,
            owner,
            loans,
            flash_loan_type: mango_v4::accounts_ix::FlashLoanType::Swap,
        })
        .await;
        tx.send().await.unwrap();
    }

    let after_owner_balance0 = solana.token_account_balance(owner_accounts[0]).await;
    let after_owner_balance1 = solana.token_account_balance(owner_accounts[1]).await;
    let after_payer_balance0 = solana.token_account_balance(payer_accounts[0]).await;
    let after_payer_balance1 = solana.token_account_balance(payer_accounts[1]).await;

    assert_eq!(after_owner_balance0, initial_owner_balance0);
    assert_eq!(after_owner_balance1, initial_owner_balance1);
    assert_eq!(
        after_payer_balance0,
        initial_payer_balance0 + withdraw_amount
    );
    assert_eq!(
        after_payer_balance1,
        initial_payer_balance1 - deposit_amount
    );

    assert_eq!(
        solana.token_account_balance(tokens[0].vault).await,
        provided_amount + initial_deposit - withdraw_amount
    );
    assert_eq!(
        solana.token_account_balance(tokens[1].vault).await,
        provided_amount + deposit_amount
    );

    let mango_withdraw_amount = account_position_f64(solana, account, tokens[0].bank).await;
    assert!(balance_f64eq(
        mango_withdraw_amount,
        (initial_deposit - withdraw_amount) as f64
    ));

    let mango_deposit_amount = account_position_f64(solana, account, tokens[1].bank).await;
    assert!(balance_f64eq(
        mango_deposit_amount,
        deposit_amount as f64 * (1.0 - deposit_fee_rate)
    ));

    Ok(())
}
