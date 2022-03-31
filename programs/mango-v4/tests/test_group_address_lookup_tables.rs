// TODO: ALTs are unavailable
#![cfg(all(feature = "test-bpf", feature = "disabled-alt-test"))]

use anchor_lang::prelude::*;
use solana_program_test::*;
use solana_sdk::signature::Keypair;

use mango_v4::address_lookup_table;
use mango_v4::state::*;
use program_test::*;

mod program_test;

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_group_address_lookup_tables() -> Result<()> {
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
    // SETUP: Create a group
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

    //
    // SETUP: Register three mints (and make oracles for them)
    //

    let register_mint = |index: TokenIndex, mint: MintCookie, address_lookup_table: Pubkey| async move {
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
                token_index: index,
                decimals: mint.decimals,
                maint_asset_weight: 0.9,
                init_asset_weight: 0.8,
                maint_liab_weight: 1.1,
                init_liab_weight: 1.2,
                liquidation_fee: 0.0,
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

    // mint0 and mint1
    let address_lookup_table1 = solana.create_address_lookup_table(admin, payer).await;
    // mint2
    solana.advance_by_slots(1).await; // to get a different address
    let address_lookup_table2 = solana.create_address_lookup_table(admin, payer).await;

    let (oracle0, bank0) = register_mint(0, mint0.clone(), address_lookup_table1).await;
    let (oracle1, bank1) = register_mint(1, mint1.clone(), address_lookup_table1).await;
    let (oracle2, bank2) = register_mint(2, mint2.clone(), address_lookup_table2).await;

    // check the resulting address maps
    let data = solana
        .get_account_data(address_lookup_table1)
        .await
        .unwrap();
    assert_eq!(
        address_lookup_table::addresses(&data),
        [bank0, oracle0, bank1, oracle1]
    );

    let data = solana
        .get_account_data(address_lookup_table2)
        .await
        .unwrap();
    assert_eq!(address_lookup_table::addresses(&data), [bank2, oracle2]);

    //
    // TEST: Deposit funds for each token
    //
    {
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
    }

    //
    // TEST: Withdraw funds for each token
    //
    {
        let withdraw_amount = 50;

        for &payer_token in payer_mint_accounts {
            send_tx(
                solana,
                WithdrawInstruction {
                    amount: withdraw_amount,
                    allow_borrow: true,
                    account,
                    owner,
                    token_account: payer_token,
                },
            )
            .await
            .unwrap();
        }
    }

    Ok(())
}
