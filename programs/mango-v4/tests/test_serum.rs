#![cfg(feature = "test-bpf")]

use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use mango_v4::state::*;
use program_test::*;

mod program_test;

#[tokio::test]
async fn test_serum() -> Result<(), TransportError> {
    let context = TestContext::new(Option::None, Option::None, Option::None, Option::None).await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mint0 = &context.mints[0];
    let mint1 = &context.mints[1];
    let payer_mint_accounts = &context.users[1].token_accounts[0..=2];

    //
    // SETUP: Create serum market
    //
    let serum_market_cookie = context.serum.list_spot_market(mint0, mint1).await;

    //
    // SETUP: Create a group and an account
    //

    let group = send_tx(solana, CreateGroupInstruction { admin, payer })
        .await
        .unwrap()
        .group;

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
    // SETUP: Register mints (and make oracles for them)
    //

    let register_mint = |index: TokenIndex, mint: MintCookie, address_lookup_table: Pubkey| async move {
        let create_stub_oracle_accounts = send_tx(
            solana,
            CreateStubOracle {
                mint: mint.pubkey,
                payer,
            },
        )
        .await
        .unwrap();
        let oracle = create_stub_oracle_accounts.oracle;
        send_tx(
            solana,
            SetStubOracle {
                mint: mint.pubkey,
                payer,
                price: "1.0",
            },
        )
        .await
        .unwrap();
        let register_token_accounts = send_tx(
            solana,
            RegisterTokenInstruction {
                token_index: index,
                decimals: mint.decimals,
                maint_asset_weight: 0.9,
                init_asset_weight: 0.8,
                maint_liab_weight: 1.1,
                init_liab_weight: 1.2,
                group,
                admin,
                mint: mint.pubkey,
                address_lookup_table,
                payer,
            },
        )
        .await
        .unwrap();
        let bank = register_token_accounts.bank;

        (oracle, bank)
    };

    let address_lookup_table = solana.create_address_lookup_table(admin, payer).await;
    let base_token_index = 0;
    let (_oracle0, bank0) =
        register_mint(base_token_index, mint0.clone(), address_lookup_table).await;
    let quote_token_index = 1;
    let (_oracle1, bank1) =
        register_mint(quote_token_index, mint1.clone(), address_lookup_table).await;

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
            base_token_index,
            quote_token_index,
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

    let native0 = account_position(solana, account, bank0).await;
    let native1 = account_position(solana, account, bank1).await;
    assert_eq!(native0, 1000);
    assert_eq!(native1, 900);

    Ok(())
}
