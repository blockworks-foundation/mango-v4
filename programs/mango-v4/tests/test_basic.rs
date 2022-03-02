#![cfg(feature = "test-bpf")]

use fixed::types::I80F48;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use mango_v4::address_lookup_table;
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
    let mint0 = &context.mints[0];
    let mint1 = &context.mints[1];
    let payer_mint0_account = context.users[1].token_accounts[0];
    let payer_mint1_account = context.users[1].token_accounts[1];
    let dust_threshold = 0.01;

    //
    // SETUP: Create a group, account, register tokens (mint0, mint1)
    //

    let group = send_tx(solana, CreateGroupInstruction { admin, payer })
        .await
        .unwrap()
        .group;

    let account = send_tx(
        solana,
        CreateAccountInstruction {
            account_num: 0,
            recent_slot: 0, // TODO: get a real recent_slot, probably from SlotHistory
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;

    let create_stub_oracle_accounts = send_tx(
        solana,
        CreateStubOracle {
            mint: mint0.pubkey,
            payer,
        },
    )
    .await
    .unwrap();
    let oracle0 = create_stub_oracle_accounts.oracle;

    send_tx(
        solana,
        SetStubOracle {
            mint: mint0.pubkey,
            payer,
            price: "1.0",
        },
    )
    .await
    .unwrap();

    let register_token_accounts = send_tx(
        solana,
        RegisterTokenInstruction {
            decimals: mint0.decimals,
            maint_asset_weight: 0.9,
            init_asset_weight: 0.8,
            maint_liab_weight: 1.1,
            init_liab_weight: 1.2,
            group,
            admin,
            mint: mint0.pubkey,
            payer,
        },
    )
    .await
    .unwrap();
    let bank0 = register_token_accounts.bank;
    let vault0 = register_token_accounts.vault;

    let create_stub_oracle_accounts = send_tx(
        solana,
        CreateStubOracle {
            mint: mint1.pubkey,
            payer,
        },
    )
    .await
    .unwrap();
    let oracle1 = create_stub_oracle_accounts.oracle;

    send_tx(
        solana,
        SetStubOracle {
            mint: mint1.pubkey,
            payer,
            price: "1.0",
        },
    )
    .await
    .unwrap();

    let register_token_accounts = send_tx(
        solana,
        RegisterTokenInstruction {
            decimals: mint1.decimals,
            maint_asset_weight: 0.9,
            init_asset_weight: 0.8,
            maint_liab_weight: 1.1,
            init_liab_weight: 1.2,
            group,
            admin,
            mint: mint1.pubkey,
            payer,
        },
    )
    .await
    .unwrap();
    let bank1 = register_token_accounts.bank;
    let vault1 = register_token_accounts.vault;

    //
    // TEST: Deposit funds
    //
    {
        let deposit_amount = 100;
        let start_balance = solana.token_account_balance(payer_mint0_account).await;

        send_tx(
            solana,
            DepositInstruction {
                amount: deposit_amount,
                account,
                token_account: payer_mint0_account,
                token_authority: payer,
            },
        )
        .await
        .unwrap();

        let account_data: MangoAccount = solana.get_account(account).await;
        let bank_data: TokenBank = solana.get_account(bank0).await;

        // Check that the deposit happened
        assert_eq!(solana.token_account_balance(vault0).await, deposit_amount);
        assert_eq!(
            solana.token_account_balance(payer_mint0_account).await,
            start_balance - deposit_amount
        );
        assert!(
            account_data.indexed_positions.values[0].native(&bank_data)
                - I80F48::from_num(deposit_amount)
                < dust_threshold
        );
        assert!(
            bank_data.native_total_deposits() - I80F48::from_num(deposit_amount) < dust_threshold
        );

        // Check the lookup table
        let lookup_data = solana
            .get_account_data(account_data.address_lookup_table)
            .await
            .unwrap();
        assert_eq!(
            address_lookup_table::addresses(&lookup_data),
            [bank0, oracle0]
        );
        assert_eq!(
            account_data.address_lookup_table_selection
                [0..account_data.address_lookup_table_selection_size as usize],
            [0, 1]
        );
    }

    {
        let deposit_amount = 100;

        send_tx(
            solana,
            DepositInstruction {
                amount: deposit_amount,
                account,
                token_account: payer_mint1_account,
                token_authority: payer,
            },
        )
        .await
        .unwrap();

        // Check the lookup table
        let account_data: MangoAccount = solana.get_account(account).await;
        let lookup_data = solana
            .get_account_data(account_data.address_lookup_table)
            .await
            .unwrap();
        assert_eq!(
            address_lookup_table::addresses(&lookup_data),
            [bank0, oracle0, bank1, oracle1]
        );
        assert_eq!(
            account_data.address_lookup_table_selection
                [0..account_data.address_lookup_table_selection_size as usize],
            [0, 2, 1, 3]
        );
    }

    //
    // TEST: Withdraw funds
    //
    {
        let withdraw_amount = 50;
        let start_balance = solana.token_account_balance(payer_mint0_account).await;

        send_tx(
            solana,
            WithdrawInstruction {
                amount: withdraw_amount,
                allow_borrow: true,
                account,
                owner,
                token_account: payer_mint0_account,
            },
        )
        .await
        .unwrap();

        assert_eq!(solana.token_account_balance(vault0).await, withdraw_amount);
        assert_eq!(
            solana.token_account_balance(payer_mint0_account).await,
            start_balance + withdraw_amount
        );
        let account_data: MangoAccount = solana.get_account(account).await;
        let bank_data: TokenBank = solana.get_account(bank0).await;
        assert!(
            account_data.indexed_positions.values[0].native(&bank_data)
                - I80F48::from_num(withdraw_amount)
                < dust_threshold
        );
        assert!(
            bank_data.native_total_deposits() - I80F48::from_num(withdraw_amount) < dust_threshold
        );
    }

    Ok(())
}
