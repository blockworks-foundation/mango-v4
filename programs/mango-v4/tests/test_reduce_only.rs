#![cfg(feature = "test-bpf")]

use fixed::types::I80F48;
use mango_setup::*;
use mango_v4::state::{Bank, MangoAccount, PerpMarket, Side};
use program_test::*;
use solana_program_test::*;
use solana_sdk::transport::TransportError;

mod program_test;

#[tokio::test]
async fn test_reduce_only_token() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..=2];

    let initial_token_deposit = 10_000;

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

    //
    // SETUP: Prepare accounts
    //
    let account_0 = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints[0..2],
        initial_token_deposit,
        0,
    )
    .await;

    // make token reduce only
    send_tx(
        solana,
        TokenMakeReduceOnly {
            admin,
            group,
            mint: mints[0].pubkey,
        },
    )
    .await
    .unwrap();

    // deposit without reduce_only should fail
    let res = send_tx(
        solana,
        TokenDepositInstruction {
            amount: 10,
            reduce_only: false,
            account: account_0,
            owner,
            token_account: payer_mint_accounts[0],
            token_authority: payer,
            bank_index: 0,
        },
    )
    .await;
    assert!(res.is_err());

    // deposit with reduce_only should pass silently
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 10,
            reduce_only: true,
            account: account_0,
            owner,
            token_account: payer_mint_accounts[0],
            token_authority: payer,
            bank_index: 0,
        },
    )
    .await
    .unwrap();
    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    let bank = solana.get_account::<Bank>(tokens[0].bank).await;
    let native = mango_account_0.tokens[0].native(&bank);
    assert_eq!(native.to_num::<u64>(), initial_token_deposit);

    // withdraw all should pass
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: initial_token_deposit,
            allow_borrow: false,
            account: account_0,
            owner,
            token_account: payer_mint_accounts[0],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // borrowing should fail
    let res = send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: initial_token_deposit,
            allow_borrow: true,
            account: account_0,
            owner,
            token_account: payer_mint_accounts[0],
            bank_index: 0,
        },
    )
    .await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_perp_reduce_only() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..=2];

    let initial_token_deposit = 1000_000;

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

    let settler =
        create_funded_account(&solana, group, owner, 251, &context.users[1], &[], 0, 0).await;
    let settler_owner = owner.clone();

    let account_0 = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints[0..1],
        initial_token_deposit,
        0,
    )
    .await;
    let account_1 = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        &mints[0..1],
        initial_token_deposit * 100, // Fund 100x, so that this is not the bound for what account_0 can settle
        0,
    )
    .await;

    //
    // TEST: Create a perp market
    //
    let mango_v4::accounts::PerpCreateMarket { perp_market, .. } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 0,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_asset_weight: 0.975,
            init_asset_weight: 0.95,
            maint_liab_weight: 1.025,
            init_liab_weight: 1.05,
            liquidation_fee: 0.012,
            maker_fee: 0.0002,
            taker_fee: 0.000,
            settle_pnl_limit_factor: 0.2,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[1]).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::from(1000))
    };

    // Set the initial oracle price
    send_tx(
        solana,
        StubOracleSetInstruction {
            group,
            admin,
            mint: mints[1].pubkey,
            price: 1000.0,
        },
    )
    .await
    .unwrap();

    //
    // Place orders and create a position
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 2,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 2,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    // account_0 - place a new bid
    // when user has a long, and market is in reduce only,
    // to reduce incoming asks to reduce position, we ignore existing bids
    let res = send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots: {
                let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
                perp_market.native_price_to_lot(I80F48::from(500))
            },
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();
    // account_1 - place a new ask
    // when user has a short, and market is in reduce only,
    // to reduce incoming bids to reduce position, we ignore existing asks
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();

    //
    // Make market reduce only
    //
    send_tx(
        solana,
        PerpMakeReduceOnly {
            group,
            admin,
            perp_market,
        },
    )
    .await
    .unwrap();

    // account_0 - place a new bid should fail
    let res = send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
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
    .await;
    assert!(res.is_err());

    // account_0 - place a new ask should pass
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();
    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].asks_base_lots, 1);

    // account_0 - place a new ask should pass
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            reduce_only: true,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();
    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].asks_base_lots, 2);

    // account_0 - place a new ask should fail if not reduce_only
    let res = send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
        },
    )
    .await;
    assert!(res.is_err());

    // account_0 - place a new ask should pass but have no effect
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            reduce_only: true,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();
    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].asks_base_lots, 2);

    // account_1 - place a new ask should fail
    let res = send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            reduce_only: false,
            client_order_id: 0,
        },
    )
    .await;
    assert!(res.is_err());

    // account_1 - place a new bid should pass
    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::from(500))
    };
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
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
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    dbg!(mango_account_1.perps[0]);
    assert_eq!(mango_account_1.perps[0].bids_base_lots, 1);

    // account_1 - place a new bid should pass
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
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
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].bids_base_lots, 2);

    // account_1 - place a new bid should fail if reduce only is false
    let res = send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
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
    .await;
    assert!(res.is_err());

    // account_1 - place a new bid should pass but have no effect
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_1,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            reduce_only: true,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();
    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].bids_base_lots, 2);

    Ok(())
}
