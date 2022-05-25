#![cfg(feature = "test-bpf")]

use fixed::types::I80F48;
use mango_v4::state::*;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use program_test::*;

mod program_test;

// Try to reach compute limits in health checks by having many different tokens in an account
#[tokio::test]
async fn test_health_compute_tokens() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..10];
    let payer_mint_accounts = &context.users[1].token_accounts[0..mints.len()];

    //
    // SETUP: Create a group and an account
    //

    let mango_setup::GroupWithTokens { group, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;

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
    // TEST: Deposit user funds for all the mints
    // each deposit will end with a health check
    //
    for &token_account in payer_mint_accounts {
        let deposit_amount = 1000;

        send_tx(
            solana,
            DepositInstruction {
                amount: deposit_amount,
                account,
                token_account,
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    // TODO: actual explicit CU comparisons.
    // On 2022-5-25 the final deposit costs 36905 CU and each new token increases it by roughly 1600 CU

    Ok(())
}

// Try to reach compute limits in health checks by having many serum markets in an account
#[tokio::test]
async fn test_health_compute_serum() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..8];
    let payer_mint_accounts = &context.users[1].token_accounts[0..mints.len()];

    //
    // SETUP: Create a group and an account
    //

    let mango_setup::GroupWithTokens { group, tokens } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;

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
            DepositInstruction {
                amount: 10,
                account,
                token_account: payer_mint_accounts[0],
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    // TODO: actual explicit CU comparisons.
    // On 2022-5-25 the final deposit costs 52252 CU and each new market increases it by roughly 4400 CU

    Ok(())
}

// Try to reach compute limits in health checks by having many perp markets in an account
#[tokio::test]
async fn test_health_compute_perp() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..8];
    let payer_mint_accounts = &context.users[1].token_accounts[0..mints.len()];

    //
    // SETUP: Create a group and an account
    //

    let mango_setup::GroupWithTokens { group, tokens } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;

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

    // Give the account some quote currency
    send_tx(
        solana,
        DepositInstruction {
            amount: 1000,
            account,
            token_account: payer_mint_accounts[0],
            token_authority: payer,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Create perp markets
    //
    let quote_token = &tokens[0];
    let mut perp_markets = vec![];
    for (perp_market_index, token) in tokens[1..].iter().enumerate() {
        let mango_v4::accounts::PerpCreateMarket {
            perp_market,
            asks,
            bids,
            event_queue,
            ..
        } = send_tx(
            solana,
            PerpCreateMarketInstruction {
                group,
                admin,
                oracle: token.oracle,
                asks: context
                    .solana
                    .create_account_for_type::<BookSide>(&mango_v4::id())
                    .await,
                bids: context
                    .solana
                    .create_account_for_type::<BookSide>(&mango_v4::id())
                    .await,
                event_queue: {
                    context
                        .solana
                        .create_account_for_type::<EventQueue>(&mango_v4::id())
                        .await
                },
                payer,
                perp_market_index: perp_market_index as PerpMarketIndex,
                base_token_index: quote_token.index,
                quote_token_index: token.index,
                quote_lot_size: 10,
                base_lot_size: 100,
                maint_asset_weight: 0.975,
                init_asset_weight: 0.95,
                maint_liab_weight: 1.025,
                init_liab_weight: 1.05,
                liquidation_fee: 0.012,
                maker_fee: 0.0002,
                taker_fee: 0.000,
            },
        )
        .await
        .unwrap();

        perp_markets.push((perp_market, asks, bids, event_queue));
    }

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_markets[0].0).await;
        perp_market.native_price_to_lot(I80F48::from(1))
    };

    //
    // TEST: Create a perp order for each market
    //
    for (i, &(perp_market, asks, bids, event_queue)) in perp_markets.iter().enumerate() {
        println!("adding market {}", i);
        send_tx(
            solana,
            PerpPlaceOrderInstruction {
                group,
                account,
                perp_market,
                asks,
                bids,
                event_queue,
                oracle: tokens[i + 1].oracle,
                owner,
                side: Side::Bid,
                price_lots,
                max_base_lots: 1,
                max_quote_lots: i64::MAX,
                client_order_id: 0,
            },
        )
        .await
        .unwrap();

        send_tx(
            solana,
            DepositInstruction {
                amount: 10,
                account,
                token_account: payer_mint_accounts[0],
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    // TODO: actual explicit CU comparisons.
    // On 2022-5-25 the final deposit costs 32700 CU and each new market increases it by roughly 1500 CU

    Ok(())
}
