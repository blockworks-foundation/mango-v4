#![cfg(feature = "test-bpf")]

use fixed::types::I80F48;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signature::Signer, transport::TransportError};

use mango_v4::state::*;
use program_test::*;

mod program_test;

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_basic() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..1];
    let payer_mint0_account = context.users[1].token_accounts[0];
    let dust_threshold = 0.01;

    //
    // SETUP: Create a group, account, register a token (mint0)
    //

    let mango_setup::GroupWithTokens { group, tokens } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;
    let bank = tokens[0].bank;
    let vault = tokens[0].vault;

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

    //
    // TEST: Deposit funds
    //
    {
        let deposit_amount = 100;
        let start_balance = solana.token_account_balance(payer_mint0_account).await;

        send_tx(
            solana,
            TokenDepositInstruction {
                amount: deposit_amount,
                account,
                token_account: payer_mint0_account,
                token_authority: payer,
            },
        )
        .await
        .unwrap();

        assert_eq!(solana.token_account_balance(vault).await, deposit_amount);
        assert_eq!(
            solana.token_account_balance(payer_mint0_account).await,
            start_balance - deposit_amount
        );
        assert_eq!(
            account_position(solana, account, bank).await,
            deposit_amount as i64
        );
        let bank_data: Bank = solana.get_account(bank).await;
        assert!(
            bank_data.native_total_deposits() - I80F48::from_num(deposit_amount) < dust_threshold
        );
    }

    //
    // TEST: Compute the account health
    //
    send_tx(
        solana,
        ComputeHealthInstruction {
            account,
            health_type: HealthType::Init,
        },
    )
    .await
    .unwrap();

    //
    // TEST: Withdraw funds
    //
    {
        let start_amount = 100;
        let withdraw_amount = 50;
        let start_balance = solana.token_account_balance(payer_mint0_account).await;

        send_tx(
            solana,
            TokenWithdrawInstruction {
                amount: withdraw_amount,
                allow_borrow: true,
                account,
                owner,
                token_account: payer_mint0_account,
            },
        )
        .await
        .unwrap();

        assert_eq!(solana.token_account_balance(vault).await, withdraw_amount);
        assert_eq!(
            solana.token_account_balance(payer_mint0_account).await,
            start_balance + withdraw_amount
        );
        assert_eq!(
            account_position(solana, account, bank).await,
            (start_amount - withdraw_amount) as i64
        );
        let bank_data: Bank = solana.get_account(bank).await;
        assert!(
            bank_data.native_total_deposits() - I80F48::from_num(start_amount - withdraw_amount)
                < dust_threshold
        );
    }

    //
    // TEST: Close account and de register bank
    //

    // withdraw whatever is remaining, can't close bank vault without this
    send_tx(
        solana,
        UpdateIndexInstruction {
            mint_info: tokens[0].mint_info,
            bank,
        },
    )
    .await
    .unwrap();
    let bank_data: Bank = solana.get_account(bank).await;
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: bank_data.native_total_deposits().to_num(),
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint0_account,
        },
    )
    .await
    .unwrap();

    // close account
    send_tx(
        solana,
        CloseAccountInstruction {
            account,
            owner,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    // deregister bank - closes bank, mint info, and bank vault
    let bank_data: Bank = solana.get_account(bank).await;
    send_tx(
        solana,
        TokenDeregisterInstruction {
            admin,
            payer,
            group,
            mint: bank_data.mint,
            token_index: bank_data.token_index,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    // close stub oracle
    send_tx(
        solana,
        CloseStubOracleInstruction {
            group,
            mint: bank_data.mint,
            admin,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    // close group
    send_tx(
        solana,
        CloseGroupInstruction {
            group,
            admin,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    Ok(())
}
