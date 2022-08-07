#![cfg(feature = "test-bpf")]

use mango_v4::state::*;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use program_test::*;

mod program_test;

#[tokio::test]
async fn test_token_update_index_and_rate() -> Result<(), TransportError> {
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

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;

    // deposit some funds, to the vaults aren't empty
    let deposit_account = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 0,
            token_count: 16,
            serum3_count: 8,
            perp_count: 8,
            perp_oo_count: 8,
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
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();
    }

    let withdraw_account = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 1,
            token_count: 16,
            serum3_count: 8,
            perp_count: 8,
            perp_oo_count: 8,
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
            token_authority: payer.clone(),
            bank_index: 0,
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
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    let bank_before_update_index_and_rate = solana.get_account::<Bank>(tokens[0].bank).await;

    solana.advance_clock().await;

    send_tx(
        solana,
        TokenUpdateIndexAndRateInstruction {
            mint_info: tokens[0].mint_info,
        },
    )
    .await
    .unwrap();

    let bank_after_update_index_and_rate = solana.get_account::<Bank>(tokens[0].bank).await;
    dbg!(bank_after_update_index_and_rate);
    dbg!(bank_after_update_index_and_rate);
    assert!(
        bank_before_update_index_and_rate.deposit_index
            < bank_after_update_index_and_rate.deposit_index
    );
    assert!(
        bank_before_update_index_and_rate.borrow_index
            < bank_after_update_index_and_rate.borrow_index
    );

    Ok(())
}
