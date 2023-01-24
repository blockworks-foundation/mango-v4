#![cfg(feature = "test-bpf")]

use fixed::types::I80F48;
use solana_program_test::*;
use solana_sdk::transport::TransportError;

use mango_v4::state::*;
use program_test::*;

mod program_test;

#[tokio::test]
async fn test_ix_gate_set() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..1];
    let payer_mint0_account = context.users[1].token_accounts[0];
    let dust_threshold = 0.01;

    //
    // SETUP: Create a group, account, register a token (mint0)
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let account = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 0,
            token_count: 8,
            serum3_count: 7,
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

    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 10,
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

    let group_data: Group = solana.get_account(group).await;
    assert!(group_data.is_ix_enabled(IxGate::TokenDeposit));

    send_tx(
        solana,
        IxGateSetInstruction {
            group,
            admin,
            ix_gate: {
                let mut ix_gate = 0u128;
                ix_gate |= 1 << IxGate::TokenDeposit as u128;
                ix_gate
            },
        },
    )
    .await
    .unwrap();

    let group_data: Group = solana.get_account(group).await;
    assert!(!group_data.is_ix_enabled(IxGate::TokenDeposit));

    let res = send_tx(
        solana,
        TokenDepositInstruction {
            amount: 10,
            reduce_only: false,
            account,
            owner,
            token_account: payer_mint0_account,
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await;
    assert!(res.is_err());

    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 10,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_mint0_account,
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    Ok(())
}
