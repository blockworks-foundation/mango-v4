use super::*;

#[tokio::test]
async fn test_trigger() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(140_000); // logging..
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];

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

    let base_token = &tokens[1];
    let quote_token = &tokens[0];

    let deposit_amount = 1000;
    let account_0 = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;
    let account_1 = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;
    let settler =
        create_funded_account(&solana, group, owner, 251, &context.users[1], &[], 0, 0).await;
    let settler_owner = owner.clone();

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
    // SETUP: Create a perp market
    //
    let mango_v4::accounts::PerpCreateMarket {
        perp_market, bids, ..
    } = send_tx(
        solana,
        PerpCreateMarketInstruction {
            group,
            admin,
            payer,
            perp_market_index: 0,
            quote_lot_size: 10,
            base_lot_size: 100,
            maint_base_asset_weight: 0.975,
            init_base_asset_weight: 0.95,
            maint_base_liab_weight: 1.025,
            init_base_liab_weight: 1.05,
            base_liquidation_fee: 0.012,
            maker_fee: -0.0001,
            taker_fee: 0.0002,
            settle_pnl_limit_factor: -1.0,
            settle_pnl_limit_window_size_ts: 24 * 60 * 60,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &base_token).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::ONE)
    };

    //
    // SETUP: User most have open orders for serum trigger orders to work
    //
    send_tx(
        solana,
        Serum3CreateOpenOrdersInstruction {
            account: account_0,
            serum_market,
            owner,
            payer,
        },
    )
    .await
    .unwrap();

    //
    // TEST: Do stuff with triggers
    //

    send_tx(
        solana,
        TriggersCreateInstruction {
            account: account_0,
            authority: owner,
            payer,
        },
    )
    .await
    .unwrap();

    {
        let oracle_condition = OraclePriceCondition {
            condition_type: ConditionType::OraclePrice.into(),
            padding0: 0,
            base_oracle: tokens[1].oracle,
            quote_oracle: tokens[0].oracle,
            threshold_ui: 0.5,
            trigger_when_above: 1,
            base_conf_filter: 0.1,
            quote_conf_filter: 0.1,
            base_max_staleness_slots: -1,
            quote_max_staleness_slots: -1,
            padding: Default::default(),
        };

        let perp_cpi = PerpCpiAction {
            action_type: ActionType::PerpCpi.into(),
            perp_market_index: 0,
            reserved: [0; 58],
        };

        let perp_place_order = mango_v4::instruction::PerpPlaceOrderV2 {
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            max_quote_lots: 1,
            client_order_id: 42,
            order_type: PlaceOrderType::Limit,
            self_trade_behavior: SelfTradeBehavior::DecrementTake,
            reduce_only: false,
            expiry_timestamp: 0,
            limit: 10,
        };

        let mut action = bytemuck::bytes_of(&perp_cpi).to_vec();
        action.extend(anchor_lang::InstructionData::data(&perp_place_order));

        send_tx(
            solana,
            TriggerCreateInstruction {
                account: account_0,
                authority: owner,
                payer,
                condition: bytemuck::bytes_of(&oracle_condition).to_vec(),
                action,
            },
        )
        .await
        .unwrap();
    }

    {
        // TODO: these structs should really move
        use mango_v4::accounts_ix::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};

        let oracle_condition = OraclePriceCondition {
            condition_type: ConditionType::OraclePrice.into(),
            padding0: 0,
            base_oracle: tokens[1].oracle,
            quote_oracle: tokens[0].oracle,
            threshold_ui: 0.5,
            trigger_when_above: 1,
            base_conf_filter: 0.1,
            quote_conf_filter: 0.1,
            base_max_staleness_slots: -1,
            quote_max_staleness_slots: -1,
            padding: Default::default(),
        };

        let serum_cpi = Serum3CpiAction {
            action_type: ActionType::Serum3Cpi.into(),
            serum3_market: serum_market,
            reserved: [0; 60],
        };

        let serum_place_order = mango_v4::instruction::Serum3PlaceOrder {
            side: Serum3Side::Bid,
            limit_price: 1,
            max_base_qty: 1,
            max_native_quote_qty_including_fees: 1,
            self_trade_behavior: Serum3SelfTradeBehavior::DecrementTake,
            order_type: Serum3OrderType::Limit,
            client_order_id: 42,
            limit: 10,
        };

        let mut action = bytemuck::bytes_of(&serum_cpi).to_vec();
        action.extend(anchor_lang::InstructionData::data(&serum_place_order));

        send_tx(
            solana,
            TriggerCreateInstruction {
                account: account_0,
                authority: owner,
                payer,
                condition: bytemuck::bytes_of(&oracle_condition).to_vec(),
                action,
            },
        )
        .await
        .unwrap();
    }

    send_tx(
        solana,
        TriggerCheckInstruction {
            account: account_0,
            id: 1,
            triggerer: payer,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        TriggerCheckAndExecuteInstruction {
            account: account_0,
            id: 1,
            triggerer: payer,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        TriggerCheckInstruction {
            account: account_0,
            id: 2,
            triggerer: payer,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        TriggerCheckAndExecuteInstruction {
            account: account_0,
            id: 2,
            triggerer: payer,
        },
    )
    .await
    .unwrap();

    Ok(())
}
