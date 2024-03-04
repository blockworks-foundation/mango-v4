use super::*;

#[tokio::test]
async fn test_token_conditional_swap_basic() -> Result<(), TransportError> {
    pub use utils::assert_equal_f64_f64 as assert_equal_f_f;

    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];

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
    let quote_token = &tokens[0];
    let base_token = &tokens[1];

    let deposit_amount = 1_000_000_000f64;
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        deposit_amount as u64,
        0,
    )
    .await;
    let liqor = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        deposit_amount as u64,
        0,
    )
    .await;
    let no_tcs_account = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 2,
            token_conditional_swap_count: 0,
            group,
            owner,
            payer,
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .account;

    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: quote_token.mint.pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                token_conditional_swap_taker_fee_rate_opt: Some(0.05),
                token_conditional_swap_maker_fee_rate_opt: Some(0.1),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    //
    // TEST: Trying to add a tcs on an account without space will fail
    //
    let tx_result = send_tx(
        solana,
        TokenConditionalSwapCreateInstruction {
            account: no_tcs_account,
            owner,
            buy_mint: quote_token.mint.pubkey,
            sell_mint: base_token.mint.pubkey,
            max_buy: 1000,
            max_sell: 1000,
            price_lower_limit: 1.0,
            price_upper_limit: 10.0,
            price_premium_rate: 0.01,
            allow_creating_deposits: true,
            allow_creating_borrows: true,
        },
    )
    .await;
    assert!(tx_result.is_err());

    //
    // TEST: Extending an account to have space for tcs works
    //
    send_tx(
        solana,
        AccountExpandInstruction {
            account_num: 0,
            token_count: 8,
            serum3_count: 4,
            perp_count: 4,
            perp_oo_count: 16,
            token_conditional_swap_count: 2,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;
    let account_data = get_mango_account(solana, account).await;
    assert_eq!(account_data.header.token_conditional_swap_count, 2);

    //
    // TEST: Can create tcs until all slots are filled
    //
    let tcs_ix = TokenConditionalSwapCreateInstruction {
        account,
        owner,
        buy_mint: quote_token.mint.pubkey,
        sell_mint: base_token.mint.pubkey,
        max_buy: 100,
        max_sell: 100,
        price_lower_limit: 0.9,
        price_upper_limit: 10.0,
        price_premium_rate: 0.1,
        allow_creating_deposits: true,
        allow_creating_borrows: true,
    };
    send_tx(
        solana,
        TokenConditionalSwapCreateInstruction {
            max_buy: 101,
            ..tcs_ix
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenConditionalSwapCreateInstruction {
            max_buy: 102,
            price_lower_limit: 1.1,
            ..tcs_ix
        },
    )
    .await
    .unwrap();
    let tx_result = send_tx(solana, tcs_ix.clone()).await;
    assert!(tx_result.is_err());

    let account_data = get_mango_account(solana, account).await;
    assert_eq!(
        account_data
            .token_conditional_swap_by_index(0)
            .unwrap()
            .max_buy,
        101
    );
    assert_eq!(
        account_data
            .token_conditional_swap_by_index(1)
            .unwrap()
            .max_buy,
        102
    );

    //
    // TEST: Can cancel, and then readd a new one
    //
    send_tx(
        solana,
        TokenConditionalSwapCancelInstruction {
            account,
            owner,
            index: 0,
            id: 0,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenConditionalSwapCreateInstruction {
            max_buy: 103,
            ..tcs_ix
        },
    )
    .await
    .unwrap();
    let tx_result = send_tx(solana, tcs_ix.clone()).await;
    assert!(tx_result.is_err());

    let account_data = get_mango_account(solana, account).await;
    assert_eq!(
        account_data
            .token_conditional_swap_by_index(0)
            .unwrap()
            .max_buy,
        103
    );
    assert_eq!(
        account_data
            .token_conditional_swap_by_index(1)
            .unwrap()
            .max_buy,
        102
    );

    //
    // TEST: can't trigger if price threshold not reached
    //
    let tx_result = send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 1,
            max_buy_token_to_liqee: 50,
            max_sell_token_to_liqor: 50,
            min_buy_token: 0,
            min_taker_price: 0.0,
        },
    )
    .await;
    assert!(tx_result.is_err());

    //
    // TEST: trigger partially
    //
    send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 50,
            max_sell_token_to_liqor: 50,
            min_buy_token: 0,
            min_taker_price: 0.0,
        },
    )
    .await
    .unwrap();

    let liqee_quote = account_position_f64(solana, account, quote_token.bank).await;
    let liqee_base = account_position_f64(solana, account, base_token.bank).await;
    assert!(assert_equal_f_f(
        liqee_quote,
        deposit_amount + 42.0, // roughly 50 / (1.1 * 1.1)
        0.01
    ));
    assert!(assert_equal_f_f(liqee_base, deposit_amount - 50.0, 0.01));

    let liqor_quote = account_position_f64(solana, liqor, quote_token.bank).await;
    let liqor_base = account_position_f64(solana, liqor, base_token.bank).await;
    assert!(assert_equal_f_f(liqor_quote, deposit_amount - 42.0, 0.01));
    assert!(assert_equal_f_f(liqor_base, deposit_amount + 44.0, 0.01)); // roughly 42*1.1*0.95

    //
    // TEST: requiring a too-high min buy token execution makes it fail
    //
    let r = send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 5000,
            max_sell_token_to_liqor: 5000,
            min_buy_token: 50,
            min_taker_price: 0.0,
        },
    )
    .await;
    assert!(r.is_err());

    //
    // TEST: trigger fully
    //
    send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 5000,
            max_sell_token_to_liqor: 5000,
            min_buy_token: 40,
            min_taker_price: 0.0,
        },
    )
    .await
    .unwrap();

    let liqee_quote = account_position_f64(solana, account, quote_token.bank).await;
    let liqee_base = account_position_f64(solana, account, base_token.bank).await;
    assert!(assert_equal_f_f(liqee_quote, deposit_amount + 84.0, 0.01));
    assert!(assert_equal_f_f(liqee_base, deposit_amount - 100.0, 0.01));

    let liqor_quote = account_position_f64(solana, liqor, quote_token.bank).await;
    let liqor_base = account_position_f64(solana, liqor, base_token.bank).await;
    assert!(assert_equal_f_f(liqor_quote, deposit_amount - 84.0, 0.01));
    assert!(assert_equal_f_f(liqor_base, deposit_amount + 88.0, 0.01));

    let account_data = get_mango_account(solana, account).await;
    assert!(!account_data
        .token_conditional_swap_by_index(0)
        .unwrap()
        .is_configured());

    Ok(())
}

#[tokio::test]
async fn test_token_conditional_swap_linear_auction() -> Result<(), TransportError> {
    pub use utils::assert_equal_f64_f64 as assert_equal_f_f;

    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];

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
    let quote_token = &tokens[0];
    let base_token = &tokens[1];

    let deposit_amount = 1_000_000_000f64;
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        deposit_amount as u64,
        0,
    )
    .await;
    let liqor = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        deposit_amount as u64,
        0,
    )
    .await;

    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: quote_token.mint.pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                token_conditional_swap_taker_fee_rate_opt: Some(0.05),
                token_conditional_swap_maker_fee_rate_opt: Some(0.1),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Extending an account to have space for tcs works
    //
    send_tx(
        solana,
        AccountExpandInstruction {
            account_num: 0,
            token_count: 8,
            serum3_count: 4,
            perp_count: 4,
            perp_oo_count: 16,
            token_conditional_swap_count: 2,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;
    let account_data = get_mango_account(solana, account).await;
    assert_eq!(account_data.header.token_conditional_swap_count, 2);

    //
    // TEST: Can create tcs auction
    //
    let initial_time = solana.clock_timestamp().await;
    send_tx(
        solana,
        TokenConditionalSwapCreateLinearAuctionInstruction {
            account,
            owner,
            buy_mint: quote_token.mint.pubkey,
            sell_mint: base_token.mint.pubkey,
            max_buy: 100000,
            max_sell: 100000,
            price_start: 1.0,
            price_end: 11.0,
            allow_creating_deposits: true,
            allow_creating_borrows: true,
            start_timestamp: initial_time + 5,
            duration_seconds: 10,
            expiry_timestamp: initial_time + 5 + 10 + 5,
        },
    )
    .await
    .unwrap();

    let account_data = get_mango_account(solana, account).await;
    let tcss = account_data.active_token_conditional_swaps().collect_vec();
    assert_eq!(tcss.len(), 1);
    assert_eq!(
        tcss[0].tcs_type,
        TokenConditionalSwapType::LinearAuction as u8
    );

    //
    // TEST: Can't take an auction at any price when it's not started yet
    //

    let res = send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 100,
            max_sell_token_to_liqor: 100,
            min_buy_token: 0,
            min_taker_price: 0.0,
        },
    )
    .await;
    assert_mango_error(
        &res,
        MangoError::TokenConditionalSwapNotStarted.into(),
        "tcs should not be started yet".to_string(),
    );

    //
    // TEST: Check the auction price in the middle
    //
    solana.set_clock_timestamp(initial_time + 7).await;
    send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 1000,
            max_sell_token_to_liqor: 3300, // expected price of 3.0 sell per buy, with 10% maker fee
            min_buy_token: 1,
            min_taker_price: 0.0,
        },
    )
    .await
    .unwrap();

    let mut account_quote_expected = deposit_amount as f64 + 1000.0;
    let mut account_base_expected = deposit_amount as f64 + -3300.0;
    let mut liqor_quote_expected = deposit_amount as f64 + -1000.0;
    let mut liqor_base_expected = deposit_amount as f64 + 2850.0;

    let account_quote = account_position_f64(solana, account, quote_token.bank).await;
    let account_base = account_position_f64(solana, account, base_token.bank).await;
    assert!(assert_equal_f64_f64(
        account_quote,
        account_quote_expected,
        0.1
    ));
    assert!(assert_equal_f64_f64(
        account_base,
        account_base_expected,
        0.1
    ));

    let liqor_quote = account_position_f64(solana, liqor, quote_token.bank).await;
    let liqor_base = account_position_f64(solana, liqor, base_token.bank).await;
    assert!(assert_equal_f64_f64(liqor_quote, liqor_quote_expected, 0.1));
    assert!(assert_equal_f64_f64(liqor_base, liqor_base_expected, 0.1));

    //
    // TEST: Stays at end price after end and before expiry
    //
    solana.set_clock_timestamp(initial_time + 17).await;
    send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 1000,
            max_sell_token_to_liqor: 12100, // expected price of 11.0 sell per buy, with 10% maker fee
            min_buy_token: 1,
            min_taker_price: 0.0,
        },
    )
    .await
    .unwrap();

    account_quote_expected += 1000.0;
    account_base_expected += -12100.0;
    liqor_quote_expected += -1000.0;
    liqor_base_expected += 10450.0;

    let account_quote = account_position_f64(solana, account, quote_token.bank).await;
    let account_base = account_position_f64(solana, account, base_token.bank).await;
    assert!(assert_equal_f64_f64(
        account_quote,
        account_quote_expected,
        0.1
    ));
    assert!(assert_equal_f64_f64(
        account_base,
        account_base_expected,
        0.1
    ));

    let liqor_quote = account_position_f64(solana, liqor, quote_token.bank).await;
    let liqor_base = account_position_f64(solana, liqor, base_token.bank).await;
    assert!(assert_equal_f64_f64(liqor_quote, liqor_quote_expected, 0.1));
    assert!(assert_equal_f64_f64(liqor_base, liqor_base_expected, 0.1));

    //
    // TEST: Can't take when expired
    //
    solana.set_clock_timestamp(initial_time + 22).await;
    let res = send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 100,
            max_sell_token_to_liqor: 100,
            min_buy_token: 1,
            min_taker_price: 0.0,
        },
    )
    .await;
    assert_mango_error(
        &res,
        MangoError::TokenConditionalSwapExpired.into(),
        "tcs should be expired".to_string(),
    );

    Ok(())
}

#[tokio::test]
async fn test_token_conditional_swap_premium_auction() -> Result<(), TransportError> {
    pub use utils::assert_equal_f64_f64 as assert_equal_f_f;

    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];

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
    let quote_token = &tokens[0];
    let base_token = &tokens[1];

    let deposit_amount = 1_000_000_000f64;
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        deposit_amount as u64,
        0,
    )
    .await;
    let liqor = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        deposit_amount as u64,
        0,
    )
    .await;

    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: quote_token.mint.pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                token_conditional_swap_taker_fee_rate_opt: Some(0.05),
                token_conditional_swap_maker_fee_rate_opt: Some(0.1),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Extending an account to have space for tcs works
    //
    send_tx(
        solana,
        AccountExpandInstruction {
            account_num: 0,
            token_count: 8,
            serum3_count: 4,
            perp_count: 4,
            perp_oo_count: 16,
            token_conditional_swap_count: 2,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;
    let account_data = get_mango_account(solana, account).await;
    assert_eq!(account_data.header.token_conditional_swap_count, 2);

    //
    // TEST: Can create premium auction
    //
    send_tx(
        solana,
        TokenConditionalSwapCreatePremiumAuctionInstruction {
            account,
            owner,
            buy_mint: quote_token.mint.pubkey,
            sell_mint: base_token.mint.pubkey,
            max_buy: 100000,
            max_sell: 100000,
            price_lower_limit: 0.5,
            price_upper_limit: 2.0,
            max_price_premium_rate: 0.01,
            allow_creating_deposits: true,
            allow_creating_borrows: true,
            duration_seconds: 10,
        },
    )
    .await
    .unwrap();

    let account_data = get_mango_account(solana, account).await;
    let tcss = account_data.active_token_conditional_swaps().collect_vec();
    assert_eq!(tcss.len(), 1);
    assert_eq!(
        tcss[0].tcs_type,
        TokenConditionalSwapType::PremiumAuction as u8
    );

    //
    // TEST: Can't take or start an auction when the oracle price is out of range
    //

    set_bank_stub_oracle_price(solana, group, &base_token, admin, 10.0).await;
    let res = send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 100,
            max_sell_token_to_liqor: 100,
            min_buy_token: 0,
            min_taker_price: 0.0,
        },
    )
    .await;
    assert_mango_error(
        &res,
        MangoError::TokenConditionalSwapNotStarted.into(),
        "not started yet".to_string(),
    );

    let res = send_tx(
        solana,
        TokenConditionalSwapStartInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
        },
    )
    .await;
    assert_mango_error(
        &res,
        MangoError::TokenConditionalSwapPriceNotInRange.into(),
        "price not in range".to_string(),
    );

    //
    // TEST: Cannot trigger without start
    //
    set_bank_stub_oracle_price(solana, group, &base_token, admin, 1.0).await;
    let res = send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 1000,
            max_sell_token_to_liqor: 1100,
            min_buy_token: 1,
            min_taker_price: 0.0,
        },
    )
    .await;
    assert_mango_error(
        &res,
        MangoError::TokenConditionalSwapNotStarted.into(),
        "not started yet".to_string(),
    );

    send_tx(
        solana,
        TokenConditionalSwapStartInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 1000,
            max_sell_token_to_liqor: 1100, // expected price of 1.0 sell per buy, with 10% maker fee
            min_buy_token: 1,
            min_taker_price: 0.0,
        },
    )
    .await
    .unwrap();

    // 0 premium because only just started
    let mut account_quote_expected = deposit_amount as f64 + 1000.0;
    let mut account_base_expected = deposit_amount as f64 + -1100.0 - 1000.0;
    let mut liqor_quote_expected = deposit_amount as f64 + -1000.0;
    let mut liqor_base_expected = deposit_amount as f64 + 950.0 + 1000.0;

    let account_quote = account_position_f64(solana, account, quote_token.bank).await;
    let account_base = account_position_f64(solana, account, base_token.bank).await;
    assert!(assert_equal_f64_f64(
        account_quote,
        account_quote_expected,
        0.1
    ));
    assert!(assert_equal_f64_f64(
        account_base,
        account_base_expected,
        0.1
    ));

    let liqor_quote = account_position_f64(solana, liqor, quote_token.bank).await;
    let liqor_base = account_position_f64(solana, liqor, base_token.bank).await;
    assert!(assert_equal_f64_f64(liqor_quote, liqor_quote_expected, 0.1));
    assert!(assert_equal_f64_f64(liqor_base, liqor_base_expected, 0.1));

    let account_data = get_mango_account(solana, account).await;
    let tcs = account_data
        .active_token_conditional_swaps()
        .next()
        .unwrap();
    assert!(tcs.start_timestamp > 0);

    //
    // TEST: Premium increases
    //
    solana.set_clock_timestamp(tcs.start_timestamp + 5).await;
    send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 90000,
            // expected price of 1.0+0.5% premium sell per buy, with 10% maker fee, so buy token transfer of 10k
            max_sell_token_to_liqor: 11055,
            min_buy_token: 1,
            min_taker_price: 0.0,
        },
    )
    .await
    .unwrap();

    account_quote_expected += 10000.0;
    account_base_expected += -11055.0;
    liqor_quote_expected += -10000.0;
    liqor_base_expected += 9547.0; // 10000 * 1 * 1.005 premium * 0.95 taker fee

    let account_quote = account_position_f64(solana, account, quote_token.bank).await;
    let account_base = account_position_f64(solana, account, base_token.bank).await;
    assert!(assert_equal_f64_f64(
        account_quote,
        account_quote_expected,
        0.1
    ));
    assert!(assert_equal_f64_f64(
        account_base,
        account_base_expected,
        0.1
    ));

    let liqor_quote = account_position_f64(solana, liqor, quote_token.bank).await;
    let liqor_base = account_position_f64(solana, liqor, base_token.bank).await;
    assert!(assert_equal_f64_f64(liqor_quote, liqor_quote_expected, 0.1));
    assert!(assert_equal_f64_f64(liqor_base, liqor_base_expected, 0.1));

    //
    // TEST: Premium stops at max increases
    //
    solana.set_clock_timestamp(tcs.start_timestamp + 50).await;
    send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 10000,
            // expected price of 1.0+1% premium sell per buy, with 10% maker fee, so sell token transfer of 11110
            max_sell_token_to_liqor: 90000,
            min_buy_token: 1,
            min_taker_price: 0.0,
        },
    )
    .await
    .unwrap();

    account_quote_expected += 10000.0;
    account_base_expected += -11110.0;
    liqor_quote_expected += -10000.0;
    liqor_base_expected += 9595.0; // 10000 * 1 * 1.01 premium * 0.95 taker fee

    let account_quote = account_position_f64(solana, account, quote_token.bank).await;
    let account_base = account_position_f64(solana, account, base_token.bank).await;
    assert!(assert_equal_f64_f64(
        account_quote,
        account_quote_expected,
        0.1
    ));
    assert!(assert_equal_f64_f64(
        account_base,
        account_base_expected,
        0.1
    ));

    let liqor_quote = account_position_f64(solana, liqor, quote_token.bank).await;
    let liqor_base = account_position_f64(solana, liqor, base_token.bank).await;
    assert!(assert_equal_f64_f64(liqor_quote, liqor_quote_expected, 0.1));
    assert!(assert_equal_f64_f64(liqor_base, liqor_base_expected, 0.1));

    //
    // SETUP: make another premium auction to test starting
    //

    send_tx(
        solana,
        TokenConditionalSwapCreatePremiumAuctionInstruction {
            account,
            owner,
            buy_mint: quote_token.mint.pubkey,
            sell_mint: base_token.mint.pubkey,
            max_buy: 100000,
            max_sell: 100000,
            price_lower_limit: 1.5,
            price_upper_limit: 3.0,
            max_price_premium_rate: 0.01,
            allow_creating_deposits: true,
            allow_creating_borrows: true,
            duration_seconds: 10,
        },
    )
    .await
    .unwrap();

    //
    // TEST: Can't start if oracle not in range
    //

    let res = send_tx(
        solana,
        TokenConditionalSwapStartInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 1,
        },
    )
    .await;
    assert_mango_error(
        &res,
        MangoError::TokenConditionalSwapPriceNotInRange.into(),
        "price is not in range".to_string(),
    );

    //
    // TEST: Can start if in range
    //

    set_bank_stub_oracle_price(solana, group, &base_token, admin, 0.5).await;
    send_tx(
        solana,
        TokenConditionalSwapStartInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 1,
        },
    )
    .await
    .unwrap();

    let account_data = get_mango_account(solana, account).await;
    let tcs = account_data.active_token_conditional_swaps().collect_vec()[1];
    assert!(tcs.start_timestamp > 0);

    // TODO: more checks of the incentive payment!
    assert!(tcs.sold > 0);

    //
    // TEST: Can't start a second time
    //

    let res = send_tx(
        solana,
        TokenConditionalSwapStartInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 1,
        },
    )
    .await;
    assert_mango_error(
        &res,
        MangoError::TokenConditionalSwapAlreadyStarted.into(),
        "already started".to_string(),
    );

    Ok(())
}

#[tokio::test]
async fn test_token_conditional_swap_deposit_limit() -> Result<(), TransportError> {
    pub use utils::assert_equal_f64_f64 as assert_equal_f_f;

    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];

    //
    // SETUP: Create a group, account, tokens
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let quote_token = &tokens[0];
    let base_token = &tokens[1];

    // total deposits on quote and base is 2x this value
    let deposit_amount = 1_000f64;
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints[..1],
        deposit_amount as u64,
        0,
    )
    .await;
    let liqor = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        deposit_amount as u64,
        0,
    )
    .await;
    create_funded_account(
        &solana,
        group,
        owner,
        99,
        &context.users[1],
        &mints[1..],
        deposit_amount as u64,
        0,
    )
    .await;

    //
    // SETUP: A base deposit limit
    //
    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: base_token.mint.pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                deposit_limit_opt: Some(2500),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Sell base token from "account" creating borrows that are deposited into liqor,
    // increasing the total deposits
    //
    send_tx(
        solana,
        TokenConditionalSwapCreateInstruction {
            account,
            owner,
            buy_mint: quote_token.mint.pubkey,
            sell_mint: base_token.mint.pubkey,
            max_buy: 900,
            max_sell: 900,
            price_lower_limit: 0.0,
            price_upper_limit: 10.0,
            price_premium_rate: 0.01,
            allow_creating_deposits: true,
            allow_creating_borrows: true,
        },
    )
    .await
    .unwrap();

    //
    // TEST: Large execution fails, a bit smaller is ok
    //

    send_tx_expect_error!(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 100000,
            max_sell_token_to_liqor: 501,
            min_buy_token: 1,
            min_taker_price: 0.0,
        },
        MangoError::BankDepositLimit,
    );

    send_tx(
        solana,
        TokenConditionalSwapTriggerInstruction {
            liqee: account,
            liqor,
            liqor_owner: owner,
            index: 0,
            max_buy_token_to_liqee: 100000,
            max_sell_token_to_liqor: 499,
            min_buy_token: 1,
            min_taker_price: 0.0,
        },
    )
    .await
    .unwrap();

    Ok(())
}
