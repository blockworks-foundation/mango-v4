#![cfg(feature = "test-bpf")]

use solana_program_test::*;
use solana_sdk::{signature::Keypair, signature::Signer, transport::TransportError};

use mango_v4::state::*;
use program_test::*;

mod program_test;

#[tokio::test]
async fn test_delegate() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let delegate = &context.users[1].key;
    let mints = &context.mints[0..1];
    let payer_mint0_account = context.users[1].token_accounts[0];

    //
    // SETUP: Create a group, register a token (mint0), create an account
    //

    let mango_setup::GroupWithTokens { group, tokens } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;
    let bank = tokens[0].bank;

    let account = send_tx(
        solana,
        CreateAccountInstruction {
            account_num: 0,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;

    // deposit
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 100,
            account,
            token_account: payer_mint0_account,
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // TEST: Edit account - Set delegate
    //
    {
        send_tx(
            solana,
            EditAccountInstruction {
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
            EditAccountInstruction {
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
    // TEST: Close account as delegate should fail
    //
    {
        let bank_data: Bank = solana.get_account(bank).await;
        send_tx(
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
        let res = send_tx(
            solana,
            CloseAccountInstruction {
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
