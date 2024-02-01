use super::*;

#[tokio::test]
async fn test_liq_perps_bankruptcy() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(200_000); // PerpLiqNegativePnlOrBankruptcy takes a lot of CU
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..3];
    let payer_mint_accounts = &context.users[1].token_accounts[0..3];

    //
    // SETUP: Create a group and an account to fill the vaults
    //

    let GroupWithTokens {
        group,
        tokens,
        insurance_vault,
        ..
    } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        zero_token_is_quote: true,
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    send_tx(
        solana,
        TokenEditWeights {
            group,
            admin,
            mint: mints[2].pubkey,
            maint_liab_weight: 1.0,
            maint_asset_weight: 1.0,
            init_liab_weight: 1.0,
            init_asset_weight: 1.0,
        },
    )
    .await
    .unwrap();

    let fund_insurance = |amount: u64| async move {
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction_direct(
            spl_token::instruction::transfer(
                &spl_token::ID,
                &payer_mint_accounts[0],
                &insurance_vault,
                &payer.pubkey(),
                &[&payer.pubkey()],
                amount,
            )
            .unwrap(),
        );
        tx.add_signer(payer);
        tx.send().await.unwrap();
    };

    let quote_token = &tokens[0]; // USDC, 1/1 weights, price 1, never changed
    let base_token = &tokens[1]; // used for perp market
    let collateral_token = &tokens[2]; // used for adjusting account health

    // all perp markets used here default to price = 1.0, base_lot_size = 100
    let price_lots = 100;

    let context_ref = &context;
    let mut perp_market_index: PerpMarketIndex = 0;
    let setup_perp_inner = |perp_market_index: PerpMarketIndex,
                            health: i64,
                            pnl: i64,
                            settle_limit: i64| async move {
        // price used later to produce negative pnl with a short:
        // doubling the price leads to -100 pnl
        let adj_price = 1.0 + pnl as f64 / -100.0;
        let adj_price_lots = (price_lots as f64 * adj_price) as i64;

        let fresh_liqor = create_funded_account(
            &solana,
            group,
            owner,
            200 + perp_market_index as u32,
            &context_ref.users[1],
            mints,
            10000,
            0,
        )
        .await;

        let mango_v4::accounts::PerpCreateMarket { perp_market, .. } = send_tx(
            solana,
            PerpCreateMarketInstruction {
                group,
                admin,
                payer,
                perp_market_index,
                quote_lot_size: 1,
                base_lot_size: 100,
                maint_base_asset_weight: 0.8,
                init_base_asset_weight: 0.6,
                maint_base_liab_weight: 1.2,
                init_base_liab_weight: 1.4,
                base_liquidation_fee: 0.05,
                maker_fee: 0.0,
                taker_fee: 0.0,
                group_insurance_fund: true,
                // adjust this factur such that we get the desired settle limit in the end
                settle_pnl_limit_factor: (settle_limit as f32 - 0.1).max(0.0)
                    / (1.0 * 100.0 * adj_price) as f32,
                settle_pnl_limit_window_size_ts: 24 * 60 * 60,
                ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, base_token).await
            },
        )
        .await
        .unwrap();
        set_perp_stub_oracle_price(solana, group, perp_market, &base_token, admin, 1.0).await;
        set_bank_stub_oracle_price(solana, group, &collateral_token, admin, 1.0).await;

        //
        // SETUP: accounts
        //
        let deposit_amount = 1000;
        let helper_account = create_funded_account(
            &solana,
            group,
            owner,
            perp_market_index as u32 * 2,
            &context_ref.users[1],
            &mints[2..3],
            deposit_amount,
            0,
        )
        .await;
        let account = create_funded_account(
            &solana,
            group,
            owner,
            perp_market_index as u32 * 2 + 1,
            &context_ref.users[1],
            &mints[2..3],
            deposit_amount,
            0,
        )
        .await;

        //
        // SETUP: Trade perps between accounts twice to generate pnl, settle_limit
        //
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction(PerpPlaceOrderInstruction {
            account: helper_account,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 1,
            ..PerpPlaceOrderInstruction::default()
        })
        .await;
        tx.add_instruction(PerpPlaceOrderInstruction {
            account: account,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            max_base_lots: 1,
            ..PerpPlaceOrderInstruction::default()
        })
        .await;
        tx.add_instruction(PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account, helper_account],
        })
        .await;
        tx.send().await.unwrap();

        set_perp_stub_oracle_price(solana, group, perp_market, &base_token, admin, adj_price).await;
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction(PerpPlaceOrderInstruction {
            account: helper_account,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots: adj_price_lots,
            max_base_lots: 1,
            ..PerpPlaceOrderInstruction::default()
        })
        .await;
        tx.add_instruction(PerpPlaceOrderInstruction {
            account: account,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots: adj_price_lots,
            max_base_lots: 1,
            ..PerpPlaceOrderInstruction::default()
        })
        .await;
        tx.add_instruction(PerpConsumeEventsInstruction {
            perp_market,
            mango_accounts: vec![account, helper_account],
        })
        .await;
        tx.send().await.unwrap();

        set_perp_stub_oracle_price(solana, group, perp_market, &base_token, admin, 1.0).await;

        // Adjust target health:
        // full health = 1000 * collat price * 1.0 + pnl
        set_bank_stub_oracle_price(
            solana,
            group,
            &collateral_token,
            admin,
            (health - pnl) as f64 / 1000.0,
        )
        .await;

        // Verify we got it right
        let account_data = solana.get_account::<MangoAccount>(account).await;
        assert_eq!(account_data.perps[0].quote_position_native(), pnl);
        assert_eq!(
            account_data.perps[0].recurring_settle_pnl_allowance,
            settle_limit
        );
        assert_eq!(
            account_init_health(solana, account).await.round(),
            health as f64
        );

        (perp_market, account, fresh_liqor)
    };
    let mut setup_perp = |health: i64, pnl: i64, settle_limit: i64| {
        let out = setup_perp_inner(perp_market_index, health, pnl, settle_limit);
        perp_market_index += 1;
        out
    };

    let limit_prec = |f: f64| (f * 1000.0).round() / 1000.0;

    let liq_event_amounts = || {
        let settlement = solana
            .program_log_events::<mango_v4::logs::PerpLiqNegativePnlOrBankruptcyLog>()
            .pop()
            .map(|v| limit_prec(I80F48::from_bits(v.settlement).to_num::<f64>()))
            .unwrap_or(0.0);
        let (insur, loss) = solana
            .program_log_events::<mango_v4::logs::PerpLiqBankruptcyLog>()
            .pop()
            .map(|v| {
                (
                    I80F48::from_bits(v.insurance_transfer).to_num::<u64>(),
                    limit_prec(I80F48::from_bits(v.socialized_loss).to_num::<f64>()),
                )
            })
            .unwrap_or((0, 0.0));
        (settlement, insur, loss)
    };

    let liqor_info = |perp_market: Pubkey, liqor: Pubkey| async move {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
        let liqor_perp = liqor_data
            .perps
            .iter()
            .find(|p| p.market_index == perp_market.perp_market_index)
            .unwrap()
            .clone();
        (liqor_data, liqor_perp)
    };

    {
        let (perp_market, account, liqor) = setup_perp(-28, -50, 10).await;
        let liqor_quote_before = account_position(solana, liqor, quote_token.bank).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: 1,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (1.0, 0, 0.0));

        assert_eq!(
            account_position(solana, account, quote_token.bank).await,
            -1
        );
        assert_eq!(
            account_position(solana, liqor, quote_token.bank).await,
            liqor_quote_before + 1
        );
        let acc_data = solana.get_account::<MangoAccount>(account).await;
        assert_eq!(acc_data.perps[0].quote_position_native(), -49);
        assert_eq!(acc_data.being_liquidated, 1);
        let (_liqor_data, liqor_perp) = liqor_info(perp_market, liqor).await;
        assert_eq!(liqor_perp.quote_position_native(), -1);
    }

    {
        let (perp_market, account, liqor) = setup_perp(-28, -50, 10).await;
        fund_insurance(2).await;
        let liqor_quote_before = account_position(solana, liqor, quote_token.bank).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: 11,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (10.0, 2, 27.0));

        assert_eq!(
            account_position(solana, account, quote_token.bank).await,
            -10
        );
        assert_eq!(
            account_position(solana, liqor, quote_token.bank).await,
            liqor_quote_before + 12
        );
        let acc_data = solana.get_account::<MangoAccount>(account).await;
        assert!(assert_equal(
            acc_data.perps[0].quote_position_native(),
            -50.0 + 11.0 + 27.0,
            0.1
        ));
        assert_eq!(acc_data.being_liquidated, 0);
        let (_liqor_data, liqor_perp) = liqor_info(perp_market, liqor).await;
        assert_eq!(liqor_perp.quote_position_native(), -11);
    }

    {
        let (perp_market, account, liqor) = setup_perp(-28, -50, 10).await;
        fund_insurance(5).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: 16,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            liq_event_amounts(),
            (10.0, 5, limit_prec(28.0 - 5.0 / 1.05))
        );
    }

    // no insurance
    {
        let (perp_market, account, liqor) = setup_perp(-28, -50, 10).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: u64::MAX,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (10.0, 0, limit_prec(28.0)));
    }

    // no settlement: no settle health
    {
        let (perp_market, account, liqor) = setup_perp(-200, -50, 10).await;
        fund_insurance(5).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: u64::MAX,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (0.0, 5, limit_prec(50.0 - 5.0 / 1.05)));
    }

    // no settlement: no settle limit
    {
        let (perp_market, account, liqor) = setup_perp(-40, -50, 0).await;
        // no insurance

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: u64::MAX,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (0.0, 0, limit_prec(40.0)));
    }

    // no socialized loss: fully covered by insurance fund
    {
        let (perp_market, account, liqor) = setup_perp(-40, -50, 5).await;
        fund_insurance(42).await;

        send_tx(
            solana,
            PerpLiqNegativePnlOrBankruptcyInstruction {
                liqor,
                liqor_owner: owner,
                liqee: account,
                perp_market,
                max_liab_transfer: u64::MAX,
            },
        )
        .await
        .unwrap();
        assert_eq!(liq_event_amounts(), (5.0, 42, 0.0));
    }

    Ok(())
}
