#![cfg(all(feature = "test-bpf"))]

use fixed::types::I80F48;
use mango_setup::*;
use mango_v4::{error::MangoError, state::*};
use program_test::*;
use solana_program_test::*;
use solana_sdk::transport::TransportError;

mod program_test;

#[tokio::test]
async fn test_perp_settle_fees() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint_accounts = &context.users[1].token_accounts[0..=2];

    let initial_token_deposit = 10_000;

    //
    // SETUP: Create a group and an account
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
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
        let deposit_amount = initial_token_deposit;

        send_tx(
            solana,
            TokenDepositInstruction {
                amount: deposit_amount,
                account: account_0,
                owner,
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
                amount: deposit_amount,
                account: account_0,
                owner,
                token_account: payer_mint_accounts[1],
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();
    }

    {
        let deposit_amount = initial_token_deposit;

        send_tx(
            solana,
            TokenDepositInstruction {
                amount: deposit_amount,
                account: account_1,
                owner,
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
                amount: deposit_amount,
                account: account_1,
                owner,
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
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[0]).await
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
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[1]).await
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
            mint: mints[0].pubkey,
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

    let mango_account_0 = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(mango_account_0.perps[0].base_position_lots(), 1);
    assert_eq!(
        mango_account_0.perps[0].quote_position_native().round(),
        -100_020
    );

    let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(mango_account_1.perps[0].base_position_lots(), -1);
    assert_eq!(mango_account_1.perps[0].quote_position_native(), 100_000);

    // Bank must be valid for quote currency
    let result = send_tx(
        solana,
        PerpSettleFeesInstruction {
            account: account_0,
            perp_market,
            settle_bank: tokens[1].bank,
            max_settle_amount: u64::MAX,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::InvalidBank.into(),
        "Bank must be valid for quote currency".to_string(),
    );

    // Cannot settle position that does not exist
    let result = send_tx(
        solana,
        PerpSettleFeesInstruction {
            account: account_1,
            perp_market: perp_market_2,
            settle_bank: tokens[0].bank,
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
        PerpSettleFeesInstruction {
            account: account_1,
            perp_market: perp_market,
            settle_bank: tokens[0].bank,
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
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        assert_eq!(
            mango_account_0.tokens[0].native(&bank).round(),
            initial_token_deposit,
            "account 0 has expected amount of tokens"
        );
        assert_eq!(
            mango_account_1.tokens[0].native(&bank).round(),
            initial_token_deposit,
            "account 1 has expected amount of tokens"
        );
    }

    // Try and settle with high price
    send_tx(
        solana,
        StubOracleSetInstruction {
            group,
            admin,
            mint: mints[0].pubkey,
            payer,
            price: "1200.0",
        },
    )
    .await
    .unwrap();

    // Account must have a loss
    let result = send_tx(
        solana,
        PerpSettleFeesInstruction {
            account: account_0,
            perp_market,
            settle_bank: tokens[0].bank,
            max_settle_amount: u64::MAX,
        },
    )
    .await;

    assert_mango_error(
        &result,
        MangoError::ProfitabilityMismatch.into(),
        "Account must be unprofitable".to_string(),
    );

    // TODO: Difficult to test health due to fees being so small. Need alternative
    // let result = send_tx(
    //     solana,
    //     PerpSettleFeesInstruction {
    //         group,
    //         account: account_1,
    //         perp_market,
    //         oracle: tokens[0].oracle,
    //         settle_bank: tokens[0].bank,
    //         max_settle_amount: I80F48::MAX,
    //     },
    // )
    // .await;

    // assert_mango_error(
    //     &result,
    //     MangoError::HealthMustBePositive.into(),
    //     "Health of losing account must be positive to settle".to_string(),
    // );

    // Change the oracle to a more reasonable price
    send_tx(
        solana,
        StubOracleSetInstruction {
            group,
            admin,
            mint: mints[0].pubkey,
            payer,
            price: "1005.0",
        },
    )
    .await
    .unwrap();

    let expected_pnl_0 = I80F48::from(480); // Less due to fees
    let expected_pnl_1 = I80F48::from(-500);

    {
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

    // Check the fees accrued
    let initial_fees = I80F48::from(20);
    {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        assert_eq!(
            perp_market.fees_accrued.round(),
            initial_fees,
            "Fees from trading have been accrued"
        );
        assert_eq!(
            perp_market.fees_settled.round(),
            0,
            "No fees have been settled yet"
        );
    }

    // Partially execute the settle
    let partial_settle_amount = 10;
    send_tx(
        solana,
        PerpSettleFeesInstruction {
            account: account_1,
            perp_market,
            settle_bank: tokens[0].bank,
            max_settle_amount: partial_settle_amount,
        },
    )
    .await
    .unwrap();

    {
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;

        assert_eq!(
            mango_account_1.perps[0].base_position_lots(),
            -1,
            "base position unchanged for account 1"
        );

        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000 + partial_settle_amount),
            "quote position increased for losing position by fee settle amount"
        );

        assert_eq!(
            mango_account_1.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit - partial_settle_amount),
            "account 1 token native position decreased (loss) by max_settle_amount"
        );

        assert_eq!(
            mango_account_1.net_settled,
            -(partial_settle_amount as i64),
            "net_settled on account 1 updated with loss from settlement"
        );

        assert_eq!(
            perp_market.fees_accrued.round(),
            initial_fees - I80F48::from(partial_settle_amount),
            "Fees accrued have been reduced by partial settle"
        );
        assert_eq!(
            perp_market.fees_settled.round(),
            partial_settle_amount,
            "Fees have been partially settled"
        );
    }

    // Fully execute the settle
    send_tx(
        solana,
        PerpSettleFeesInstruction {
            account: account_1,
            perp_market,
            settle_bank: tokens[0].bank,
            max_settle_amount: u64::MAX,
        },
    )
    .await
    .unwrap();

    {
        let bank = solana.get_account::<Bank>(tokens[0].bank).await;
        let mango_account_1 = solana.get_account::<MangoAccount>(account_1).await;
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;

        assert_eq!(
            mango_account_1.perps[0].base_position_lots(),
            -1,
            "base position unchanged for account 1"
        );

        assert_eq!(
            mango_account_1.perps[0].quote_position_native().round(),
            I80F48::from(100_000) + initial_fees,
            "quote position increased for losing position by fees settled"
        );

        assert_eq!(
            mango_account_1.tokens[0].native(&bank).round(),
            I80F48::from(initial_token_deposit) - initial_fees,
            "account 1 token native position decreased (loss)"
        );

        assert_eq!(
            mango_account_1.net_settled, -initial_fees,
            "net_settled on account 1 updated with loss from settlement"
        );

        assert_eq!(
            perp_market.fees_accrued.round(),
            0,
            "Fees accrued have been reduced to zero"
        );
        assert_eq!(
            perp_market.fees_settled.round(),
            initial_fees,
            "Fees have been fully settled"
        );
    }

    Ok(())
}
