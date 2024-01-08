use super::*;
use mango_v4::accounts_ix::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};

#[tokio::test]
async fn test_health_wrap() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];

    //
    // SETUP: Create a group, account, register a token (mint0)
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let quote_token = &tokens[0];
    let base_token = &tokens[1];

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

    // SETUP: Create an account with deposits, so the second account can borrow more than it has
    create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        200000000,
        0,
    )
    .await;

    // SETUP: Make a second account
    let account = create_funded_account(
        &solana,
        group,
        owner,
        1,
        &context.users[1],
        &mints[0..=1],
        100,
        0,
    )
    .await;

    send_tx(
        solana,
        Serum3CreateOpenOrdersInstruction {
            account,
            serum_market,
            owner,
            payer,
        },
    )
    .await
    .unwrap();

    let send_test_tx = |limit_price, order_size, cancel| {
        async move {
            let mut tx = ClientTransaction::new(solana);
            tx.add_instruction(HealthRegionBeginInstruction { account })
                .await;
            tx.add_instruction(Serum3PlaceOrderInstruction {
                side: Serum3Side::Ask,
                limit_price: (limit_price * 100.0 / 10.0) as u64, // in quote_lot (10) per base lot (100)
                max_base_qty: (order_size as u64) / 100,          // in base lot (100)
                max_native_quote_qty_including_fees: (limit_price * (order_size as f64)) as u64,
                self_trade_behavior: Serum3SelfTradeBehavior::AbortTransaction,
                order_type: Serum3OrderType::Limit,
                client_order_id: 42,
                limit: 10,
                account,
                owner,
                serum_market,
            })
            .await;
            if cancel {
                tx.add_instruction(Serum3CancelAllOrdersInstruction {
                    limit: 10,
                    account,
                    owner,
                    serum_market,
                })
                .await;
            }
            tx.add_instruction(HealthRegionEndInstruction {
                account,
                affected_bank: None,
            })
            .await;
            tx.send_get_metadata().await
        }
    };

    //
    // TEST: Placing a giant order fails
    //
    {
        let result = send_test_tx(1.0, 100000, false).await.unwrap();
        assert!(result.result.is_err());
        let logs = result.metadata.unwrap().log_messages;
        // reaches the End instruction
        assert!(logs
            .iter()
            .any(|line| line.contains("Instruction: HealthRegionEnd")));
        // errors due to health
        assert!(logs
            .iter()
            .any(|line| line.contains("Error Code: HealthMustBePositiveOrIncrease")));
        // health computed only once
        assert_eq!(
            logs.iter()
                .filter(|line| line.contains("post_init_health"))
                .count(),
            1
        );
    }

    //
    // TEST: If we cancel the order again before the HealthRegionEnd, it can go through
    //
    {
        let result = send_test_tx(1.0, 100000, true).await.unwrap();
        assert!(result.result.is_ok());
        let logs = result.metadata.unwrap().log_messages;
        // health computed only once
        assert_eq!(
            logs.iter()
                .filter(|line| line.contains("post_init_health"))
                .count(),
            1
        );
    }

    //
    // TEST: Try using withdraw in a health region
    //
    {
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction(HealthRegionBeginInstruction { account })
            .await;
        tx.add_instruction(TokenWithdrawInstruction {
            amount: 1,
            allow_borrow: true,
            account,
            owner,
            token_account: context.users[1].token_accounts[0],
            bank_index: 0,
        })
        .await;
        tx.add_instruction(HealthRegionEndInstruction {
            account,
            affected_bank: None,
        })
        .await;
        tx.send().await.unwrap_err();
    }

    //
    // TEST: Try using a different program in a health region
    //
    {
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction(HealthRegionBeginInstruction { account })
            .await;
        tx.add_instruction_direct(
            spl_token::instruction::transfer(
                &spl_token::ID,
                &context.users[1].token_accounts[0],
                &context.users[0].token_accounts[0],
                &owner.pubkey(),
                &[&owner.pubkey()],
                1,
            )
            .unwrap(),
        );
        tx.add_instruction(HealthRegionEndInstruction {
            account,
            affected_bank: None,
        })
        .await;
        tx.add_signer(owner);
        tx.send().await.unwrap_err();
    }

    Ok(())
}
