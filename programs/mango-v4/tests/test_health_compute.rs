#![cfg(feature = "test-bpf")]

use solana_program_test::*;
use solana_sdk::signature::Keypair;

use program_test::*;

mod program_test;

// Try to reach compute limits in health checks by having many different tokens in an account
#[tokio::test]
async fn test_health_compute_tokens() -> Result<(), BanksClientError> {
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
    // On 2022-3-29 the final deposit costs 43900 CU and each new token increases it by roughly 1800 CU

    Ok(())
}

// Try to reach compute limits in health checks by having many serum markets in an account
#[tokio::test]
async fn test_health_compute_serum() -> Result<(), BanksClientError> {
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
                token_account: payer_mint_accounts[i],
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    // TODO: actual explicit CU comparisons.
    // On 2022-3-29 the final deposit costs 60380 CU and each new market increases it by roughly 5000 CU

    Ok(())
}
