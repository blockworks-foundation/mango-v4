#![cfg(feature = "test-bpf")]

use anchor_lang::prelude::*;
use solana_program_test::*;
use solana_sdk::signature::Keypair;

use mango_v4::state::*;
use program_test::*;

mod program_test;

// Check opening and closing positions
#[tokio::test]
async fn test_position_lifetime() -> Result<()> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mint0 = &context.mints[0];
    let mint1 = &context.mints[1];
    let mint2 = &context.mints[2];

    let payer_mint_accounts = &context.users[1].token_accounts[0..=2];

    //
    // SETUP: Create a group and accounts
    //

    let group = send_tx(solana, CreateGroupInstruction { admin, payer })
        .await
        .unwrap()
        .group;

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

    let funding_account = send_tx(
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

    //
    // SETUP: Register three mints (and make oracles for them)
    //

    let address_lookup_table = solana.create_address_lookup_table(admin, payer).await;

    let register_mint = |mint: MintCookie| async move {
        let create_stub_oracle_accounts = send_tx(
            solana,
            CreateStubOracle {
                mint: mint.pubkey,
                payer,
            },
        )
        .await
        .unwrap();
        let oracle = create_stub_oracle_accounts.oracle;
        send_tx(
            solana,
            SetStubOracle {
                mint: mint.pubkey,
                payer,
                price: "1.0",
            },
        )
        .await
        .unwrap();
        let register_token_accounts = send_tx(
            solana,
            RegisterTokenInstruction {
                decimals: mint.decimals,
                maint_asset_weight: 0.9,
                init_asset_weight: 0.8,
                maint_liab_weight: 1.1,
                init_liab_weight: 1.2,
                group,
                admin,
                mint: mint.pubkey,
                address_lookup_table,
                payer,
            },
        )
        .await
        .unwrap();
        let bank = register_token_accounts.bank;

        (oracle, bank)
    };
    register_mint(mint0.clone()).await;
    register_mint(mint1.clone()).await;
    register_mint(mint2.clone()).await;

    //
    // SETUP: Put some tokens into the funding account to allow borrowing
    //
    {
        let funding_amount = 1000000;
        for &payer_token in payer_mint_accounts {
            send_tx(
                solana,
                DepositInstruction {
                    amount: funding_amount,
                    account: funding_account,
                    token_account: payer_token,
                    token_authority: payer,
                },
            )
            .await
            .unwrap();
        }
    }

    //
    // TEST: Deposit and withdraw tokens for all mints
    //
    {
        let start_balance = solana.token_account_balance(payer_mint_accounts[0]).await;

        // this activates the positions
        let deposit_amount = 100;
        for &payer_token in payer_mint_accounts {
            send_tx(
                solana,
                DepositInstruction {
                    amount: deposit_amount,
                    account,
                    token_account: payer_token,
                    token_authority: payer,
                },
            )
            .await
            .unwrap();
        }

        // this closes the positions
        for &payer_token in payer_mint_accounts {
            send_tx(
                solana,
                WithdrawInstruction {
                    amount: u64::MAX,
                    allow_borrow: false,
                    account,
                    owner,
                    token_account: payer_token,
                },
            )
            .await
            .unwrap();
        }

        // Check that positions are fully deactivated
        let account: MangoAccount = solana.get_account(account).await;
        assert_eq!(account.indexed_positions.iter_active().count(), 0);

        // No user tokens got lost
        for &payer_token in payer_mint_accounts {
            assert_eq!(
                start_balance,
                solana.token_account_balance(payer_token).await
            );
        }
    }

    //
    // TEST: Activate a position by borrowing, then close the borrow
    //
    {
        let start_balance = solana.token_account_balance(payer_mint_accounts[0]).await;

        // collateral for the incoming borrow
        let collateral_amount = 1000;
        send_tx(
            solana,
            DepositInstruction {
                amount: collateral_amount,
                account,
                token_account: payer_mint_accounts[0],
                token_authority: payer,
            },
        )
        .await
        .unwrap();

        // borrow some of mint1, activating the position
        let borrow_amount = 10;
        send_tx(
            solana,
            WithdrawInstruction {
                amount: borrow_amount,
                allow_borrow: true,
                account,
                owner,
                token_account: payer_mint_accounts[1],
            },
        )
        .await
        .unwrap();

        // give it back, closing the position
        send_tx(
            solana,
            DepositInstruction {
                amount: borrow_amount,
                account,
                token_account: payer_mint_accounts[1],
                token_authority: payer,
            },
        )
        .await
        .unwrap();

        // withdraw the collateral, closing the position
        send_tx(
            solana,
            WithdrawInstruction {
                amount: collateral_amount,
                allow_borrow: false,
                account,
                owner,
                token_account: payer_mint_accounts[0],
            },
        )
        .await
        .unwrap();

        // Check that positions are fully deactivated
        let account: MangoAccount = solana.get_account(account).await;
        assert_eq!(account.indexed_positions.iter_active().count(), 0);

        // No user tokens got lost
        for &payer_token in payer_mint_accounts {
            assert_eq!(
                start_balance,
                solana.token_account_balance(payer_token).await
            );
        }
    }

    Ok(())
}
