use super::*;

#[tokio::test]
async fn test_trigger() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(95_000); // logging..
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
    // TEST: Create a perp market
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
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, &tokens[0]).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::ONE)
    };

    let oracle_condition = OraclePriceCondition {
        condition_type: ConditionType::OraclePrice.into(),
        padding0: 0,
        oracle: Pubkey::default(),
        threshold: I80F48::ZERO,
        trigger_when_above: 0,
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
            num: 1,
            condition: bytemuck::bytes_of(&oracle_condition).to_vec(),
            action,
        },
    )
    .await
    .unwrap();

    Ok(())
}
