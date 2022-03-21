#![cfg(feature = "test-bpf")]

use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use mango_v4::state::*;
use program_test::*;

mod program_test;

#[tokio::test]
async fn test_serum() -> Result<(), TransportError> {
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
    let base_token = &tokens[0];
    let quote_token = &tokens[1];

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
    // SETUP: Create serum market
    //
    let serum_market_cookie = context
        .serum
        .list_spot_market(&base_token.mint, &quote_token.mint)
        .await;

    //
    // SETUP: Deposit user funds
    //
    {
        let deposit_amount = 1000;

        send_tx(
            solana,
            DepositInstruction {
                amount: deposit_amount,
                account,
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
                account,
                token_account: payer_mint_accounts[1],
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    //
    // TEST: Register a serum market
    //
    let serum_market = send_tx(
        solana,
        Serum3RegisterMarketInstruction {
            group,
            admin,
            serum_program: context.serum.program_id,
            serum_market_external: serum_market_cookie.market,
            market_index: 0,
            base_token_index: base_token.index,
            quote_token_index: quote_token.index,
            payer,
        },
    )
    .await
    .unwrap()
    .serum_market;

    //
    // TEST: Create an open orders account
    //
    let open_orders = send_tx(
        solana,
        Serum3CreateOpenOrdersInstruction {
            account,
            serum_market,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .open_orders;

    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(
        account_data
            .serum3_account_map
            .iter_active()
            .map(|v| (v.open_orders, v.market_index))
            .collect::<Vec<_>>(),
        [(open_orders, 0)]
    );

    //
    // TEST: Place an order
    //
    send_tx(
        solana,
        Serum3PlaceOrderInstruction {
            side: 0,         // TODO: Bid
            limit_price: 10, // in quote_lot (10) per base lot (100)
            max_base_qty: 1, // in base lot (100)
            max_native_quote_qty_including_fees: 100,
            self_trade_behavior: 0,
            order_type: 0, // TODO: Limit
            client_order_id: 0,
            limit: 10,
            account,
            owner,
            serum_market,
        },
    )
    .await
    .unwrap();

    let native0 = account_position(solana, account, base_token.bank).await;
    let native1 = account_position(solana, account, quote_token.bank).await;
    assert_eq!(native0, 1000);
    assert_eq!(native1, 900);

    // get the order id
    let open_orders_bytes = solana.get_account_data(open_orders).await.unwrap();
    let open_orders_data: &serum_dex::state::OpenOrders = bytemuck::from_bytes(
        &open_orders_bytes[5..5 + std::mem::size_of::<serum_dex::state::OpenOrders>()],
    );
    let order_id = open_orders_data.orders[0];
    assert!(order_id != 0);

    //
    // TEST: Cancel the order
    //
    send_tx(
        solana,
        Serum3CancelOrderInstruction {
            side: 0,
            order_id,
            account,
            owner,
            serum_market,
        },
    )
    .await
    .unwrap();

    //
    // TEST: Settle, moving the freed up funds back
    //
    send_tx(
        solana,
        Serum3SettleFundsInstruction {
            account,
            owner,
            serum_market,
        },
    )
    .await
    .unwrap();

    let native0 = account_position(solana, account, base_token.bank).await;
    let native1 = account_position(solana, account, quote_token.bank).await;
    assert_eq!(native0, 1000);
    assert_eq!(native1, 1000);

    Ok(())
}
