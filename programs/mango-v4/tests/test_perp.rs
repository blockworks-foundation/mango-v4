#![cfg(all(feature = "test-bpf"))]

use anchor_lang::prelude::Pubkey;
use fixed_macro::types::I80F48;
use mango_v4::state::*;
use program_test::*;
use solana_program_test::*;
use solana_sdk::signature::Keypair;

mod program_test;

#[tokio::test]
async fn test_perp() -> Result<(), BanksClientError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..=2];

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

    let account_0 = send_tx(
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

    let account_1 = send_tx(
        solana,
        CreateAccountInstruction {
            account_num: 1,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;

    //
    // SETUP: Deposit user funds
    //
    {
        let deposit_amount = 1000;

        send_tx(
            solana,
            DepositInstruction {
                amount: deposit_amount,
                account: account_0,
                token_account: payer_mint_accounts[0],
                token_authority: payer,
            },
        )
        .await
        .unwrap();

        send_tx(
            solana,
            DepositInstruction {
                amount: deposit_amount,
                account: account_0,
                token_account: payer_mint_accounts[1],
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    {
        let deposit_amount = 1000;

        send_tx(
            solana,
            DepositInstruction {
                amount: deposit_amount,
                account: account_1,
                token_account: payer_mint_accounts[0],
                token_authority: payer,
            },
        )
        .await
        .unwrap();

        send_tx(
            solana,
            DepositInstruction {
                amount: deposit_amount,
                account: account_1,
                token_account: payer_mint_accounts[1],
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    //
    // TEST: Create a perp market
    //
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
            oracle: tokens[0].oracle,
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
            perp_market_index: 0,
            base_token_index: tokens[0].index,
            quote_token_index: tokens[1].index,
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

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48!(1))
    };

    //
    // Place and cancel order with order_id
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
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

    let order_id_to_cancel = solana
        .get_account::<MangoAccount>(account_0)
        .await
        .perps
        .order_id[0];
    send_tx(
        solana,
        PerpCancelOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            owner,
            order_id: order_id_to_cancel,
        },
    )
    .await
    .unwrap();

    assert_no_perp_orders(solana, account_0).await;

    //
    // Place and cancel order with client_order_id
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 1,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpCancelOrderByClientOrderIdInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            owner,
            client_order_id: 1,
        },
    )
    .await
    .unwrap();

    assert_no_perp_orders(solana, account_0).await;

    //
    // Place and cancel all orders
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 2,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 3,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 4,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpCancelAllOrdersInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            owner,
        },
    )
    .await
    .unwrap();

    assert_no_perp_orders(solana, account_0).await;

    //
    // Place a bid, corresponding ask, and consume event
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_0,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 5,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account: account_1,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
            client_order_id: 6,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpConsumeEventsInstruction {
            group,
            perp_market,
            event_queue,
            mango_accounts: vec![account_0, account_1],
        },
    )
    .await
    .unwrap();

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps.accounts[0].base_position_lots, 1);
    assert!(mango_account_0.perps.accounts[0].quote_position_native < -100.019);

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps.accounts[0].base_position_lots, -1);
    assert_eq!(mango_account_1.perps.accounts[0].quote_position_native, 100);

    Ok(())
}

async fn assert_no_perp_orders(solana: &SolanaCookie, account_0: Pubkey) {
    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;

    for i in 0..MAX_PERP_OPEN_ORDERS {
        assert!(mango_account_0.perps.order_id[i] == 0);
        assert!(mango_account_0.perps.order_side[i] == Side::Bid);
        assert!(mango_account_0.perps.client_order_id[i] == 0);
        assert!(mango_account_0.perps.order_market[i] == FREE_ORDER_SLOT);
    }
}
