use super::*;

#[tokio::test]
async fn test_delegate() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let delegate = context.users[1].key;
    let mints = &context.mints[0..1];
    let payer_mint0_account = context.users[1].token_accounts[0];

    //
    // SETUP: Create a group, register a token (mint0), create an account
    //

    let GroupWithTokens { group, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let account =
        create_funded_account(&solana, group, owner, 0, &context.users[1], mints, 100, 0).await;

    //
    // TEST: Edit account - Set delegate
    //
    {
        send_tx(
            solana,
            AccountEditInstruction {
                delegate: delegate.pubkey(),
                account_num: 0,
                group,
                owner,
                name: "new_name".to_owned(),
            },
        )
        .await
        .unwrap();
    }

    //
    // TEST: Edit account as delegate - should fail
    //
    {
        let res = send_tx(
            solana,
            AccountEditInstruction {
                delegate: delegate.pubkey(),
                account_num: 0,
                group,
                owner: delegate,
                name: "new_name".to_owned(),
            },
        )
        .await;
        assert!(res.is_err());
    }

    //
    // TEST: Withdraw funds as delegate should fail
    //
    {
        let withdraw_amount = 50;
        let res = send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: withdraw_amount,
                allow_borrow: true,
                account,
                owner: delegate,
                token_account: payer_mint0_account,
                bank_index: 0,
            },
        )
        .await;
        assert!(res.is_err());
    }

    //
    // TEST: Withdrawing a tiny amount as delegate should be ok
    //
    {
        // withdraw most
        send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: 99,
                allow_borrow: false,
                account,
                owner,
                token_account: payer_mint0_account,
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: u64::MAX,
                allow_borrow: false,
                account,
                owner: delegate,
                token_account: context.users[0].token_accounts[0],
                bank_index: 0,
            },
        )
        .await
        .unwrap();
    }

    //
    // TEST: Close account as delegate should fail
    //
    {
        let res = send_tx(
            solana,
            AccountCloseInstruction {
                group,
                account,
                owner: delegate,
                sol_destination: payer.pubkey(),
            },
        )
        .await;
        assert!(res.is_err());
    }

    Ok(())
}
