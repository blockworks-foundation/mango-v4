#![cfg(feature = "test-bpf")]

use anchor_lang::InstructionData;
use fixed::types::I80F48;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::Signer;
use solana_sdk::{signature::Keypair, transport::TransportError};
use std::str::FromStr;

use mango_v4::state::*;
use program_test::*;

mod program_test;

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_basic() -> Result<(), TransportError> {
    let margin_trade_program_id =
        Pubkey::from_str("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix").unwrap();
    let margin_trade_token_account = Keypair::new();
    let (mtta_owner, mtta_bump_seeds) =
        Pubkey::find_program_address(&[b"margintrade"], &margin_trade_program_id);
    let context = TestContext::new(
        Option::None,
        Some(&margin_trade_program_id),
        Some(&margin_trade_token_account),
        Some(&mtta_owner),
    )
    .await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mint0 = &context.mints[0];
    let payer_mint0_account = context.users[1].token_accounts[0];
    let dust_threshold = 0.01;

    //
    // SETUP: Create a group, account, register a token (mint0)
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
    let _oracle = create_stub_oracle_accounts.oracle;

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
    let bank = register_token_accounts.bank;
    let vault = register_token_accounts.vault;

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

        assert_eq!(solana.token_account_balance(vault).await, deposit_amount);
        assert_eq!(
            solana.token_account_balance(payer_mint0_account).await,
            start_balance - deposit_amount
        );
        let account_data: MangoAccount = solana.get_account(account).await;
        let bank_data: TokenBank = solana.get_account(bank).await;
        assert!(
            account_data.indexed_positions.values[0].native(&bank_data)
                - I80F48::from_num(deposit_amount)
                < dust_threshold
        );
        assert!(
            bank_data.native_total_deposits() - I80F48::from_num(deposit_amount) < dust_threshold
        );
    }

    //
    // TEST: Margin trade
    //
    {
        send_tx(
            solana,
            MarginTradeInstruction {
                account,
                owner,
                mango_token_vault: vault,
                mango_group: group,
                margin_trade_program_id,
                loan_token_account: margin_trade_token_account.pubkey(),
                loan_token_account_owner: mtta_owner,
                margin_trade_program_ix_cpi_data: {
                    let ix = margin_trade::instruction::MarginTrade {
                        amount_from: 2,
                        amount_to: 1,
                        loan_token_account_owner_bump_seeds: mtta_bump_seeds,
                    };
                    ix.data()
                },
            },
        )
        .await
        .unwrap();
    }
    let margin_trade_loan = 1;

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

        assert_eq!(
            solana.token_account_balance(vault).await,
            withdraw_amount - margin_trade_loan
        );
        assert_eq!(
            solana.token_account_balance(payer_mint0_account).await,
            start_balance + withdraw_amount
        );
        let account_data: MangoAccount = solana.get_account(account).await;
        let bank_data: TokenBank = solana.get_account(bank).await;
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
