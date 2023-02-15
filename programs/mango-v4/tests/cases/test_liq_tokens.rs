use super::*;

use mango_v4::accounts_ix::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};

#[tokio::test]
async fn test_liq_tokens_force_cancel() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(95_000); // Serum3PlaceOrder needs 92.8k
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..2];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let base_token = &tokens[0];
    let quote_token = &tokens[1];

    // deposit some funds, to the vaults aren't empty
    create_funded_account(&solana, group, owner, 0, &context.users[1], mints, 10000, 0).await;

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
    let deposit_amount = 1000;
    let account = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        &mints[1..2],
        deposit_amount,
        0,
    )
    .await;

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
    set_bank_stub_oracle_price(solana, group, base_token, admin, 10.0).await;

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
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(85_000); // LiqTokenWithToken needs 79k
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..4];
    let payer_mint_accounts = &context.users[1].token_accounts[0..4];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
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
        AccountCreateInstruction {
            account_num: 2,
            token_count: 16,
            serum3_count: 8,
            perp_count: 8,
            perp_oo_count: 8,
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
                reduce_only: false,
                account: vault_account,
                owner,
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
        AccountCreateInstruction {
            account_num: 0,
            token_count: 16,
            serum3_count: 8,
            perp_count: 8,
            perp_oo_count: 8,
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
            reduce_only: false,
            account,
            owner,
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
            reduce_only: false,
            account,
            owner,
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
    set_bank_stub_oracle_price(solana, group, borrow_token1, admin, 2.0).await;

    //
    // TEST: liquidate borrow2 against too little collateral2
    //

    send_tx(
        solana,
        TokenLiqWithTokenInstruction {
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
    assert!(account_position_closed(solana, account, collateral_token2.bank).await,);
    let liqee = get_mango_account(solana, account).await;
    assert!(liqee.being_liquidated());

    //
    // TEST: liquidate the remaining borrow2 against collateral1,
    // bringing the borrow2 balance to 0 but keeping account health negative
    //
    send_tx(
        solana,
        TokenLiqWithTokenInstruction {
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
    assert!(account_position_closed(solana, account, borrow_token2.bank).await);
    assert_eq!(
        account_position(solana, account, collateral_token1.bank).await,
        1000 - 32
    );
    let liqee = get_mango_account(solana, account).await;
    assert!(liqee.being_liquidated());

    //
    // TEST: liquidate borrow1 with collateral1, but place a limit
    //
    send_tx(
        solana,
        TokenLiqWithTokenInstruction {
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
    let liqee = get_mango_account(solana, account).await;
    assert!(liqee.being_liquidated());

    //
    // TEST: liquidate borrow1 with collateral1, making the account healthy again
    //
    send_tx(
        solana,
        TokenLiqWithTokenInstruction {
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
    let liqee = get_mango_account(solana, account).await;
    assert!(!liqee.being_liquidated());

    //
    // TEST: bankruptcy when collateral is dusted
    //

    // Setup: make collateral really valueable, remove nearly all of it
    set_bank_stub_oracle_price(solana, group, collateral_token1, admin, 100000.0).await;
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: (account_position(solana, account, collateral_token1.bank).await) as u64 - 1,
            allow_borrow: false,
            account,
            owner,
            token_account: payer_mint_accounts[2],
            bank_index: 0,
        },
    )
    .await
    .unwrap();
    // Setup: reduce collateral value to trigger liquidatability
    // We have -93 borrows, so -93*2*1.4 = -260.4 health from that
    // And 1-2 collateral, so max 2*0.6*X health; say X=150 for max 180 health
    set_bank_stub_oracle_price(solana, group, collateral_token1, admin, 150.0).await;

    send_tx(
        solana,
        TokenLiqWithTokenInstruction {
            liqee: account,
            liqor: vault_account,
            liqor_owner: owner,
            asset_token_index: collateral_token1.index,
            liab_token_index: borrow_token1.index,
            max_liab_transfer: I80F48::from_num(10001.0),
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Liqee's remaining collateral got dusted, only borrows remain
    // but the borrow amount is so tiny, that being_liquidated is already switched off
    let liqee = get_mango_account(solana, account).await;
    assert_eq!(liqee.active_token_positions().count(), 1);
    assert!(account_position_f64(solana, account, borrow_token1.bank).await > -1.0);
    assert!(!liqee.being_liquidated());

    Ok(())
}
