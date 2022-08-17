#![cfg(feature = "test-bpf")]

use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;

use program_test::*;

mod program_test;

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_margin_trade() -> Result<(), BanksClientError> {
    let builder = TestContextBuilder::new();
    let context = builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mints = &context.mints[0..2];
    let payer_mint0_account = context.users[1].token_accounts[0];
    let payer_mint1_account = context.users[1].token_accounts[1];
    let loan_origination_fee = 0.0005;

    // higher resolution that the loan_origination_fee for one token
    let balance_f64eq = |a: f64, b: f64| (a - b).abs() < 0.0001;

    //
    // SETUP: Create a group, account, register a token (mint0)
    //

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints,
    }
    .create(solana)
    .await;
    let bank = tokens[0].bank;
    let vault = tokens[0].vault;

    //
    // provide some funds for tokens, so the test user can borrow
    //
    let provided_amount = 1000;

    let provider_account = send_tx(
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

    send_tx(
        solana,
        TokenDepositInstruction {
            amount: provided_amount,
            account: provider_account,
            token_account: payer_mint0_account,
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenDepositInstruction {
            amount: provided_amount,
            account: provider_account,
            token_account: payer_mint1_account,
            token_authority: payer.clone(),
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // create thes test user account
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

    //
    // TEST: Deposit funds
    //
    let deposit_amount_initial = 100;
    {
        let start_balance = solana.token_account_balance(payer_mint0_account).await;

        send_tx(
            solana,
            TokenDepositInstruction {
                amount: deposit_amount_initial,
                account,
                token_account: payer_mint0_account,
                token_authority: payer.clone(),
                bank_index: 0,
            },
        )
        .await
        .unwrap();

        assert_eq!(
            solana.token_account_balance(vault).await,
            provided_amount + deposit_amount_initial
        );
        assert_eq!(
            solana.token_account_balance(payer_mint0_account).await,
            start_balance - deposit_amount_initial
        );
        assert_eq!(
            account_position(solana, account, bank).await,
            deposit_amount_initial as i64,
        );
    }

    //
    // TEST: Margin trade
    //
    let margin_account = payer_mint0_account;
    let margin_account_initial = solana.token_account_balance(margin_account).await;
    let target_token_account = context.users[0].token_accounts[0];
    let withdraw_amount = 2;
    let deposit_amount = 1;
    let send_flash_loan_tx = |solana, withdraw_amount, deposit_amount| async move {
        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction(FlashLoanBeginInstruction {
            group,
            mango_token_bank: bank,
            mango_token_vault: vault,
            target_token_account,
            withdraw_amount,
        })
        .await;
        if withdraw_amount > 0 {
            tx.add_instruction_direct(
                spl_token::instruction::transfer(
                    &spl_token::ID,
                    &target_token_account,
                    &margin_account,
                    &owner.pubkey(),
                    &[&owner.pubkey()],
                    withdraw_amount,
                )
                .unwrap(),
            );
        }
        if deposit_amount > 0 {
            tx.add_instruction_direct(
                spl_token::instruction::transfer(
                    &spl_token::ID,
                    &margin_account,
                    &target_token_account,
                    &payer.pubkey(),
                    &[&payer.pubkey()],
                    deposit_amount,
                )
                .unwrap(),
            );
            tx.add_signer(&payer);
        }
        tx.add_instruction(FlashLoanEndInstruction {
            account,
            owner,
            mango_token_bank: bank,
            mango_token_vault: vault,
            target_token_account,
            swap_indicator: false,
        })
        .await;
        tx.send().await.unwrap();
    };
    send_flash_loan_tx(solana, withdraw_amount, deposit_amount).await;

    assert_eq!(
        solana.token_account_balance(vault).await,
        provided_amount + deposit_amount_initial - withdraw_amount + deposit_amount
    );
    assert_eq!(
        solana.token_account_balance(margin_account).await,
        margin_account_initial + withdraw_amount - deposit_amount
    );
    // no fee because user had positive balance
    assert!(balance_f64eq(
        account_position_f64(solana, account, bank).await,
        (deposit_amount_initial - withdraw_amount + deposit_amount) as f64
    ));

    //
    // TEST: Bringing the balance to 0 deactivates the token
    //
    let deposit_amount_initial = account_position(solana, account, bank).await;
    let margin_account_initial = solana.token_account_balance(margin_account).await;
    let withdraw_amount = deposit_amount_initial as u64;
    let deposit_amount = 0;
    send_flash_loan_tx(solana, withdraw_amount, deposit_amount).await;
    assert_eq!(solana.token_account_balance(vault).await, provided_amount);
    assert_eq!(
        solana.token_account_balance(margin_account).await,
        margin_account_initial + withdraw_amount
    );
    // Check that position is fully deactivated
    let account_data = get_mango_account(solana, account).await;
    assert_eq!(account_data.token_iter_active().count(), 0);

    //
    // TEST: Activating a token via margin trade
    //
    let margin_account_initial = solana.token_account_balance(margin_account).await;
    let withdraw_amount = 0;
    let deposit_amount = 100;
    send_flash_loan_tx(solana, withdraw_amount, deposit_amount).await;
    assert_eq!(
        solana.token_account_balance(vault).await,
        provided_amount + deposit_amount
    );
    assert_eq!(
        solana.token_account_balance(margin_account).await,
        margin_account_initial - deposit_amount
    );
    assert!(balance_f64eq(
        account_position_f64(solana, account, bank).await,
        deposit_amount as f64
    ));

    //
    // TEST: Try loan fees by withdrawing more than the user balance
    //
    let margin_account_initial = solana.token_account_balance(margin_account).await;
    let deposit_amount_initial = account_position(solana, account, bank).await as u64;
    let withdraw_amount = 500;
    let deposit_amount = 450;
    println!("{}", deposit_amount_initial);
    send_flash_loan_tx(solana, withdraw_amount, deposit_amount).await;
    assert_eq!(
        solana.token_account_balance(vault).await,
        provided_amount + deposit_amount_initial + deposit_amount - withdraw_amount
    );
    assert_eq!(
        solana.token_account_balance(margin_account).await,
        margin_account_initial + withdraw_amount - deposit_amount
    );
    assert!(balance_f64eq(
        account_position_f64(solana, account, bank).await,
        (deposit_amount_initial + deposit_amount - withdraw_amount) as f64
            - (withdraw_amount - deposit_amount_initial) as f64 * loan_origination_fee
    ));

    Ok(())
}
