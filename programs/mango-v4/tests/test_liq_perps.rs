#![cfg(feature = "test-bpf")]

use fixed::types::I80F48;
use solana_program_test::*;
use solana_sdk::transport::TransportError;

use mango_v4::state::*;
use program_test::*;

use mango_setup::*;

mod program_test;

use utils::assert_equal_fixed_f64 as assert_equal;

#[tokio::test]
async fn test_liq_perps_force_cancel() -> Result<(), TransportError> {
    let test_builder = TestContextBuilder::new();
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
    //let quote_token = &tokens[0];
    let base_token = &tokens[1];

    // deposit some funds, to the vaults aren't empty
    create_funded_account(&solana, group, owner, 0, &context.users[1], mints, 10000, 0).await;

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
            maint_asset_weight: 0.8,
            init_asset_weight: 0.6,
            maint_liab_weight: 1.2,
            init_liab_weight: 1.4,
            liquidation_fee: 0.05,
            maker_fee: 0.0,
            taker_fee: 0.0,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, base_token).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::ONE)
    };

    //
    // SETUP: Make an account and deposit some quote and base
    //
    let deposit_amount = 1000;
    let account = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        &mints[0..1],
        deposit_amount,
        0,
    )
    .await;

    send_tx(
        solana,
        TokenDepositInstruction {
            amount: 1,
            account,
            owner,
            token_account: payer_mint_accounts[1],
            token_authority: payer,
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Place a perp order
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account,
            perp_market,
            owner,
            side: Side::Ask,
            price_lots,
            // health was 1000 * 0.6 = 600; this order is -14*100*(1.4-1) = -560
            max_base_lots: 14,
            max_quote_lots: i64::MAX,
            client_order_id: 0,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Change the oracle to make health go negative
    //
    set_bank_stub_oracle_price(solana, group, base_token, admin, 10.0).await;

    // verify health is bad: can't withdraw
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
        PerpLiqForceCancelOrdersInstruction {
            account,
            perp_market,
        },
    )
    .await
    .unwrap();

    // can withdraw again
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
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
async fn test_liq_perps_base_position_and_bankruptcy() -> Result<(), TransportError> {
    let test_builder = TestContextBuilder::new();
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

    // fund the insurance vault
    let insurance_vault_funding = 100;
    {
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction_direct(
            spl_token::instruction::transfer(
                &spl_token::ID,
                &payer_mint_accounts[0],
                &insurance_vault,
                &payer.pubkey(),
                &[&payer.pubkey()],
                insurance_vault_funding,
            )
            .unwrap(),
        );
        tx.add_signer(payer);
        tx.send().await.unwrap();
    }

    let quote_token = &tokens[0];
    let base_token = &tokens[1];

    // deposit some funds, to the vaults aren't empty
    let liqor = create_funded_account(
        &solana,
        group,
        owner,
        250,
        &context.users[1],
        mints,
        10000,
        0,
    )
    .await;
    let settler =
        create_funded_account(&solana, group, owner, 251, &context.users[1], &[], 0, 0).await;
    let settler_owner = owner.clone();

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
            maint_asset_weight: 0.8,
            init_asset_weight: 0.6,
            maint_liab_weight: 1.2,
            init_liab_weight: 1.4,
            liquidation_fee: 0.05,
            maker_fee: 0.0,
            taker_fee: 0.0,
            group_insurance_fund: true,
            ..PerpCreateMarketInstruction::with_new_book_and_queue(&solana, base_token).await
        },
    )
    .await
    .unwrap();

    let price_lots = {
        let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
        perp_market.native_price_to_lot(I80F48::ONE)
    };

    //
    // SETUP: Make an two accounts and deposit some quote and base
    //
    let context_ref = &context;
    let make_account = |idx: u32| async move {
        let deposit_amount = 1000;
        let account = create_funded_account(
            &solana,
            group,
            owner,
            idx,
            &context_ref.users[1],
            &mints[0..1],
            deposit_amount,
            0,
        )
        .await;

        send_tx(
            solana,
            TokenDepositInstruction {
                amount: 1,
                account,
                owner,
                token_account: payer_mint_accounts[1],
                token_authority: payer,
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        account
    };
    let account_0 = make_account(0).await;
    let account_1 = make_account(1).await;

    //
    // SETUP: Trade perps between accounts
    //
    // health was 1000 + 1 * 0.8 = 1000.8 before
    // after this order it is changed by -20*100*(1.4-1) = -800 for the short
    // and 20*100*(0.6-1) = -800 for the long
    //
    send_tx(
        solana,
        PerpPlaceOrderInstruction {
            account: account_0,
            perp_market,
            owner,
            side: Side::Bid,
            price_lots,
            max_base_lots: 20,
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
            max_base_lots: 20,
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

    //
    // SETUP: Change the oracle to make health go negative for account_0
    //
    set_bank_stub_oracle_price(solana, group, base_token, admin, 0.5).await;

    // verify health is bad: can't withdraw
    assert!(send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: false,
            account: account_0,
            owner,
            token_account: payer_mint_accounts[1],
            bank_index: 0,
        }
    )
    .await
    .is_err());

    //
    // TEST: Liquidate base position
    //
    send_tx(
        solana,
        PerpLiqBasePositionInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_0,
            perp_market,
            max_base_transfer: 10,
        },
    )
    .await
    .unwrap();

    let liq_amount = 10.0 * 100.0 * 0.5 * (1.0 - 0.05);
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 10);
    assert!(assert_equal(
        liqor_data.perps[0].quote_position_native(),
        -liq_amount,
        0.1
    ));
    let liqee_data = solana.get_account::<MangoAccount>(account_0).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 10);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        -20.0 * 100.0 + liq_amount,
        0.1
    ));

    //
    // SETUP: Change the oracle to make health go negative for account_1
    //
    set_bank_stub_oracle_price(solana, group, base_token, admin, 2.0).await;

    // verify health is bad: can't withdraw
    assert!(send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: false,
            account: account_1,
            owner,
            token_account: payer_mint_accounts[1],
            bank_index: 0,
        }
    )
    .await
    .is_err());

    //
    // TEST: Liquidate base position
    //
    send_tx(
        solana,
        PerpLiqBasePositionInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_1,
            perp_market,
            max_base_transfer: i64::MIN,
        },
    )
    .await
    .unwrap();

    let liq_amount_2 = 20.0 * 100.0 * 2.0 * (1.0 + 0.05);
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    assert_eq!(liqor_data.perps[0].base_position_lots(), 10 - 20);
    assert!(assert_equal(
        liqor_data.perps[0].quote_position_native(),
        -liq_amount + liq_amount_2,
        0.1
    ));
    let liqee_data = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 0);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        20.0 * 100.0 - liq_amount_2,
        0.1
    ));

    //
    // TEST: Can't trigger perp bankruptcy yet, account_1 isn't bankrupt
    //
    assert!(send_tx(
        solana,
        PerpLiqBankruptcyInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_1,
            perp_market,
            max_liab_transfer: u64::MAX,
        }
    )
    .await
    .is_err());

    //
    // TEST: Can settle-pnl even though health is negative
    //
    send_tx(
        solana,
        PerpSettlePnlInstruction {
            settler,
            settler_owner,
            account_a: liqor,
            account_b: account_1,
            perp_market,
            settle_bank: tokens[0].bank,
        },
    )
    .await
    .unwrap();

    let liqee_settle_health_before = 1000.0 + 1.0 * 2.0 * 0.8;
    let remaining_pnl = 20.0 * 100.0 - liq_amount_2 + liqee_settle_health_before;
    assert!(remaining_pnl < 0.0);
    let liqee_data = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 0);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        remaining_pnl,
        0.1
    ));
    assert_eq!(
        account_position(solana, account_1, quote_token.bank).await,
        -2
    );
    assert_eq!(
        account_position(solana, account_1, base_token.bank).await,
        1
    );

    //
    // TEST: Still can't trigger perp bankruptcy, account_1 has token collateral left
    //
    assert!(send_tx(
        solana,
        PerpLiqBankruptcyInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_1,
            perp_market,
            max_liab_transfer: u64::MAX,
        }
    )
    .await
    .is_err());

    //
    // SETUP: Liquidate token collateral
    //
    send_tx(
        solana,
        TokenLiqWithTokenInstruction {
            liqee: account_1,
            liqor: liqor,
            liqor_owner: owner,
            asset_token_index: base_token.index,
            liab_token_index: quote_token.index,
            max_liab_transfer: I80F48::MAX,
            asset_bank_index: 0,
            liab_bank_index: 0,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        account_position(solana, account_1, quote_token.bank).await,
        0
    );
    assert!(account_position_closed(solana, account_1, base_token.bank).await);

    //
    // TEST: Now perp-bankruptcy will work, eat the insurance vault and socialize losses
    //
    let liqor_before = account_position_f64(solana, liqor, quote_token.bank).await;
    send_tx(
        solana,
        PerpLiqBankruptcyInstruction {
            liqor,
            liqor_owner: owner,
            liqee: account_1,
            perp_market,
            max_liab_transfer: u64::MAX,
        },
    )
    .await
    .unwrap();

    // insurance fund was depleted and the liqor received it
    assert_eq!(solana.token_account_balance(insurance_vault).await, 0);
    let liqor_data = solana.get_account::<MangoAccount>(liqor).await;
    let quote_bank = solana.get_account::<Bank>(tokens[0].bank).await;
    assert!(assert_equal(
        liqor_data.tokens[0].native(&quote_bank),
        liqor_before + insurance_vault_funding as f64,
        0.1
    ));

    // liqee's position is gone
    let liqee_data = solana.get_account::<MangoAccount>(account_1).await;
    assert_eq!(liqee_data.perps[0].base_position_lots(), 0);
    assert!(assert_equal(
        liqee_data.perps[0].quote_position_native(),
        0.0,
        0.1
    ));

    // the remainder got socialized via funding payments
    let socialized_amount = -remaining_pnl - 100.0 / 1.05;
    let perp_market = solana.get_account::<PerpMarket>(perp_market).await;
    assert!(assert_equal(
        perp_market.long_funding,
        socialized_amount / 20.0,
        0.1
    ));
    assert!(assert_equal(
        perp_market.short_funding,
        -socialized_amount / 20.0,
        0.1
    ));

    Ok(())
}
