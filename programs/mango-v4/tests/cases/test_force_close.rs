use super::*;

#[tokio::test]
async fn test_force_close() -> Result<(), TransportError> {
    let test_builder = TestContextBuilder::new();
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..4];
    let payer_mint_accounts = &context.users[1].token_accounts[0..4];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let collateral_token = &tokens[0];
    let borrow_token = &tokens[1];

    // deposit some funds, to the vaults aren't empty
    create_funded_account(
        &solana,
        group,
        owner,
        99,
        &context.users[1],
        mints,
        100000,
        0,
    )
    .await;

    let liqor = send_tx(
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

    let deposit1_amount = 100;
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: deposit1_amount,
            reduce_only: false,
            account: liqor,
            owner,
            token_account: payer_mint_accounts[0],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Make an account with some collateral and some borrows
    //
    let liqee = send_tx(
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

    let deposit1_amount = 100;
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: deposit1_amount,
            reduce_only: false,
            account: liqee,
            owner,
            token_account: payer_mint_accounts[0],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    let borrow1_amount = 10;
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: borrow1_amount,
            allow_borrow: true,
            account: liqee,
            owner,
            token_account: payer_mint_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // test force close is enabled
    //
    assert!(send_tx(
        solana,
        TokenForceCloseBorrowsWithTokenInstruction {
            liqee: liqee,
            liqor: liqor,
            liqor_owner: owner,
            asset_token_index: collateral_token.index,
            liab_token_index: borrow_token.index,
            max_liab_transfer: 10000,
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .is_err());

    // set force close, and reduce only to 1
    send_tx(
        solana,
        TokenMakeReduceOnly {
            admin,
            group,
            mint: mints[1].pubkey,
            reduce_only: 1,
            force_close: false,
        },
    )
    .await
    .unwrap();

    //
    // test liqor needs deposits to be gte than the borrows it wants to liquidate
    //
    assert!(send_tx(
        solana,
        TokenForceCloseBorrowsWithTokenInstruction {
            liqee: liqee,
            liqor: liqor,
            liqor_owner: owner,
            asset_token_index: collateral_token.index,
            liab_token_index: borrow_token.index,
            max_liab_transfer: 10000,
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .is_err());

    //
    // test deposit with reduce only set to 1
    //
    let deposit1_amount = 11;
    assert!(send_tx(
        solana,
        TokenDepositInstruction {
            amount: deposit1_amount,
            reduce_only: false,
            account: liqor,
            owner,
            token_account: payer_mint_accounts[1],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .is_err());

    // set force close, and reduce only to 2
    send_tx(
        solana,
        TokenMakeReduceOnly {
            admin,
            group,
            mint: mints[1].pubkey,
            reduce_only: 2,
            force_close: true,
        },
    )
    .await
    .unwrap();

    //
    // test deposit with reduce only set to 2
    //
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: deposit1_amount,
            reduce_only: false,
            account: liqor,
            owner,
            token_account: payer_mint_accounts[1],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // test force close borrows
    //
    send_tx(
        solana,
        TokenForceCloseBorrowsWithTokenInstruction {
            liqee: liqee,
            liqor: liqor,
            liqor_owner: owner,
            asset_token_index: collateral_token.index,
            liab_token_index: borrow_token.index,
            max_liab_transfer: 10000,
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .unwrap();

    assert!(account_position_closed(solana, liqee, borrow_token.bank).await);
    assert_eq!(
        account_position(solana, liqee, collateral_token.bank).await,
        100 - 10
    );

    Ok(())
}
