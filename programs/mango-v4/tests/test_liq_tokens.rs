#![cfg(feature = "test-bpf")]

use fixed::types::I80F48;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use mango_v4::{
    instructions::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side},
    state::*,
};
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
            TokenDepositInstruction {
                amount: 10000,
                account: vault_account,
                token_account,
                token_authority: payer.clone(),
                bank_index: 0,
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
            base_bank: base_token.bank,
            quote_bank: quote_token.bank,
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
        TokenDepositInstruction {
            amount: deposit_amount,
            account,
            token_account: payer_mint_accounts[1],
            token_authority: payer.clone(),
            bank_index: 0,
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
            side: Serum3Side::Ask,
            limit_price: 10, // in quote_lot (10) per base lot (100)
            max_base_qty: 5, // in base lot (100)
            max_native_quote_qty_including_fees: 600,
            self_trade_behavior: Serum3SelfTradeBehavior::DecrementTake,
            order_type: Serum3OrderType::Limit,
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
        SetStubOracleInstruction {
            group,
            admin,
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
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint_accounts[1],
            bank_index: 0,
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
        TokenWithdrawInstruction {
            amount: 2,
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint_accounts[1],
            bank_index: 0,
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
    let mints = &context.mints[0..4];
    let payer_mint_accounts = &context.users[1].token_accounts[0..4];

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
    let borrow_token1 = &tokens[0];
    let borrow_token2 = &tokens[1];
    let collateral_token1 = &tokens[2];
    let collateral_token2 = &tokens[3];

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
            TokenDepositInstruction {
                amount: 100000,
                account: vault_account,
                token_account,
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();
    }

    //
    // SETUP: Make an account with some collateral and some borrows
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

    let deposit1_amount = 1000;
    let deposit2_amount = 20;
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: deposit1_amount,
            account,
            token_account: payer_mint_accounts[2],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: deposit2_amount,
            account,
            token_account: payer_mint_accounts[3],
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    let borrow1_amount = 350;
    let borrow2_amount = 50;
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: borrow1_amount,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_mint_accounts[0],
            bank_index: 0,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: borrow2_amount,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_mint_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Change the oracle to make health go negative
    //
    send_tx(
        solana,
        SetStubOracleInstruction {
            group,
            admin,
            mint: borrow_token1.mint.pubkey,
            payer,
            price: "2.0",
        },
    )
    .await
    .unwrap();

    //
    // TEST: liquidate borrow2 against too little collateral2
    //

    send_tx(
        solana,
        LiqTokenWithTokenInstruction {
            liqee: account,
            liqor: vault_account,
            liqor_owner: owner,
            asset_token_index: collateral_token2.index,
            liab_token_index: borrow_token2.index,
            asset_bank_index: 0,
            liab_bank_index: 0,
            max_liab_transfer: I80F48::from_num(10000.0),
        },
    )
    .await
    .unwrap();

    // the we only have 20 collateral2, and can trade them for 20 / 1.04 = 19.2 borrow2
    assert_eq!(
        account_position(solana, account, borrow_token2.bank).await,
        -50 + 19
    );
    assert_eq!(
        account_position(solana, account, collateral_token2.bank).await,
        0
    );
    let liqee: MangoAccount = solana.get_account(account).await;
    assert!(liqee.being_liquidated());
    assert!(!liqee.is_bankrupt());

    //
    // TEST: liquidate the remaining borrow2 against collateral1,
    // bringing the borrow2 balance to 0 but keeping account health negative
    //
    send_tx(
        solana,
        LiqTokenWithTokenInstruction {
            liqee: account,
            liqor: vault_account,
            liqor_owner: owner,
            asset_token_index: collateral_token1.index,
            liab_token_index: borrow_token2.index,
            max_liab_transfer: I80F48::from_num(10000.0),
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .unwrap();

    // the asset cost for 50-19=31 borrow2 is 31 * 1.04 = 32.24
    assert_eq!(
        account_position(solana, account, borrow_token2.bank).await,
        0
    );
    assert_eq!(
        account_position(solana, account, collateral_token1.bank).await,
        1000 - 32
    );
    let liqee: MangoAccount = solana.get_account(account).await;
    assert!(liqee.being_liquidated());
    assert!(!liqee.is_bankrupt());

    //
    // TEST: liquidate borrow1 with collateral1, but place a limit
    //
    send_tx(
        solana,
        LiqTokenWithTokenInstruction {
            liqee: account,
            liqor: vault_account,
            liqor_owner: owner,
            asset_token_index: collateral_token1.index,
            liab_token_index: borrow_token1.index,
            max_liab_transfer: I80F48::from_num(10.0),
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .unwrap();

    // the asset cost for 10 borrow1 is 10 * 2 * 1.04 = 20.8
    assert_eq!(
        account_position(solana, account, borrow_token1.bank).await,
        -350 + 10
    );
    assert_eq!(
        account_position(solana, account, collateral_token1.bank).await,
        1000 - 32 - 21
    );
    let liqee: MangoAccount = solana.get_account(account).await;
    assert!(liqee.being_liquidated());
    assert!(!liqee.is_bankrupt());

    //
    // TEST: liquidate borrow1 with collateral1, making the account healthy again
    //
    send_tx(
        solana,
        LiqTokenWithTokenInstruction {
            liqee: account,
            liqor: vault_account,
            liqor_owner: owner,
            asset_token_index: collateral_token1.index,
            liab_token_index: borrow_token1.index,
            max_liab_transfer: I80F48::from_num(10000.0),
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .unwrap();

    // health after borrow2 liquidation was (1000-32) * 0.6 - 350 * 2 * 1.4 = -399.2
    // borrow1 needed 399.2 / (1.4*2 - 0.6*2*1.04) = 257.2
    // asset cost = 257.2 * 2 * 1.04 = 535
    // loan orignation fee = 1
    assert_eq!(
        account_position(solana, account, borrow_token1.bank).await,
        -350 + 257
    );
    assert_eq!(
        account_position(solana, account, collateral_token1.bank).await,
        1000 - 32 - 535 - 1
    );
    let liqee: MangoAccount = solana.get_account(account).await;
    assert!(!liqee.being_liquidated());
    assert!(!liqee.is_bankrupt());

    Ok(())
}
