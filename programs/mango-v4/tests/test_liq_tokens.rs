#![cfg(feature = "test-bpf")]

use fixed::types::I80F48;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use mango_v4::state::*;
use program_test::*;

mod program_test;

#[tokio::test]
async fn test_liq_tokens_force_cancel() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..2];

    //
    // SETUP: Create a group and an account to fill the vaults
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

    // deposit some funds, to the vaults aren't empty
    let vault_account = send_tx(
        solana,
        CreateAccountInstruction {
            account_num: 2,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;
    for &token_account in payer_mint_accounts {
        send_tx(
            solana,
            DepositInstruction {
                amount: 10000,
                account: vault_account,
                token_account,
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    //
    // SETUP: Create serum market
    //
    let serum_market_cookie = context
        .serum
        .list_spot_market(&base_token.mint, &quote_token.mint)
        .await;

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
    // SETUP: Make an account and deposit some quote
    //
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

    let deposit_amount = 1000;
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

    //
    // SETUP: Create an open orders account and an order
    //
    let _open_orders = send_tx(
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

    // short some base
    send_tx(
        solana,
        Serum3PlaceOrderInstruction {
            side: 1,         // TODO: Ask
            limit_price: 10, // in quote_lot (10) per base lot (100)
            max_base_qty: 5, // in base lot (100)
            max_native_quote_qty_including_fees: 600,
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

    //
    // TEST: Change the oracle to make health go negative
    //
    send_tx(
        solana,
        SetStubOracle {
            mint: base_token.mint.pubkey,
            payer,
            price: "10.0",
        },
    )
    .await
    .unwrap();

    // can't withdraw
    assert!(send_tx(
        solana,
        WithdrawInstruction {
            amount: 1,
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint_accounts[1],
        }
    )
    .await
    .is_err());

    //
    // TEST: force cancel orders, making the account healthy again
    //
    send_tx(
        solana,
        Serum3LiqForceCancelOrdersInstruction {
            account,
            serum_market,
            limit: 10,
        },
    )
    .await
    .unwrap();

    // can withdraw again
    send_tx(
        solana,
        WithdrawInstruction {
            amount: 2,
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint_accounts[1],
        },
    )
    .await
    .unwrap();

    Ok(())
}

#[tokio::test]
async fn test_liq_tokens_with_token() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..2];

    //
    // SETUP: Create a group and an account to fill the vaults
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

    // deposit some funds, to the vaults aren't empty
    let vault_account = send_tx(
        solana,
        CreateAccountInstruction {
            account_num: 2,
            group,
            owner,
            payer,
        },
    )
    .await
    .unwrap()
    .account;
    for &token_account in payer_mint_accounts {
        send_tx(
            solana,
            DepositInstruction {
                amount: 10000,
                account: vault_account,
                token_account,
                token_authority: payer,
            },
        )
        .await
        .unwrap();
    }

    //
    // SETUP: Make an account and deposit some quote, borrow some base
    //
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

    let deposit_amount = 1000;
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

    let withdraw_amount = 400;
    send_tx(
        solana,
        WithdrawInstruction {
            amount: withdraw_amount,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_mint_accounts[0],
        },
    )
    .await
    .unwrap();

    //
    // TEST: Change the oracle to make health go negative, then liquidate
    //
    send_tx(
        solana,
        SetStubOracle {
            mint: base_token.mint.pubkey,
            payer,
            price: "2.0",
        },
    )
    .await
    .unwrap();

    // first limited liq
    send_tx(
        solana,
        LiqTokenWithTokenInstruction {
            liqee: account,
            liqor: vault_account,
            liqor_owner: owner,
            asset_token_index: quote_token.index,
            liab_token_index: base_token.index,
            max_liab_transfer: I80F48::from_num(10.0),
        },
    )
    .await
    .unwrap();

    // the asset cost for 10 liab is 10 * 2 * 1.04 = 20.8
    assert_eq!(
        account_position(solana, account, base_token.bank).await,
        -400 + 10
    );
    assert_eq!(
        account_position(solana, account, quote_token.bank).await,
        1000 - 21
    );
    let liqee: MangoAccount = solana.get_account(account).await;
    assert!(liqee.being_liquidated);

    // remaining liq
    send_tx(
        solana,
        LiqTokenWithTokenInstruction {
            liqee: account,
            liqor: vault_account,
            liqor_owner: owner,
            asset_token_index: quote_token.index,
            liab_token_index: base_token.index,
            max_liab_transfer: I80F48::from_num(10000.0),
        },
    )
    .await
    .unwrap();

    // health before liquidation was 1000 * 0.6 - 400 * 1.4 = -520
    // liab needed 520 / (1.4*2 - 0.6*2*(1 + 0.02 + 0.02)) = 335.0515
    // asset cost = 335 * 2 * 1.04 = 696.8
    assert_eq!(
        account_position(solana, account, base_token.bank).await,
        -400 + 335
    );
    assert_eq!(
        account_position(solana, account, quote_token.bank).await,
        1000 - 697
    );
    let liqee: MangoAccount = solana.get_account(account).await;
    assert!(!liqee.being_liquidated);

    Ok(())
}
