#![cfg(all(feature = "test-bpf"))]

use mango_setup::*;
use program_test::*;
use solana_program_test::*;
use solana_sdk::transport::TransportError;

mod program_test;

#[tokio::test]
async fn test_bank_utilization_based_borrow_limit() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..=2];

    let initial_token_deposit = 10_000;

    //
    // SETUP: Create a group and an account
    //

    let GroupWithTokens { group, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let account_0 = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 0,
            token_count: 2,
            serum3_count: 0,
            perp_count: 0,
            perp_oo_count: 0,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;

    let account_1 = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 1,
            token_count: 2,
            serum3_count: 0,
            perp_count: 0,
            perp_oo_count: 0,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;

    //
    // SETUP: Deposit user funds
    //
    {
        let deposit_amount = initial_token_deposit;

        // account_0 deposits mint_0
        send_tx(
            solana,
            TokenDepositInstruction {
                amount: deposit_amount,
                account: account_0,
                owner,
                token_account: payer_mint_accounts[0],
                token_authority: payer,
                bank_index: 0,
            },
        )
        .await
        .unwrap();
        solana.advance_clock().await;

        // account_1 deposits mint_1
        send_tx(
            solana,
            TokenDepositInstruction {
                amount: deposit_amount * 10,
                account: account_1,
                owner,
                token_account: payer_mint_accounts[1],
                token_authority: payer,
                bank_index: 1,
            },
        )
        .await
        .unwrap();
        solana.advance_clock().await;
    }

    {
        let deposit_amount = initial_token_deposit;

        // account_1 tries to borrow all existing deposits on mint_0
        // should fail because borrow limit would be reached
        let res = send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: deposit_amount,
                allow_borrow: true,
                account: account_1,
                owner,
                token_account: payer_mint_accounts[0],
                bank_index: 0,
            },
        )
        .await;
        assert!(res.is_err());
        solana.advance_clock().await;

        // account_1 tries to borrow < limit on mint_0
        // should succeed because borrow limit won't be reached
        send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: deposit_amount / 10 * 7,
                allow_borrow: true,
                account: account_1,
                owner,
                token_account: payer_mint_accounts[0],
                bank_index: 0,
            },
        )
        .await
        .unwrap();
        solana.advance_clock().await;

        // account_0 tries to withdraw all remaining on mint_0
        // should succeed because withdraws without borrows are not limited
        send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: deposit_amount / 10 * 3,
                allow_borrow: false,
                account: account_0,
                owner,
                token_account: payer_mint_accounts[0],
                bank_index: 0,
            },
        )
        .await
        .unwrap();
    }

    Ok(())
}

#[tokio::test]
async fn test_bank_net_borrows_based_borrow_limit() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..=2];

    //
    // SETUP: Create a group and an account
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let account_0 = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 0,
            token_count: 2,
            serum3_count: 0,
            perp_count: 0,
            perp_oo_count: 0,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;

    let account_1 = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 1,
            token_count: 2,
            serum3_count: 0,
            perp_count: 0,
            perp_oo_count: 0,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;

    {
        send_tx(
            solana,
            TokenEditNetBorrows {
                group,
                admin,
                mint: tokens[0].mint.pubkey,
                // we want to test net borrow limits in isolation
                min_vault_to_deposits_ratio_opt: Some(0.0),
                net_borrows_limit_native_opt: Some(6000),
                net_borrows_window_size_ts_opt: Some(3),
            },
        )
        .await
        .unwrap();
    }

    //
    // SETUP: Deposit user funds
    //
    {
        // account_0 deposits mint_0
        send_tx(
            solana,
            TokenDepositInstruction {
                amount: 10_000,
                account: account_0,
                owner,
                token_account: payer_mint_accounts[0],
                token_authority: payer,
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        // account_1 deposits mint_1
        send_tx(
            solana,
            TokenDepositInstruction {
                amount: 10_000 * 10,
                account: account_1,
                owner,
                token_account: payer_mint_accounts[1],
                token_authority: payer,
                bank_index: 1,
            },
        )
        .await
        .unwrap();
    }

    // We elapse at least 3 seconds, so that next block is in new window
    solana.advance_clock().await;
    solana.advance_clock().await;
    solana.advance_clock().await;

    {
        // succeeds because borrow is less than net borrow limit
        send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: 5000,
                allow_borrow: true,
                account: account_1,
                owner,
                token_account: payer_mint_accounts[0],
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        // fails because borrow is greater than remaining margin in net borrow limit
        let res = send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: 4000,
                allow_borrow: true,
                account: account_1,
                owner,
                token_account: payer_mint_accounts[0],
                bank_index: 0,
            },
        )
        .await;
        assert!(res.is_err());

        // succeeds because is not a borrow
        send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: 4000,
                allow_borrow: false,
                account: account_0,
                owner,
                token_account: payer_mint_accounts[0],
                bank_index: 0,
            },
        )
        .await
        .unwrap();
    }

    // We elapse at least 3 seconds, so that next block is in new window
    solana.advance_clock().await;
    solana.advance_clock().await;
    solana.advance_clock().await;

    // succeeds because borrow is less than net borrow limit in a fresh window
    {
        send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: 1000,
                allow_borrow: true,
                account: account_1,
                owner,
                token_account: payer_mint_accounts[0],
                bank_index: 0,
            },
        )
        .await
        .unwrap();
        solana.advance_clock().await;
    }

    Ok(())
}
