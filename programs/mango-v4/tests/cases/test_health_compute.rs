use super::*;
use anchor_lang::prelude::AccountMeta;
use mango_v4::accounts_ix::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};

async fn deposit_cu_datapoint(
    solana: &SolanaCookie,
    account: Pubkey,
    owner: TestKeypair,
    token_account: Pubkey,
) -> u64 {
    let result = send_tx_get_metadata(
        solana,
        TokenDepositInstruction {
            amount: 10,
            reduce_only: false,
            account,
            owner,
            token_account,
            token_authority: owner,
            bank_index: 0,
        },
    )
    .await
    .unwrap();
    result.metadata.unwrap().compute_units_consumed
}

async fn deposit_cu_fallbacks_datapoint(
    solana: &SolanaCookie,
    account: Pubkey,
    owner: TestKeypair,
    token_account: Pubkey,
    remaining_accounts: Vec<AccountMeta>,
) -> u64 {
    let result = send_tx_with_extra_accounts(
        solana,
        TokenDepositInstruction {
            amount: 10,
            reduce_only: false,
            account,
            owner,
            token_account,
            token_authority: owner,
            bank_index: 0,
        },
        remaining_accounts,
    )
    .await
    .unwrap();
    result.metadata.unwrap().compute_units_consumed
}

// Try to reach compute limits in health checks by having many different tokens in an account
#[tokio::test]
async fn test_health_compute_tokens() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..8];

    //
    // SETUP: Create a group and an account
    //

    let GroupWithTokens { group, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let account =
        create_funded_account(&solana, group, owner, 0, &context.users[1], &[], 1000, 0).await;

    let mut cu_measurements = vec![];
    for token_account in &context.users[0].token_accounts[..mints.len()] {
        cu_measurements.push(deposit_cu_datapoint(solana, account, owner, *token_account).await);
    }

    for (i, pair) in cu_measurements.windows(2).enumerate() {
        println!(
            "after adding token {}: {} (+{})",
            i,
            pair[1],
            pair[1] - pair[0]
        );
    }

    let avg_cu_increase = cu_measurements.windows(2).map(|p| p[1] - p[0]).sum::<u64>()
        / (cu_measurements.len() - 1) as u64;
    println!("average cu increase: {avg_cu_increase}");
    assert!(avg_cu_increase < 3350);

    Ok(())
}

#[tokio::test]
async fn test_health_compute_tokens_during_maint_weight_shift() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..8];

    //
    // SETUP: Create a group and an account
    //

    let GroupWithTokens { group, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let account =
        create_funded_account(&solana, group, owner, 0, &context.users[1], &[], 1000, 0).await;

    let now = solana.clock_timestamp().await;
    for mint in mints {
        send_tx(
            solana,
            TokenEdit {
                group,
                admin,
                mint: mint.pubkey,
                fallback_oracle: Pubkey::default(),
                options: mango_v4::instruction::TokenEdit {
                    maint_weight_shift_start_opt: Some(now - 1000),
                    maint_weight_shift_end_opt: Some(now + 1000),
                    maint_weight_shift_asset_target_opt: Some(0.1),
                    maint_weight_shift_liab_target_opt: Some(1.1),
                    ..token_edit_instruction_default()
                },
            },
        )
        .await
        .unwrap();
    }

    let mut cu_measurements = vec![];
    for token_account in &context.users[0].token_accounts[..mints.len()] {
        cu_measurements.push(deposit_cu_datapoint(solana, account, owner, *token_account).await);
    }

    for (i, pair) in cu_measurements.windows(2).enumerate() {
        println!(
            "after adding token {}: {} (+{})",
            i,
            pair[1],
            pair[1] - pair[0]
        );
    }

    let avg_cu_increase = cu_measurements.windows(2).map(|p| p[1] - p[0]).sum::<u64>()
        / (cu_measurements.len() - 1) as u64;
    println!("average cu increase: {avg_cu_increase}");
    assert!(avg_cu_increase < 4300);

    Ok(())
}

// Try to reach compute limits in health checks by having many different tokens in an account and using fallback oracles for them
#[tokio::test]
async fn test_health_compute_tokens_fallback_oracles() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(450_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let num_tokens = 8;

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..num_tokens];

    let mut fallback_oracle_kps = Vec::with_capacity(num_tokens);
    for _ in 0..num_tokens {
        fallback_oracle_kps.push(TestKeypair::new());
    }
    let success_metas: Vec<AccountMeta> = fallback_oracle_kps
        .iter()
        .map(|x| AccountMeta {
            pubkey: x.pubkey(),
            is_signer: false,
            is_writable: false,
        })
        .collect();

    let failure_metas = vec![];

    //
    // SETUP: Create a group and an account
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let account =
        create_funded_account(&solana, group, owner, 0, &context.users[1], &[], 1000, 0).await;

    let mut success_measurements = vec![];
    let mut failure_measurements = vec![];
    for token_account in &context.users[0].token_accounts[..mints.len()] {
        deposit_cu_datapoint(solana, account, owner, *token_account).await;
    }

    //
    // SETUP: Create and register fallback oracles for each token
    //
    for (i, _token_account) in context.users[0].token_accounts[..mints.len()]
        .iter()
        .enumerate()
    {
        send_tx(
            solana,
            StubOracleCreate {
                oracle: fallback_oracle_kps[i],
                group,
                mint: mints[i].pubkey,
                admin,
                payer,
            },
        )
        .await
        .unwrap();

        send_tx(
            solana,
            TokenEdit {
                group,
                admin,
                mint: mints[i].pubkey,
                fallback_oracle: fallback_oracle_kps[i].pubkey(),
                options: mango_v4::instruction::TokenEdit {
                    set_fallback_oracle: true,
                    ..token_edit_instruction_default()
                },
            },
        )
        .await
        .unwrap();
    }

    //
    // TEST: Progressively make each oracle invalid so that the fallback is used
    //
    for (i, token_account) in context.users[0].token_accounts[..mints.len()]
        .iter()
        .enumerate()
    {
        send_tx(
            solana,
            StubOracleSetTestInstruction {
                oracle: tokens[i].oracle,
                group,
                mint: mints[i].pubkey,
                admin,
                price: 1.0,
                last_update_slot: 0,
                deviation: 100.0,
            },
        )
        .await
        .unwrap();

        success_measurements.push(
            deposit_cu_fallbacks_datapoint(
                solana,
                account,
                owner,
                *token_account,
                success_metas.clone(),
            )
            .await,
        );

        failure_measurements.push(
            deposit_cu_fallbacks_datapoint(
                solana,
                account,
                owner,
                *token_account,
                failure_metas.clone(),
            )
            .await,
        );
    }
    println!("successful fallbacks:");
    for (i, pair) in success_measurements.windows(2).enumerate() {
        println!(
            "after adding token {}: {} (+{})",
            i,
            pair[1],
            pair[1] - pair[0]
        );
    }
    println!("failed fallbacks:");
    for (i, pair) in failure_measurements.windows(2).enumerate() {
        println!(
            "after adding token {}: {} (+{})",
            i,
            pair[1],
            pair[1] - pair[0]
        );
    }

    let avg_success_increase = success_measurements
        .windows(2)
        .map(|p| p[1] - p[0])
        .sum::<u64>()
        / (success_measurements.len() - 1) as u64;

    let avg_failure_increase = failure_measurements
        .windows(2)
        .map(|p| p[1] - p[0])
        .sum::<u64>()
        / (failure_measurements.len() - 1) as u64;
    println!("average success increase: {avg_success_increase}");
    println!("average failure increase: {avg_failure_increase}");
    assert!(avg_success_increase < 2_050);
    assert!(avg_failure_increase < 19_500);

    Ok(())
}

// Try to reach compute limits in health checks by having many serum markets in an account
#[tokio::test]
async fn test_health_compute_serum() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(135_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..5];
    let token_account = context.users[0].token_accounts[0];

    //
    // SETUP: Create a group and an account
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // Also activate all token positions
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints,
        10000,
        0,
    )
    .await;

    // Allow 8 tokens + 8 serum
    send_tx(
        solana,
        AccountExpandInstruction {
            account_num: 0,
            token_count: 8,
            serum3_count: 8,
            perp_count: 0,
            perp_oo_count: 0,
            token_conditional_swap_count: 0,
            group,
            owner,
            payer,
            ..Default::default()
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Create serum markets and register them
    //
    let quote_token = &tokens[0];
    let mut serum_market_cookies = vec![];
    for token in tokens[1..].iter() {
        serum_market_cookies.push((
            token,
            context
                .serum
                .list_spot_market(&token.mint, &quote_token.mint)
                .await,
        ));
    }

    let mut serum_markets = vec![];
    for (base_token, spot) in serum_market_cookies {
        serum_markets.push(
            send_tx(
                solana,
                Serum3RegisterMarketInstruction {
                    group,
                    admin,
                    serum_program: context.serum.program_id,
                    serum_market_external: spot.market,
                    market_index: spot.coin_mint.index as u16,
                    base_bank: base_token.bank,
                    quote_bank: quote_token.bank,
                    payer,
                },
            )
            .await
            .unwrap()
            .serum_market,
        );
    }

    let mut cu_measurements = vec![];

    // Get the baseline cost of a deposit without an active serum3 oo
    cu_measurements.push(deposit_cu_datapoint(solana, account, owner, token_account).await);

    //
    // TEST: Create open orders and trigger a Deposit to check health
    //
    for &serum_market in serum_markets.iter() {
        send_tx(
            solana,
            Serum3CreateOpenOrdersInstruction {
                account,
                serum_market,
                owner,
                payer,
            },
        )
        .await
        .unwrap();

        // Place a bid and ask to make the health computation use more compute
        send_tx(
            solana,
            Serum3PlaceOrderInstruction {
                side: Serum3Side::Bid,
                limit_price: 1,
                max_base_qty: 1,
                max_native_quote_qty_including_fees: 1,
                self_trade_behavior: Serum3SelfTradeBehavior::AbortTransaction,
                order_type: Serum3OrderType::Limit,
                client_order_id: 0,
                limit: 10,
                account,
                owner,
                serum_market,
            },
        )
        .await
        .unwrap();
        send_tx(
            solana,
            Serum3PlaceOrderInstruction {
                side: Serum3Side::Ask,
                limit_price: 2,
                max_base_qty: 1,
                max_native_quote_qty_including_fees: 1,
                self_trade_behavior: Serum3SelfTradeBehavior::AbortTransaction,
                order_type: Serum3OrderType::Limit,
                client_order_id: 1,
                limit: 10,
                account,
                owner,
                serum_market,
            },
        )
        .await
        .unwrap();

        // Simple deposit: it's cost is what we use as reference for health compute cost
        cu_measurements.push(deposit_cu_datapoint(solana, account, owner, token_account).await);
    }

    for (i, pair) in cu_measurements.windows(2).enumerate() {
        println!(
            "after adding market {}: {} (+{})",
            i,
            pair[1],
            pair[1] - pair[0]
        );
    }

    let avg_cu_increase = cu_measurements.windows(2).map(|p| p[1] - p[0]).sum::<u64>()
        / (cu_measurements.len() - 1) as u64;
    println!("average cu increase: {avg_cu_increase}");
    assert!(avg_cu_increase < 7000);

    Ok(())
}

// Try to reach compute limits in health checks by having many perp markets in an account
#[tokio::test]
async fn test_health_compute_perp() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(90_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..5];
    let token_account = context.users[0].token_accounts[0];

    //
    // SETUP: Create a group and an account
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints[..1],
        1000,
        0,
    )
    .await;

    //
    // SETUP: Create perp markets
    //
    let mut perp_markets = vec![];
    for (perp_market_index, token) in tokens[1..].iter().enumerate() {
        let mango_v4::accounts::PerpCreateMarket { perp_market, .. } = send_tx(
            solana,
            PerpCreateMarketInstruction {
                group,
                admin,
                payer,
                perp_market_index: perp_market_index as PerpMarketIndex,
                quote_lot_size: 10,
                base_lot_size: 100,
                maint_base_asset_weight: 0.975,
                init_base_asset_weight: 0.95,
                maint_base_liab_weight: 1.025,
                init_base_liab_weight: 1.05,
                base_liquidation_fee: 0.012,
                maker_fee: 0.0002,
                taker_fee: 0.000,
                settle_pnl_limit_factor: 0.2,
                settle_pnl_limit_window_size_ts: 24 * 60 * 60,
                ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &token).await
            },
        )
        .await
        .unwrap();

        perp_markets.push(perp_market);
    }

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_markets[0]).await;
        perp_market.native_price_to_lot(I80F48::from(1))
    };

    let mut cu_measurements = vec![];

    // Get the baseline cost of a deposit without an active serum3 oo
    cu_measurements.push(deposit_cu_datapoint(solana, account, owner, token_account).await);

    //
    // TEST: Create a perp order for each market
    //
    for (i, &perp_market) in perp_markets.iter().enumerate() {
        println!("adding market {}", i);
        send_tx(
            solana,
            PerpPlaceOrderInstruction {
                account,
                perp_market,
                owner,
                side: Side::Bid,
                price_lots,
                max_base_lots: 1,
                ..PerpPlaceOrderInstruction::default()
            },
        )
        .await
        .unwrap();

        cu_measurements.push(deposit_cu_datapoint(solana, account, owner, token_account).await);
    }

    for (i, pair) in cu_measurements.windows(2).enumerate() {
        println!(
            "after adding perp market {}: {} (+{})",
            i,
            pair[1],
            pair[1] - pair[0]
        );
    }

    let avg_cu_increase = cu_measurements.windows(2).map(|p| p[1] - p[0]).sum::<u64>()
        / (cu_measurements.len() - 1) as u64;
    println!("average cu increase: {avg_cu_increase}");
    assert!(avg_cu_increase < 4800);

    Ok(())
}
