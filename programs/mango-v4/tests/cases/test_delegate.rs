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

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let bank = tokens[0].bank;

    let account =
        create_funded_account(&solana, group, owner, 0, &context.users[1], mints, 100, 0).await;

    //
    // TEST: Edit account - Set delegate
    //
    {
        mango_client::send_tx(
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
        let res = mango_client::send_tx(
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
        let res = mango_client::send_tx(
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
    // TEST: Close account as delegate should fail
    //
    {
        let bank_data: Bank = solana.get_account(bank).await;
        mango_client::send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: bank_data.native_deposits().to_num(),
                allow_borrow: false,
                account,
                owner,
                token_account: payer_mint0_account,
                bank_index: 0,
            },
        )
        .await
        .unwrap();
        let res = mango_client::send_tx(
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
