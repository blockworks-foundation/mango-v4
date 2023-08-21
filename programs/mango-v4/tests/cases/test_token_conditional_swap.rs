use super::*;

#[tokio::test]
async fn test_token_conditional_swap() -> Result<(), TransportError> {
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
        .has_data());

    Ok(())
}
