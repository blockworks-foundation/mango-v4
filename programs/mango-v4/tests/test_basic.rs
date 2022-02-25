#![cfg(feature = "test-bpf")]
use fixed::types::I80F48;
use mango_v4::state::*;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::instruction::Instruction;
use solana_sdk::{signature::Keypair, signer::Signer, transport::TransportError};

use program_test::*;

mod program_test;

#[tokio::test]
async fn test_basic() -> Result<(), TransportError> {
    let context = TestContext::new().await;

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;

    let group = send_tx(&context.solana, CreateGroupInstruction { admin, payer })
        .await
        .group;

    let account = send_tx(
        &context.solana,
        CreateAccountInstruction {
            account_num: 0,
            group,
            owner,
            payer,
        },
    )
    .await
    .account;

    let register_token_accounts = send_tx(
        &context.solana,
        RegisterTokenInstruction {
            decimals: context.mints[0].decimals,
            group,
            admin,
            mint: context.mints[0].pubkey,
            payer,
        },
    )
    .await;
    let bank = register_token_accounts.bank;
    let vault = register_token_accounts.vault;

    send_tx(
        &context.solana,
        DepositInstruction {
            amount: 100,
            group,
            account,
            deposit_token: context.users[1].token_accounts[0],
            deposit_authority: payer,
        },
    )
    .await;

    assert_eq!(
        context.solana.token_account_balance(vault).await.unwrap(),
        100
    );

    let account_data: MangoAccount = context.solana.get_account(account).await.unwrap();
    let bank_data: TokenBank = context.solana.get_account(bank).await.unwrap();
    assert!(
        account_data.indexed_positions.values[0].native(&bank_data) - I80F48::from_num(100.0) < 0.1
    );
    assert!(bank_data.native_total_deposits() - I80F48::from_num(100.0) < 0.1);

    Ok(())
}
