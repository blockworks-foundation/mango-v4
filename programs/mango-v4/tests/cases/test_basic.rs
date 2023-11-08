use super::*;

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_basic() -> Result<(), TransportError> {
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
    let bank = tokens[0].bank;
    let vault = tokens[0].vault;

    let account = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 0,
            token_count: 6,
            serum3_count: 3,
            perp_count: 3,
            perp_oo_count: 3,
            token_conditional_swap_count: 3,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;
    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens.len(), 6);
    assert_eq!(
        account_data.tokens.iter().filter(|t| t.is_active()).count(),
        0
    );
    assert_eq!(account_data.serum3.len(), 3);
    assert_eq!(
        account_data.serum3.iter().filter(|s| s.is_active()).count(),
        0
    );

    assert_eq!(account_data.perps.len(), 3);
    assert_eq!(account_data.perp_open_orders.len(), 3);

    send_tx(
        solana,
        AccountExpandInstruction {
            account_num: 0,
            token_count: 1,
            serum3_count: 1,
            perp_count: 1,
            perp_oo_count: 1,
            token_conditional_swap_count: 1,
            group,
            owner,
            payer,
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .account;
    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens.len(), 1);
    assert_eq!(
        account_data.tokens.iter().filter(|t| t.is_active()).count(),
        0
    );
    assert_eq!(account_data.serum3.len(), 1);
    assert_eq!(
        account_data.serum3.iter().filter(|s| s.is_active()).count(),
        0
    );
    assert_eq!(account_data.perps.len(), 1);
    assert_eq!(account_data.perp_open_orders.len(), 1);

    send_tx(
        solana,
        AccountExpandInstruction {
            account_num: 0,
            token_count: 8,
            serum3_count: 4,
            perp_count: 4,
            perp_oo_count: 8,
            token_conditional_swap_count: 4,
            group,
            owner,
            payer,
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .account;
    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens.len(), 8);
    assert_eq!(
        account_data.tokens.iter().filter(|t| t.is_active()).count(),
        0
    );
    assert_eq!(account_data.serum3.len(), 4);
    assert_eq!(
        account_data.serum3.iter().filter(|s| s.is_active()).count(),
        0
    );
    assert_eq!(account_data.perps.len(), 4);
    assert_eq!(account_data.perp_open_orders.len(), 8);

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
        assert!(bank_data.native_deposits() - I80F48::from_num(deposit_amount) < dust_threshold);

        let account_data: MangoAccount = solana.get_account(account).await;
        // Assumes oracle price of 1
        assert_eq!(account_data.net_deposits, deposit_amount as i64);
    }

    //
    // TEST: Compute the account health
    //
    assert_eq!(account_init_health(solana, account).await.round(), 60.0);

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
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        check_prev_instruction_post_health(&solana, account).await;

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
            bank_data.native_deposits() - I80F48::from_num(start_amount - withdraw_amount)
                < dust_threshold
        );

        let account_data: MangoAccount = solana.get_account(account).await;
        // Assumes oracle price of 1
        assert_eq!(
            account_data.net_deposits,
            (start_amount - withdraw_amount) as i64
        );
    }

    //
    // TEST: Close account and de register bank
    //

    // withdraw whatever is remaining, can't close bank vault without this
    send_tx(
        solana,
        TokenUpdateIndexAndRateInstruction {
            mint_info: tokens[0].mint_info,
        },
    )
    .await
    .unwrap();
    let bank_data: Bank = solana.get_account(bank).await;
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: bank_data.native_deposits().to_num(),
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint0_account,
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // close account
    send_tx(
        solana,
        AccountCloseInstruction {
            group,
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
            mint_info: tokens[0].mint_info,
            banks: {
                let mint_info: MintInfo = solana.get_account(tokens[0].mint_info).await;
                mint_info.banks.to_vec()
            },
            vaults: {
                let mint_info: MintInfo = solana.get_account(tokens[0].mint_info).await;
                mint_info.vaults.to_vec()
            },
            dust_vault: payer_mint0_account,
            token_index: bank_data.token_index,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    // close stub oracle
    send_tx(
        solana,
        StubOracleCloseInstruction {
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
        GroupCloseInstruction {
            group,
            admin,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    Ok(())
}

#[tokio::test]
async fn test_account_size_migration() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..1];

    let mango_setup::GroupWithTokens { group, .. } = mango_setup::GroupWithTokensConfig {
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
            token_count: 6,
            serum3_count: 3,
            perp_count: 3,
            perp_oo_count: 3,
            token_conditional_swap_count: 3,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;

    // Manually extend the account to have too many perp positions
    let mut account_raw = solana
        .context
        .borrow_mut()
        .banks_client
        .get_account(account)
        .await
        .unwrap()
        .unwrap();
    let mango_account = MangoAccountValue::from_bytes(&account_raw.data[8..]).unwrap();

    let perp_start = mango_account.header.perp_offset(0);
    let mut new_bytes: Vec<u8> = Vec::new();
    new_bytes.extend_from_slice(&account_raw.data[..8 + std::mem::size_of::<MangoAccountFixed>()]);
    new_bytes.extend_from_slice(&mango_account.dynamic[..perp_start - 4]);
    new_bytes.extend_from_slice(&(3u32 + 10u32).to_le_bytes()); // perp pos len
    for _ in 0..10 {
        new_bytes.extend_from_slice(&bytemuck::bytes_of(&PerpPosition::default()));
    }
    // remove the 64 reserved bytes at the end
    new_bytes
        .extend_from_slice(&mango_account.dynamic[perp_start..mango_account.dynamic.len() - 64]);

    account_raw.data = new_bytes.clone();
    account_raw.lamports = 1_000_000_000; // 1 SOL is enough
    solana
        .context
        .borrow_mut()
        .set_account(&account, &account_raw.into());

    //
    // TEST: Size migration reduces number of available perp positions
    //

    let new_mango_account = MangoAccountValue::from_bytes(&new_bytes[8..]).unwrap();
    assert!(
        new_mango_account.header.expected_health_accounts()
            > MangoAccountDynamicHeader::max_health_accounts()
    );
    assert_eq!(new_mango_account.header.perp_count, 13);

    send_tx(solana, AccountSizeMigrationInstruction { account, payer })
        .await
        .unwrap();

    let mango_account = get_mango_account(solana, account).await;
    assert!(
        mango_account.header.expected_health_accounts()
            <= MangoAccountDynamicHeader::max_health_accounts()
    );
    assert_eq!(mango_account.header.perp_count, 4);

    println!("{:#?}", mango_account.header);

    //
    // TEST: running size migration again has no effect
    //

    let before_bytes = solana.get_account_data(account).await;
    send_tx(solana, AccountSizeMigrationInstruction { account, payer })
        .await
        .unwrap();
    let after_bytes = solana.get_account_data(account).await;
    assert_eq!(before_bytes, after_bytes);

    Ok(())
}
