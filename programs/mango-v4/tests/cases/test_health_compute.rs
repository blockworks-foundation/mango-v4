use super::*;

// Try to reach compute limits in health checks by having many different tokens in an account
#[tokio::test]
async fn test_health_compute_tokens() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..10];

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

    // each deposit ends with a health check
    create_funded_account(&solana, group, owner, 0, &context.users[1], mints, 1000, 0).await;

    // TODO: actual explicit CU comparisons.
    // On 2023-2-5 the final deposit costs 57622 CU and each new token increases it by roughly 2400 CU

    Ok(())
}

// Try to reach compute limits in health checks by having many serum markets in an account
#[tokio::test]
async fn test_health_compute_serum() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(90_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..8];
    let payer_mint_accounts = &context.users[1].token_accounts[0..mints.len()];

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

    let account = send_tx(
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

    //
    // TEST: Create open orders and trigger a Deposit to check health
    //
    for (i, &serum_market) in serum_markets.iter().enumerate() {
        println!("adding market {}", i);
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

        send_tx(
            solana,
            TokenDepositInstruction {
                amount: 10,
                reduce_only: false,
                account,
                owner,
                token_account: payer_mint_accounts[0],
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();
    }

    // TODO: actual explicit CU comparisons.
    // On 2023-2-23 the final deposit costs 81141 CU and each new market increases it by roughly 6184 CU

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
    let mints = &context.mints[0..8];
    let payer_mint_accounts = &context.users[1].token_accounts[0..mints.len()];

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
                max_quote_lots: i64::MAX,
                reduce_only: false,
                client_order_id: 0,
            },
        )
        .await
        .unwrap();

        send_tx(
            solana,
            TokenDepositInstruction {
                amount: 10,
                reduce_only: false,
                account,
                owner,
                token_account: payer_mint_accounts[0],
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();
    }

    // TODO: actual explicit CU comparisons.
    // On 2023-2-5 the final deposit costs 60732 CU and each new market increases it by roughly 3400 CU

    Ok(())
}
