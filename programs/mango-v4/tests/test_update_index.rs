#![cfg(feature = "test-bpf")]

use mango_v4::state::Bank;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use program_test::*;

mod program_test;

#[tokio::test]
async fn test_update_index() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..2];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let mango_setup::GroupWithTokens { group, tokens } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;

    // deposit some funds, to the vaults aren't empty
    let deposit_account = send_tx(
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
    for &token_account in payer_mint_accounts {
        send_tx(
            solana,
            TokenDepositInstruction {
                amount: 10000,
                account: deposit_account,
                token_account,
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    let withdraw_account = send_tx(
        solana,
        CreateAccountInstruction {
            account_num: 1,
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
        TokenDepositInstruction {
            amount: 100000,
            account: withdraw_account,
            token_account: payer_mint_accounts[1],
            token_authority: payer,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 5000,
            allow_borrow: true,
            account: withdraw_account,
            owner,
            token_account: context.users[0].token_accounts[0],
        },
    )
    .await
    .unwrap();

    let bank_before_update_index = solana.get_account::<Bank>(tokens[0].bank).await;

    solana.advance_clock().await;

    send_tx(
        solana,
        UpdateIndexInstruction {
            mint_info: tokens[0].mint_info,
            bank: tokens[0].bank,
        },
    )
    .await
    .unwrap();

    let bank_after_update_index = solana.get_account::<Bank>(tokens[0].bank).await;
    dbg!(bank_after_update_index);
    dbg!(bank_after_update_index);
    assert!(bank_before_update_index.deposit_index < bank_after_update_index.deposit_index);
    assert!(bank_before_update_index.borrow_index < bank_after_update_index.borrow_index);

    Ok(())
}
