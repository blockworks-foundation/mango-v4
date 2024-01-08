use super::*;

// Check opening and closing positions
#[tokio::test]
async fn test_position_lifetime() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..3];

    let payer_mint_accounts = &context.users[1].token_accounts[0..=2];

    //
    // SETUP: Create a group and accounts
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

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

    let funding_amount = 1000000;
    create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        funding_amount,
        0,
    )
    .await;

    //
    // TEST: Deposit and withdraw tokens for all mints
    //
    {
        let start_balance = solana.token_account_balance(payer_mint_accounts[0]).await;

        let deposit_amount = 100;

        // cannot deposit_into_existing if no token deposit exists
        assert!(send_tx(
            solana,
            TokenDepositIntoExistingInstruction {
                amount: deposit_amount,
                reduce_only: false,
                account,
                token_account: payer_mint_accounts[0],
                token_authority: payer,
                bank_index: 0,
            }
        )
        .await
        .is_err());

        // this activates the positions
        for &payer_token in payer_mint_accounts {
            send_tx(
                solana,
                TokenDepositInstruction {
                    amount: deposit_amount,
                    reduce_only: false,
                    account,
                    owner,
                    token_account: payer_token,
                    token_authority: payer.clone(),
                    bank_index: 0,
                },
            )
            .await
            .unwrap();
        }

        // now depositing into an active account works
        send_tx(
            solana,
            TokenDepositIntoExistingInstruction {
                amount: deposit_amount,
                reduce_only: false,
                account,
                token_account: payer_mint_accounts[0],
                token_authority: payer,
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        // this closes the positions
        for &payer_token in payer_mint_accounts {
            send_tx(
                solana,
                TokenWithdrawInstruction {
                    amount: u64::MAX,
                    allow_borrow: false,
                    account,
                    owner,
                    token_account: payer_token,
                    bank_index: 0,
                },
            )
            .await
            .unwrap();
        }

        // Check that positions are fully deactivated
        let account = get_mango_account(solana, account).await;
        assert_eq!(account.active_token_positions().count(), 0);

        // No user tokens got lost
        for &payer_token in payer_mint_accounts {
            assert_eq!(
                start_balance,
                solana.token_account_balance(payer_token).await
            );
        }
    }

    //
    // TEST: Activate a position by borrowing, then close the borrow
    //
    {
        let start_balance = solana.token_account_balance(payer_mint_accounts[0]).await;

        // collateral for the incoming borrow
        let collateral_amount = 1000;
        send_tx(
            solana,
            TokenDepositInstruction {
                amount: collateral_amount,
                reduce_only: false,
                account,
                owner,
                token_account: payer_mint_accounts[0],
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        // borrow some of mint1, activating the position
        let borrow_amount = 10;
        send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: borrow_amount,
                allow_borrow: true,
                account,
                owner,
                token_account: payer_mint_accounts[1],
                bank_index: 0,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            account_position(solana, account, tokens[1].bank).await,
            -(borrow_amount as i64)
        );

        // give it back, closing the position
        {
            send_tx(
                solana,
                TokenDepositInstruction {
                    // deposit withdraw amount + some more to cover loan origination fees
                    amount: borrow_amount + 2,
                    reduce_only: false,
                    account,
                    owner,
                    token_account: payer_mint_accounts[1],
                    token_authority: payer.clone(),
                    bank_index: 0,
                },
            )
            .await
            .unwrap();
            send_tx(
                solana,
                TokenWithdrawInstruction {
                    // withdraw residual amount left
                    amount: u64::MAX,
                    allow_borrow: false,
                    account,
                    owner,
                    token_account: payer_mint_accounts[1],
                    bank_index: 0,
                },
            )
            .await
            .unwrap();
        }

        // withdraw the collateral, closing the position
        send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: collateral_amount,
                allow_borrow: false,
                account,
                owner,
                token_account: payer_mint_accounts[0],
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        // Check that positions are fully deactivated
        let account = get_mango_account(solana, account).await;
        assert_eq!(account.active_token_positions().count(), 0);

        // No user tokens got lost
        // TODO: -1 is a workaround for rounding down in withdraw
        for &payer_token in payer_mint_accounts {
            assert!(start_balance - 1 <= solana.token_account_balance(payer_token).await);
        }
    }

    Ok(())
}
