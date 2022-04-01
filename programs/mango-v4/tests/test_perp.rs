// TODO: Test disabled since it fails
#![cfg(all(feature = "test-bpf", feature = "disabled-perp-test"))]

use mango_v4::state::*;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use program_test::*;

mod program_test;

#[tokio::test]
async fn test_perp() -> Result<(), TransportError> {
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
            init_asset_weight: 0.95,
            maint_asset_weight: 0.975,
            init_liab_weight: 1.05,
            maint_liab_weight: 1.025,
            liquidation_fee: 0.012,
            maker_fee: 0.0002,
            taker_fee: 0.000,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Bid,
            price: 1,
            quantity: 1,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            group,
            account,
            perp_market,
            asks,
            bids,
            event_queue,
            oracle: tokens[0].oracle,
            owner,
            side: Side::Ask,
            price: 1,
            quantity: 1,
        },
    )
    .await
    .unwrap();

    Ok(())
}
