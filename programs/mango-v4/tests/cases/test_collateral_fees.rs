#![allow(unused_assignments)]

use super::*;
use crate::cases::test_serum::SerumOrderPlacer;
use num::ToPrimitive;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

#[tokio::test]
async fn test_collateral_fees() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];
    let mut prices = HashMap::new();

    // 1 unit = 1$
    prices.insert(mints[0].pubkey, 1_000_000f64);
    prices.insert(mints[1].pubkey, 1_000_000f64);

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        prices: prices,
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // fund the vaults to allow borrowing
    create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        1_000_000,
        0,
    )
    .await;

    let account = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        &mints[0..1],
        1_500, // maint: 0.8 * 1500 = 1200
        0,
    )
    .await;

    let empty_account = create_funded_account(
        &solana,
        group,
        owner,
        2,
        &context.users[1],
        &mints[0..0],
        0,
        0,
    )
    .await;

    let hour = 60 * 60;

    send_tx(
        solana,
        GroupEdit {
            group,
            admin,
            options: mango_v4::instruction::GroupEdit {
                collateral_fee_interval_opt: Some(6 * hour),
                ..group_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    set_collateral_fees(solana, admin, mints, group, 0, 0.1).await;
    set_loan_orig_fee(solana, admin, mints, group, 1, 0.0).await;

    //
    // TEST: It works on empty accounts
    //

    send_tx(
        solana,
        TokenChargeCollateralFeesInstruction {
            account: empty_account,
        },
    )
    .await
    .unwrap();
    let mut last_time = solana.clock_timestamp().await;
    solana.set_clock_timestamp(last_time + 9 * hour).await;

    // send it twice, because the first time will never charge anything
    send_tx(
        solana,
        TokenChargeCollateralFeesInstruction {
            account: empty_account,
        },
    )
    .await
    .unwrap();
    last_time = solana.clock_timestamp().await;

    //
    // TEST: Without borrows, charging collateral fees has no effect
    //

    send_tx(solana, TokenChargeCollateralFeesInstruction { account })
        .await
        .unwrap();
    last_time = solana.clock_timestamp().await;
    solana.set_clock_timestamp(last_time + 9 * hour).await;

    // send it twice, because the first time will never charge anything
    send_tx(solana, TokenChargeCollateralFeesInstruction { account })
        .await
        .unwrap();
    last_time = solana.clock_timestamp().await;

    // no effect
    assert_eq!(
        account_position(solana, account, tokens[0].bank).await,
        1_500
    );

    //
    // TEST: With borrows, there's an effect depending on the time that has passed
    //

    withdraw(&context, solana, owner, account, 500, 1).await; // maint: -1.2 * 500 = -600 (half of 1200)

    solana.set_clock_timestamp(last_time + 9 * hour).await;

    send_tx(solana, TokenChargeCollateralFeesInstruction { account })
        .await
        .unwrap();
    last_time = solana.clock_timestamp().await;

    let fee = 1500.0 * (0.1 * (9.0 / 24.0) * (600.0 / 1200.0));
    println!("fee -> {}", fee);
    assert_eq_f64!(
        account_position_f64(solana, account, tokens[0].bank).await,
        1500.0,
        0.01
    );
    assert_eq_f64!(
        account_position_f64(solana, account, tokens[1].bank).await,
        -500.0 - fee,
        0.01
    );
    let last_balance = account_position_f64(solana, account, tokens[1].bank).await;

    //
    // TEST: More borrows
    //

    withdraw(&context, solana, owner, account, 100, 1).await; // maint: -1.2 * 600 = -720

    solana.set_clock_timestamp(last_time + 7 * hour).await;

    send_tx(solana, TokenChargeCollateralFeesInstruction { account })
        .await
        .unwrap();
    //last_time = solana.clock_timestamp().await;
    let fee = 1500.0 * 0.1 * (7.0 / 24.0) * ((last_balance.abs() + 100.0) * 1.2 / (1500.0 * 0.8));
    println!("fee -> {}", fee);
    assert_eq_f64!(
        account_position_f64(solana, account, tokens[0].bank).await,
        1500.0,
        0.01
    );
    assert_eq_f64!(
        account_position_f64(solana, account, tokens[1].bank).await,
        -(last_balance.abs() + 100.0) - fee,
        0.01
    );

    Ok(())
}

#[tokio::test]
async fn test_collateral_fees_multi() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..4];
    let mut prices = HashMap::new();

    prices.insert(mints[0].pubkey, 1_000_000f64); // 1 unit = 1$
    prices.insert(mints[1].pubkey, 3_000_000f64); // 1 unit = 3$
    prices.insert(mints[2].pubkey, 5_000_000f64); // 1 unit = 5$
    prices.insert(mints[3].pubkey, 20_000_000f64); // 1 unit = 20$

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        prices,
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // fund the vaults to allow borrowing
    create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        1_000_000,
        0,
    )
    .await;

    let account = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        &mints[0..2],
        1_500, // maint: 0.8 * 1500 = 1200
        0,
    )
    .await;

    let hour = 60 * 60;

    send_tx(
        solana,
        GroupEdit {
            group,
            admin,
            options: mango_v4::instruction::GroupEdit {
                collateral_fee_interval_opt: Some(6 * hour),
                ..group_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    // Set fees

    set_collateral_fees(solana, admin, mints, group, 0, 0.1).await;
    set_collateral_fees(solana, admin, mints, group, 1, 0.2).await;
    set_loan_orig_fee(solana, admin, mints, group, 2, 0.0).await;
    set_loan_orig_fee(solana, admin, mints, group, 3, 0.0).await;

    //
    // TEST: With borrows, there's an effect depending on the time that has passed
    //

    withdraw(&context, solana, owner, account, 50, 2).await; // maint: -1.2 * 50 = -60 (250$ -> 300$)
    withdraw(&context, solana, owner, account, 100, 3).await; // maint: -1.2 * 100 = -120 (2000$ -> 2400$)

    send_tx(solana, TokenChargeCollateralFeesInstruction { account })
        .await
        .unwrap();
    let mut last_time = solana.clock_timestamp().await;
    solana.set_clock_timestamp(last_time + 9 * hour).await;

    // send it twice, because the first time will never charge anything
    send_tx(solana, TokenChargeCollateralFeesInstruction { account })
        .await
        .unwrap();
    last_time = solana.clock_timestamp().await;

    let usage_factor = (60.0 * 5.0 + 120.0 * 20.0) / ((1500.0 + 1500.0 * 3.0) * 0.8);
    let time_factor = 9.0 / 24.0;
    let collateral_fee_factor = 1500.0 * 0.1 + 1500.0 * 3.0 * 0.2;
    let collateral_fee = collateral_fee_factor * time_factor * usage_factor;
    // println!("fee -> {}", collateral_fee);
    assert_eq_f64!(
        account_position_f64(solana, account, tokens[0].bank).await,
        1500.0,
        0.01
    );
    assert_eq_f64!(
        account_position_f64(solana, account, tokens[1].bank).await,
        1500.0,
        0.01
    );
    assert_eq_f64!(
        account_position_f64(solana, account, tokens[2].bank).await,
        -50.0 - (300.0 / 2700.0) * collateral_fee / 5.0,
        0.01
    );
    assert_eq_f64!(
        account_position_f64(solana, account, tokens[3].bank).await,
        -100.0 - (2400.0 / 2700.0) * collateral_fee / 20.0,
        0.01
    );

    Ok(())
}

// Test convention
//
// T = Token without collateral fee
// Tc = Token with collateral fee
// B_x = Balance of x
// O_x = Amount in OO for x (market will be x/T1)
// F_x = Collateral Fee charged on x
//
// Asset weight = 0.8
// Liab weight = 1.2
// All amounts in USD
// Base lot is 100

#[tokio::test]
async fn test_basics() -> Result<(), TransportError> {
    let test_cases = parse_test_cases("\
         B_T1 ;  B_T2 ;  B_Tc1 ; B_Tc2 ; B_Tc3 ; B_Tc4 ; O_T1 ; O_T2 ; O_Tc1 ; O_Tc2 ; O_Tc3 ; O_Tc4 ; CF_T1 ; CF_T2 ; CF_Tc1 ; CF_Tc2 ; CF_Tc3 ; CF_Tc4 \r\n \
        -2000 ;    0  ;  10000 ;     0 ;     0 ;     0 ;    0 ;    0 ;     0 ;     0 ;     0 ;     0 ;   -300 ;     0 ;      0 ;      0 ;      0 ;      0 \r\n \
        -2000 ;    0  ;   5000 ;  5000 ;     0 ;     0 ;    0 ;    0 ;     0 ;     0 ;     0 ;     0 ;   -300 ;     0 ;      0 ;      0 ;      0 ;      0 \r\n \
         -500 ; -1500 ;  10000 ;     0 ;     0 ;     0 ;    0 ;    0 ;     0 ;     0 ;     0 ;     0 ;    -75 ;  -225 ;      0 ;      0 ;      0 ;      0 \r\n \
    ");

    run_scenario(test_cases).await
}

#[tokio::test]
async fn test_creating_borrow_from_oo() -> Result<(), TransportError> {
    let test_cases = parse_test_cases("\
         B_T1 ;  B_T2 ;  B_Tc1 ; B_Tc2 ; B_Tc3 ; B_Tc4 ; O_T1 ; O_T2 ; O_Tc1 ; O_Tc2 ; O_Tc3 ; O_Tc4 ; CF_T1 ; CF_T2 ; CF_Tc1 ; CF_Tc2 ; CF_Tc3 ; CF_Tc4 \r\n \
        -2000 ;    0  ;  10000 ;     0 ;     0 ;     0 ;    0 ;  200 ;     0 ;     0 ;     0 ;     0 ;   -300 ;     0 ;      0 ;      0 ;      0 ;      0 \r\n \
        -2000 ;    0  ;  10000 ;     0 ;     0 ;     0 ;    0 ;    0 ;   300 ;     0 ;     0 ;     0 ;   -300 ;     0 ;      0 ;      0 ;      0 ;      0 \r\n \
    ");

    run_scenario(test_cases).await
}

#[tokio::test]
async fn test_hiding_collateral_using_oo() -> Result<(), TransportError> {
    let test_cases = parse_test_cases("\
         B_T1 ;  B_T2 ;  B_Tc1 ; B_Tc2 ; B_Tc3 ; B_Tc4 ; O_T1 ; O_T2 ; O_Tc1 ; O_Tc2 ; O_Tc3 ; O_Tc4 ; CF_T1 ; CF_T2 ; CF_Tc1 ; CF_Tc2 ; CF_Tc3 ; CF_Tc4 \r\n \
        -2000 ;    0  ;  10000 ;     0 ;     0 ;     0 ;    0 ; -200 ;     0 ;     0 ;     0 ;     0 ;   -300 ;     0 ;      0 ;      0 ;      0 ;      0 \r\n \
        -2000 ;    0  ;  10000 ;     0 ;     0 ;     0 ;    0 ;    0 ;  -300 ;     0 ;     0 ;     0 ;   -300 ;     0 ;      0 ;      0 ;      0 ;      0 \r\n \
    ");

    run_scenario(test_cases).await
}

async fn run_scenario(test_cases: Vec<Vec<f64>>) -> Result<(), TransportError> {
    for test_case in test_cases {
        if test_case.len() == 0 {
            continue;
        }

        let mut test_builder = TestContextBuilder::new();
        test_builder.test().set_compute_max_units(200_000);
        let context = test_builder.start_default().await;
        let solana = &context.solana.clone();

        let admin = TestKeypair::new();
        let owner = context.users[0].key;
        let payer = context.users[1].key;
        let mints = &context.mints[0..6];
        let mut prices = HashMap::new();

        // Setup prices
        for i in 0..6 {
            prices.insert(mints[i].pubkey, (i as f64 + 1.0) * 1_000_000f64); // 1 unit = i$
        }

        let mango_setup::GroupWithTokens { group, tokens, .. } =
            mango_setup::GroupWithTokensConfig {
                admin,
                payer,
                mints: mints.to_vec(),
                prices,
                ..mango_setup::GroupWithTokensConfig::default()
            }
            .create(solana)
            .await;

        // Setup fees
        set_collateral_fees(solana, admin, mints, group, 2, 0.1).await;
        set_collateral_fees(solana, admin, mints, group, 3, 0.1).await;
        set_collateral_fees(solana, admin, mints, group, 4, 0.1).await;
        set_collateral_fees(solana, admin, mints, group, 5, 0.1).await;
        for i in 0..6 {
            set_loan_orig_fee(solana, admin, mints, group, i, 0.0).await;
        }

        // fund the vaults to allow borrowing
        create_funded_account(
            &solana,
            group,
            owner,
            0,
            &context.users[1],
            mints,
            9_000_000_000,
            0,
        )
        .await;

        let account = send_tx(
            solana,
            AccountCreateInstruction {
                account_num: 1,
                group,
                owner,
                payer: context.users[1].key,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .account;

        // For Spot order

        let hour = 60 * 60;

        send_tx(
            solana,
            GroupEdit {
                group,
                admin,
                options: mango_v4::instruction::GroupEdit {
                    collateral_fee_interval_opt: Some(24 * hour),
                    ..group_edit_instruction_default()
                },
            },
        )
        .await
        .unwrap();

        // Setup balance
        for (index, balance) in test_case[0..6].iter().enumerate() {
            if *balance > 0.0 {
                deposit(
                    solana,
                    owner,
                    &context.users[1],
                    account,
                    ((*balance as f64) / (index + 1) as f64).ceil() as u64,
                    index,
                )
                .await;
            }
        }
        for (index, balance) in test_case[0..6].iter().enumerate() {
            if *balance < 0.0 {
                withdraw(
                    &context,
                    solana,
                    owner,
                    account,
                    ((balance.abs() as f64) / (index + 1) as f64).ceil() as u64,
                    index,
                )
                .await;
            }
        }

        // Setup orders
        for (index, order) in test_case[6..12].iter().enumerate() {
            if *order == 0.0 {
                continue;
            }

            create_order(
                solana,
                &context,
                group,
                admin,
                owner,
                &context.users[0],
                account,
                (index + 1) as f64,
                (order / (index + 1) as f64).floor() as i64,
                &tokens[index],
                &tokens[0],
            )
            .await;
        }

        //
        // TEST
        //

        let mut balances = vec![];
        for i in 0..6 {
            if test_case[i] == 0.0 {
                balances.push(0f64);
            } else {
                balances.push(account_position_f64(solana, account, tokens[i].bank).await);
            }
        }

        send_tx(solana, TokenChargeCollateralFeesInstruction { account })
            .await
            .unwrap();
        let mut last_time = solana.clock_timestamp().await;
        solana.set_clock_timestamp(last_time + 24 * hour).await;

        // send it twice, because the first time will never charge anything
        send_tx(solana, TokenChargeCollateralFeesInstruction { account })
            .await
            .unwrap();
        last_time = solana.clock_timestamp().await;

        // Assert balance change
        for (index, expected_fee) in test_case[12..].iter().enumerate() {
            if test_case[index] == 0.0 {
                continue;
            }

            let current_balance = account_position_f64(solana, account, tokens[index].bank).await;
            let previous_balance = balances[index];
            let actual_fee = (current_balance - previous_balance) * (index + 1) as f64;

            assert_eq_f64!(actual_fee, expected_fee.to_f64().unwrap(), 0.01);
        }
    }

    Ok(())
}

fn parse_test_cases(test_cases: &str) -> Vec<Vec<f64>> {
    test_cases
        .split("\r\n")
        .skip(1)
        .map(|x| {
            x.split(";")
                .filter_map(|y| {
                    let y = y.trim();
                    if y.len() == 0 {
                        return None;
                    }
                    Some(f64::from_str(y).unwrap())
                })
                .collect_vec()
        })
        .collect_vec()
}

async fn create_order(
    solana: &Arc<SolanaCookie>,
    context: &TestContext,
    group: Pubkey,
    admin: TestKeypair,
    owner: TestKeypair,
    payer: &UserCookie,
    account: Pubkey,
    price: f64,
    quantity: i64,
    base_token: &Token,
    quote_token: &Token,
) -> Option<(u128, u64)> {
    let serum_market_cookie = context
        .serum
        .list_spot_market(&base_token.mint, &quote_token.mint)
        .await;

    //
    // TEST: Register a serum market
    //
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
            payer: payer.key,
        },
    )
    .await
    .unwrap()
    .serum_market;

    //
    // TEST: Create an open orders account
    //
    let open_orders = send_tx(
        solana,
        Serum3CreateOpenOrdersInstruction {
            account,
            serum_market,
            owner,
            payer: payer.key,
        },
    )
    .await
    .unwrap()
    .open_orders;

    let mut order_placer = SerumOrderPlacer {
        solana: solana.clone(),
        serum: context.serum.clone(),
        account,
        owner: owner.clone(),
        serum_market,
        open_orders,
        next_client_order_id: 0,
    };

    if quantity > 0 {
        order_placer.bid_maker(price, quantity as u64).await
    } else {
        order_placer.ask(price, quantity.abs() as u64).await
    }
}

async fn withdraw(
    context: &TestContext,
    solana: &Arc<SolanaCookie>,
    owner: TestKeypair,
    account: Pubkey,
    amount: u64,
    token_index: usize,
) {
    // println!("WITHDRAWING {} - token index {}", amount, token_index);
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: amount,
            allow_borrow: true,
            account,
            owner,
            token_account: context.users[1].token_accounts[token_index],
            bank_index: 0,
        },
    )
    .await
    .unwrap();
}

async fn deposit(
    solana: &Arc<SolanaCookie>,
    owner: TestKeypair,
    payer: &UserCookie,
    account: Pubkey,
    amount: u64,
    token_index: usize,
) {
    // println!("DEPOSITING {} - token index {}", amount, token_index);
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: amount,
            reduce_only: false,
            account,
            owner,
            token_account: payer.token_accounts[token_index],
            token_authority: payer.key,
            bank_index: 0,
        },
    )
    .await
    .unwrap();
}

async fn set_loan_orig_fee(
    solana: &Arc<SolanaCookie>,
    admin: TestKeypair,
    mints: &[MintCookie],
    group: Pubkey,
    token_index: usize,
    rate: f32,
) {
    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: mints[token_index].pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                loan_origination_fee_rate_opt: Some(rate),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();
}

async fn set_collateral_fees(
    solana: &Arc<SolanaCookie>,
    admin: TestKeypair,
    mints: &[MintCookie],
    group: Pubkey,
    token_index: usize,
    rate: f32,
) {
    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: mints[token_index].pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                collateral_fee_per_day_opt: Some(rate),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();
}
