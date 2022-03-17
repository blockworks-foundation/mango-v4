#![cfg(feature = "test-bpf")]

use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use mango_v4::state::*;
use program_test::*;

mod program_test;

// Try to reach compute limits in health checks by having many different tokens in an account
#[tokio::test]
async fn test_health_compute_tokens() -> Result<(), TransportError> {
    let context = TestContext::new(Option::None, Option::None, Option::None, Option::None).await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..10];
    let payer_mint_accounts = &context.users[1].token_accounts[0..mints.len()];

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
    for mint in mints {
        register_mint(mint.index as u16, mint.clone(), address_lookup_table).await;
    }

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
    // On 2022-3-17 the final deposit costs 51010 CU and each new token increases it by roughly 2500 CU

    Ok(())
}

// Try to reach compute limits in health checks by having many serum markets in an account
#[tokio::test]
async fn test_health_compute_serum() -> Result<(), TransportError> {
    let context = TestContext::new(Option::None, Option::None, Option::None, Option::None).await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..8];
    let payer_mint_accounts = &context.users[1].token_accounts[0..mints.len()];

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
    for mint in mints {
        register_mint(mint.index as u16, mint.clone(), address_lookup_table).await;
    }

    //
    // SETUP: Create serum markets and register them
    //
    let quote_mint = &mints[0];
    let mut serum_market_cookies = vec![];
    for mint in mints[1..].iter() {
        serum_market_cookies.push(context.serum.list_spot_market(mint, quote_mint).await);
    }

    let mut serum_markets = vec![];
    for s in serum_market_cookies {
        serum_markets.push(
            send_tx(
                solana,
                RegisterSerumMarketInstruction {
                    group,
                    admin,
                    serum_program: context.serum.program_id,
                    serum_market_external: s.market,
                    market_index: s.coin_mint.index as u16,
                    base_token_index: s.coin_mint.index as u16,
                    quote_token_index: s.pc_mint.index as u16,
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
            CreateSerumOpenOrdersInstruction {
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
                token_account: payer_mint_accounts[i],
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    // TODO: actual explicit CU comparisons.
    // On 2022-3-17 the final deposit costs 71593 CU and each new market increases it by roughly 6000 CU

    Ok(())
}
