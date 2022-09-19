#![cfg(all(feature = "test-bpf"))]

use fixed::types::I80F48;
use mango_v4::{error::MangoError, state::*};
use program_test::*;
use solana_program_test::*;
use solana_sdk::transport::TransportError;

mod program_test;

#[tokio::test]
async fn test_perp_settle_pnl() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..=2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..=2];

    let initial_token_deposit0 = 10_000;
    // only deposited because perps currently require the base token position to be active
    let initial_token_deposit1 = 1;

    //
    // SETUP: Create a group and an account
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;

    let account_0 = send_tx(
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

    let account_1 = send_tx(
        solana,
        AccountCreateInstruction {
            account_num: 1,
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

    //
    // SETUP: Deposit user funds
    //
    {
        send_tx(
            solana,
            TokenDepositInstruction {
                amount: initial_token_deposit0,
                account: account_0,
                token_account: payer_mint_accounts[0],
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        send_tx(
            solana,
            TokenDepositInstruction {
                amount: initial_token_deposit1,
                account: account_0,
                token_account: payer_mint_accounts[1],
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();
    }

    {
        send_tx(
            solana,
            TokenDepositInstruction {
                amount: initial_token_deposit0,
                account: account_1,
                token_account: payer_mint_accounts[0],
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        send_tx(
            solana,
            TokenDepositInstruction {
                amount: initial_token_deposit1,
                account: account_1,
                token_account: payer_mint_accounts[1],
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();
    }

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
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[1]).await
        },
    )
    .await
    .unwrap();

    //
    // TEST: Create another perp market
    //
    let mango_v4::accounts::PerpCreateMarket {
        perp_market: perp_market_2,
        ..
    } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 1,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_asset_weight: 0.975,
            init_asset_weight: 0.95,
            maint_liab_weight: 1.025,
            init_liab_weight: 1.05,
            liquidation_fee: 0.012,
            maker_fee: 0.0002,
            taker_fee: 0.000,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[2]).await
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
            payer,
            price: "1000.0",
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
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
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
            max_base_lots: 1,
            max_quote_lots: i64::MAX,
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

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;

        assert_eq!(mango_account_0.perps[0].base_position_lots(), 1);
        assert_eq!(mango_account_1.perps[0].base_position_lots(), -1);
        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            -100_020
        );
        assert_eq!(mango_account_1.perps[0].quote_position_native(), 100_000);
    }

    // Bank must be valid for quote currency
    let result = send_tx(
        solana,
        PerpSettlePnlInstruction {
            account_a: account_1,
            account_b: account_0,
            perp_market,
            quote_bank: tokens[1].bank,
            max_settle_amount: u64::MAX,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::InvalidBank.into(),
        "Bank must be valid for quote currency".to_string(),
    );

    // Cannot settle with yourself
    let result = send_tx(
        solana,
        PerpSettlePnlInstruction {
            account_a: account_0,
            account_b: account_0,
            perp_market,
            quote_bank: tokens[0].bank,
            max_settle_amount: u64::MAX,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::CannotSettleWithSelf.into(),
        "Cannot settle with yourself".to_string(),
    );

    // Cannot settle position that does not exist
    let result = send_tx(
        solana,
        PerpSettlePnlInstruction {
            account_a: account_0,
            account_b: account_1,
            perp_market: perp_market_2,
            quote_bank: tokens[0].bank,
            max_settle_amount: u64::MAX,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::PerpPositionDoesNotExist.into(),
        "Cannot settle a position that does not exist".to_string(),
    );

    // max_settle_amount must be greater than zero
    let result = send_tx(
        solana,
        PerpSettlePnlInstruction {
            account_a: account_0,
            account_b: account_1,
            perp_market: perp_market,
            quote_bank: tokens[0].bank,
            max_settle_amount: 0,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::MaxSettleAmountMustBeGreaterThanZero.into(),
        "max_settle_amount must be greater than zero".to_string(),
    );

    // TODO: Test funding settlement

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        assert_eq!(
            mango_account_0.tokens[0].native(&bank).round(),
            initial_token_deposit0,
            "account 0 has expected amount of tokens"
        );
        assert_eq!(
            mango_account_1.tokens[0].native(&bank).round(),
            initial_token_deposit0,
            "account 1 has expected amount of tokens"
        );
    }

    // Try and settle with high price
    send_tx(
        solana,
        StubOracleSetInstruction {
            group,
            admin,
            mint: mints[1].pubkey,
            payer,
            price: "1200.0",
        },
    )
    .await
    .unwrap();

    // Account a must be the profitable one
    let result = send_tx(
        solana,
        PerpSettlePnlInstruction {
            account_a: account_1,
            account_b: account_0,
            perp_market,
            quote_bank: tokens[0].bank,
            max_settle_amount: u64::MAX,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::ProfitabilityMismatch.into(),
        "Account a must be the profitable one".to_string(),
    );

    // Change the oracle to a more reasonable price
    send_tx(
        solana,
        StubOracleSetInstruction {
            group,
            admin,
            mint: mints[1].pubkey,
            payer,
            price: "1005.0",
        },
    )
    .await
    .unwrap();

    let expected_pnl_0 = I80F48::from(480); // Less due to fees
    let expected_pnl_1 = I80F48::from(-500);

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            get_pnl_native(&mango_account_0.perps[0], &perp_market, I80F48::from(1005)).round(),
            expected_pnl_0
        );
        assert_eq!(
            get_pnl_native(&mango_account_1.perps[0], &perp_market, I80F48::from(1005)),
            expected_pnl_1
        );
    }

    // Partially execute the settle
    let partial_settle_amount = 200;
    send_tx(
        solana,
        PerpSettlePnlInstruction {
            account_a: account_0,
            account_b: account_1,
            perp_market,
            quote_bank: tokens[0].bank,
            max_settle_amount: partial_settle_amount,
        },
    )
    .await
    .unwrap();

    {
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;

        assert_eq!(
            mango_account_0.perps[0].base_position_lots(),
            1,
            "base position unchanged for account 0"
        );
        assert_eq!(
            mango_account_1.perps[0].base_position_lots(),
            -1,
            "base position unchanged for account 1"
        );

        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            I80F48::from(-100_020) - I80F48::from(partial_settle_amount),
            "quote position reduced for profitable position by max_settle_amount"
        );
        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000 + partial_settle_amount),
            "quote position increased for losing position by opposite of first account"
        );

        assert_eq!(
            mango_account_0.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit0 + partial_settle_amount),
            "account 0 token native position increased (profit) by max_settle_amount"
        );
        assert_eq!(
            mango_account_1.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit0) - I80F48::from(partial_settle_amount),
            "account 1 token native position decreased (loss) by max_settle_amount"
        );

        assert_eq!(
            mango_account_0.net_settled, partial_settle_amount as i64,
            "net_settled on account 0 updated with profit from settlement"
        );
        assert_eq!(
            mango_account_1.net_settled,
            -(partial_settle_amount as i64),
            "net_settled on account 1 updated with loss from settlement"
        );
    }

    // Change the oracle to a very high price, such that the pnl exceeds the account funding
    send_tx(
        solana,
        StubOracleSetInstruction {
            group,
            admin,
            mint: mints[1].pubkey,
            payer,
            price: "1500.0",
        },
    )
    .await
    .unwrap();

    let expected_pnl_0 = I80F48::from(50000 - 20 - partial_settle_amount as i64);
    let expected_pnl_1 = I80F48::from(-50000 + partial_settle_amount as i64);

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            get_pnl_native(&mango_account_0.perps[0], &perp_market, I80F48::from(1500)).round(),
            expected_pnl_0
        );
        assert_eq!(
            get_pnl_native(&mango_account_1.perps[0], &perp_market, I80F48::from(1500)),
            expected_pnl_1
        );
    }

    // Settle as much PNL as account_1's health allows
    let account_1_health_non_perp = I80F48::from(9040); // 0.8 * (10000-200+1*1500)
    let expected_total_settle = I80F48::from(partial_settle_amount) + account_1_health_non_perp;
    send_tx(
        solana,
        PerpSettlePnlInstruction {
            account_a: account_0,
            account_b: account_1,
            perp_market,
            quote_bank: tokens[0].bank,
            max_settle_amount: u64::MAX,
        },
    )
    .await
    .unwrap();

    {
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;

        assert_eq!(
            mango_account_0.perps[0].base_position_lots(),
            1,
            "base position unchanged for account 0"
        );
        assert_eq!(
            mango_account_1.perps[0].base_position_lots(),
            -1,
            "base position unchanged for account 1"
        );

        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            I80F48::from(-100_020) - expected_total_settle,
            "quote position reduced for profitable position"
        );
        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000) + expected_total_settle,
            "quote position increased for losing position by opposite of first account"
        );

        assert_eq!(
            mango_account_0.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit0) + expected_total_settle,
            "account 0 token native position increased (profit)"
        );
        assert_eq!(
            mango_account_1.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit0) - expected_total_settle,
            "account 1 token native position decreased (loss)"
        );

        assert_eq!(
            mango_account_0.net_settled, expected_total_settle,
            "net_settled on account 0 updated with profit from settlement"
        );
        assert_eq!(
            mango_account_1.net_settled, -expected_total_settle,
            "net_settled on account 1 updated with loss from settlement"
        );
    }

    // Change the oracle to a reasonable price in other direction
    send_tx(
        solana,
        StubOracleSetInstruction {
            group,
            admin,
            mint: mints[1].pubkey,
            payer,
            price: "995.0",
        },
    )
    .await
    .unwrap();

    let expected_pnl_0 = I80F48::from(-9760);
    let expected_pnl_1 = I80F48::from(9740);

    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            get_pnl_native(&mango_account_0.perps[0], &perp_market, I80F48::from(995)).round(),
            expected_pnl_0
        );
        assert_eq!(
            get_pnl_native(&mango_account_1.perps[0], &perp_market, I80F48::from(995)).round(),
            expected_pnl_1
        );
    }

    // Fully execute the settle
    let expected_total_settle = expected_total_settle - expected_pnl_1;
    send_tx(
        solana,
        PerpSettlePnlInstruction {
            account_a: account_1,
            account_b: account_0,
            perp_market,
            quote_bank: tokens[0].bank,
            max_settle_amount: u64::MAX,
        },
    )
    .await
    .unwrap();

    {
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;

        assert_eq!(
            mango_account_0.perps[0].base_position_lots(),
            1,
            "base position unchanged for account 0"
        );
        assert_eq!(
            mango_account_1.perps[0].base_position_lots(),
            -1,
            "base position unchanged for account 1"
        );

        assert_eq!(
            mango_account_0.perps[0].quote_position_native().round(),
            I80F48::from(-100_020) - expected_total_settle,
            "quote position increased for losing position"
        );
        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000) + expected_total_settle,
            "quote position reduced for losing position by opposite of first account"
        );

        // 480 was previous settlement
        assert_eq!(
            mango_account_0.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit0) + expected_total_settle,
            "account 0 token native position decreased (loss)"
        );
        assert_eq!(
            mango_account_1.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit0) - expected_total_settle,
            "account 1 token native position increased (profit)"
        );

        assert_eq!(
            mango_account_0.net_settled, expected_total_settle,
            "net_settled on account 0 updated with loss from settlement"
        );
        assert_eq!(
            mango_account_1.net_settled, -expected_total_settle,
            "net_settled on account 1 updated with profit from settlement"
        );
    }

    // no more settleable pnl left
    {
        let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            get_pnl_native(&mango_account_0.perps[0], &perp_market, I80F48::from(995)).round(),
            -20 // fees
        );
        assert_eq!(
            get_pnl_native(&mango_account_1.perps[0], &perp_market, I80F48::from(995)).round(),
            0
        );
    }

    Ok(())
}
