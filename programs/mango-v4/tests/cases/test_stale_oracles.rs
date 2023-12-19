use std::{path::PathBuf, str::FromStr};

use super::*;
use anchor_lang::prelude::AccountMeta;
use solana_sdk::account::AccountSharedData;

#[tokio::test]
async fn test_stale_oracle_deposit_withdraw() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000); // bad oracles log a lot
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..3];
    let payer_token_accounts = &context.users[1].token_accounts[0..3];

    //
    // SETUP: Create a group, account, register tokens
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // fill vaults, so we can borrow
    let _vault_account = create_funded_account(
        &solana,
        group,
        owner,
        2,
        &context.users[1],
        mints,
        100000,
        0,
    )
    .await;

    // Create account with token0 deposits
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints[0..1],
        100,
        0,
    )
    .await;

    // Create some token1 borrows
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 10,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_token_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Make oracles invalid by increasing deviation
    send_tx(
        solana,
        StubOracleSetTestInstruction {
            oracle: tokens[0].oracle,
            group,
            mint: mints[0].pubkey,
            admin,
            price: 1.0,
            last_update_slot: 0,
            deviation: 100.0,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        StubOracleSetTestInstruction {
            oracle: tokens[1].oracle,
            group,
            mint: mints[1].pubkey,
            admin,
            price: 1.0,
            last_update_slot: 0,
            deviation: 100.0,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        StubOracleSetTestInstruction {
            oracle: tokens[2].oracle,
            group,
            mint: mints[2].pubkey,
            admin,
            price: 1.0,
            last_update_slot: 0,
            deviation: 100.0,
        },
    )
    .await
    .unwrap();

    // Can't activate a token position for a bad oracle
    assert!(send_tx(
        solana,
        TokenDepositInstruction {
            amount: 11,
            reduce_only: false,
            account,
            owner,
            token_account: payer_token_accounts[2],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .is_err());

    // Verify that creating a new borrow won't work
    assert!(send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_token_accounts[2],
            bank_index: 0,
        },
    )
    .await
    .is_err());

    // Repay token1 borrows
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 11,
            reduce_only: true,
            account,
            owner,
            token_account: payer_token_accounts[1],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Withdraw token0 deposits
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 100,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_token_accounts[0],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    Ok(())
}

#[tokio::test]
async fn test_fallback_oracle_withdraw() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000); // bad oracles log a lot
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let fallback_oracle_kp = TestKeypair::new();
    let fallback_oracle = fallback_oracle_kp.pubkey();
    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..3];
    let payer_token_accounts = &context.users[1].token_accounts[0..3];

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // setup fallback_oracle
    send_tx(
        solana,
        StubOracleCreate {
            oracle: fallback_oracle_kp,
            group,
            mint: mints[2].pubkey,
            admin,
            payer,
        },
    )
    .await
    .unwrap();

    // add a fallback oracle
    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: mints[2].pubkey,
            fallback_oracle,
            options: mango_v4::instruction::TokenEdit {
                set_fallback_oracle: true,
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    let bank_data: Bank = solana.get_account(tokens[2].bank).await;
    assert!(bank_data.fallback_oracle == fallback_oracle);

    // fill vaults, so we can borrow
    let _vault_account = create_funded_account(
        &solana,
        group,
        owner,
        2,
        &context.users[1],
        mints,
        100_000,
        0,
    )
    .await;

    // Create account with token3 of deposits
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &[mints[2]],
        1_000_000,
        0,
    )
    .await;

    // Create some token1 borrows
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_token_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Make oracle invalid by increasing deviation
    send_tx(
        solana,
        StubOracleSetTestInstruction {
            oracle: tokens[2].oracle,
            group,
            mint: mints[2].pubkey,
            admin,
            price: 1.0,
            last_update_slot: 0,
            deviation: 100.0,
        },
    )
    .await
    .unwrap();

    let token_withdraw_ix = TokenWithdrawInstruction {
        amount: 1,
        allow_borrow: true,
        account,
        owner,
        token_account: payer_token_accounts[2],
        bank_index: 0,
    };

    // Verify that withdrawing collateral won't work
    assert!(send_tx(solana, token_withdraw_ix.clone(),).await.is_err());

    // now send txn with a fallback oracle in the remaining accounts
    let fallback_oracle_meta = AccountMeta {
        pubkey: fallback_oracle,
        is_writable: false,
        is_signer: false,
    };
    send_tx_with_extra_accounts(solana, token_withdraw_ix, vec![fallback_oracle_meta])
        .await
        .unwrap();

    Ok(())
}

#[tokio::test]
async fn test_clmm_fallback_oracle() -> Result<(), TransportError> {
    // add ability to find fixtures
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("resources/test");

    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000); // bad oracles log a lot
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let fixtures = vec![
        (
            "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d",
            "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
        ),
        (
            "Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD",
            "FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH",
        ),
    ];

    let fallback_oracle = Pubkey::from_str(fixtures[0].0).unwrap();
    let pyth_usd_oracle = Pubkey::from_str(fixtures[1].0).unwrap();

    // setup pyth and clmm accounts
    for fixture in fixtures {
        let filename = format!("resources/test/{}.bin", fixture.0);
        let data = read_file(find_file(&filename).unwrap());
        let mut account =
            AccountSharedData::new(u64::MAX, data.len(), &Pubkey::from_str(fixture.1).unwrap());
        account.set_data(data);
        let mut program_test_context = solana.context.borrow_mut();
        program_test_context.set_account(&Pubkey::from_str(fixture.0).unwrap(), &account);
    }

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..3];
    let payer_token_accounts = &context.users[1].token_accounts[0..3];

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // add a fallback oracle
    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: mints[2].pubkey,
            fallback_oracle,
            options: mango_v4::instruction::TokenEdit {
                set_fallback_oracle: true,
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    let bank_data: Bank = solana.get_account(tokens[2].bank).await;
    assert!(bank_data.fallback_oracle == fallback_oracle);

    // fill vaults, so we can borrow
    let _vault_account = create_funded_account(
        &solana,
        group,
        owner,
        2,
        &context.users[1],
        mints,
        100_000,
        0,
    )
    .await;

    // Create account with token3 of deposits
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &[mints[2]],
        10_000,
        0,
    )
    .await;

    // Adjust oracle prices to match CLMM
    for i in 0..3 {
        send_tx(
            solana,
            StubOracleSetTestInstruction {
                oracle: tokens[i].oracle,
                group,
                mint: mints[i].pubkey,
                admin,
                price: 0.06300727055072872,
                last_update_slot: 0,
                deviation: 0.0,
            },
        )
        .await
        .unwrap();
    }

    // Create some token1 borrows
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 100,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_token_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Make oracle invalid by increasing deviation
    send_tx(
        solana,
        StubOracleSetTestInstruction {
            oracle: tokens[2].oracle,
            group,
            mint: mints[2].pubkey,
            admin,
            price: 0.06300727055072872,
            last_update_slot: 0,
            deviation: 100.0,
        },
    )
    .await
    .unwrap();

    let token_withdraw_ix = TokenWithdrawInstruction {
        amount: 1,
        allow_borrow: true,
        account,
        owner,
        token_account: payer_token_accounts[2],
        bank_index: 0,
    };

    // Verify that withdrawing collateral won't work
    assert!(send_tx(solana, token_withdraw_ix.clone()).await.is_err());

    // Send txn with a fallback oracle in the remaining accounts, but no pyth USD feed
    let fallback_oracle_meta = AccountMeta {
        pubkey: fallback_oracle,
        is_writable: false,
        is_signer: false,
    };
    assert!(send_tx_with_extra_accounts(
        solana,
        token_withdraw_ix.clone(),
        vec![fallback_oracle_meta.clone()]
    )
    .await
    .unwrap()
    .result
    .is_err());

    // Finally send txn with a fallback oracle and pyth USD feed
    let pyth_usd_oracle_meta = AccountMeta {
        pubkey: pyth_usd_oracle,
        is_writable: false,
        is_signer: false,
    };
    send_tx_with_extra_accounts(
        solana,
        token_withdraw_ix,
        vec![fallback_oracle_meta, pyth_usd_oracle_meta],
    )
    .await
    .unwrap()
    .result
    .unwrap();

    Ok(())
}
