#![cfg(feature = "test-bpf")]

use anchor_lang::InstructionData;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;

use mango_v4::state::*;
use program_test::*;

mod program_test;

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_margin_trade1() -> Result<(), BanksClientError> {
    let mut builder = TestContextBuilder::new();
    let margin_trade = builder.add_margin_trade_program();
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
        CreateAccountInstruction {
            account_num: 1,
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
        CreateAccountInstruction {
            account_num: 0,
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
    let withdraw_amount = 2;
    let deposit_amount = 1;
    {
        send_tx(
            solana,
            FlashLoanInstruction {
                account,
                owner,
                mango_token_bank: bank,
                mango_token_vault: vault,
                withdraw_amount,
                margin_trade_program_id: margin_trade.program,
                deposit_account: margin_trade.token_account.pubkey(),
                deposit_account_owner: margin_trade.token_account_owner,
                margin_trade_program_ix_cpi_data: {
                    let ix = margin_trade::instruction::MarginTrade {
                        amount_from: withdraw_amount,
                        amount_to: deposit_amount,
                        deposit_account_owner_bump_seeds: margin_trade.token_account_bump,
                    };
                    ix.data()
                },
            },
        )
        .await
        .unwrap();
    }
    assert_eq!(
        solana.token_account_balance(vault).await,
        provided_amount + deposit_amount_initial - withdraw_amount + deposit_amount
    );
    assert_eq!(
        solana
            .token_account_balance(margin_trade.token_account.pubkey())
            .await,
        withdraw_amount - deposit_amount
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
    let margin_account_initial = solana
        .token_account_balance(margin_trade.token_account.pubkey())
        .await;
    let withdraw_amount = deposit_amount_initial as u64;
    let deposit_amount = 0;
    {
        send_tx(
            solana,
            FlashLoanInstruction {
                account,
                owner,
                mango_token_bank: bank,
                mango_token_vault: vault,
                withdraw_amount,
                margin_trade_program_id: margin_trade.program,
                deposit_account: margin_trade.token_account.pubkey(),
                deposit_account_owner: margin_trade.token_account_owner,
                margin_trade_program_ix_cpi_data: {
                    let ix = margin_trade::instruction::MarginTrade {
                        amount_from: withdraw_amount,
                        amount_to: deposit_amount,
                        deposit_account_owner_bump_seeds: margin_trade.token_account_bump,
                    };
                    ix.data()
                },
            },
        )
        .await
        .unwrap();
    }
    assert_eq!(solana.token_account_balance(vault).await, provided_amount);
    assert_eq!(
        solana
            .token_account_balance(margin_trade.token_account.pubkey())
            .await,
        margin_account_initial + withdraw_amount
    );
    // Check that position is fully deactivated
    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens.iter_active().count(), 0);

    //
    // TEST: Activating a token via margin trade
    //
    let margin_account_initial = solana
        .token_account_balance(margin_trade.token_account.pubkey())
        .await;
    let withdraw_amount = 0;
    let deposit_amount = margin_account_initial;
    {
        send_tx(
            solana,
            FlashLoanInstruction {
                account,
                owner,
                mango_token_bank: bank,
                mango_token_vault: vault,
                withdraw_amount,
                margin_trade_program_id: margin_trade.program,
                deposit_account: margin_trade.token_account.pubkey(),
                deposit_account_owner: margin_trade.token_account_owner,
                margin_trade_program_ix_cpi_data: {
                    let ix = margin_trade::instruction::MarginTrade {
                        amount_from: withdraw_amount,
                        amount_to: deposit_amount,
                        deposit_account_owner_bump_seeds: margin_trade.token_account_bump,
                    };
                    ix.data()
                },
            },
        )
        .await
        .unwrap();
    }
    assert_eq!(
        solana.token_account_balance(vault).await,
        provided_amount + deposit_amount
    );
    assert_eq!(
        solana
            .token_account_balance(margin_trade.token_account.pubkey())
            .await,
        0
    );
    assert!(balance_f64eq(
        account_position_f64(solana, account, bank).await,
        deposit_amount as f64
    ));

    //
    // TEST: Try loan fees by withdrawing more than the user balance
    //
    let deposit_amount_initial = account_position(solana, account, bank).await as u64;
    let withdraw_amount = 500;
    let deposit_amount = 450;
    {
        send_tx(
            solana,
            FlashLoanInstruction {
                account,
                owner,
                mango_token_bank: bank,
                mango_token_vault: vault,
                withdraw_amount,
                margin_trade_program_id: margin_trade.program,
                deposit_account: margin_trade.token_account.pubkey(),
                deposit_account_owner: margin_trade.token_account_owner,
                margin_trade_program_ix_cpi_data: {
                    let ix = margin_trade::instruction::MarginTrade {
                        amount_from: withdraw_amount,
                        amount_to: deposit_amount,
                        deposit_account_owner_bump_seeds: margin_trade.token_account_bump,
                    };
                    ix.data()
                },
            },
        )
        .await
        .unwrap();
    }
    assert_eq!(
        solana.token_account_balance(vault).await,
        provided_amount + deposit_amount_initial + deposit_amount - withdraw_amount
    );
    assert_eq!(
        solana
            .token_account_balance(margin_trade.token_account.pubkey())
            .await,
        withdraw_amount - deposit_amount
    );
    assert!(balance_f64eq(
        account_position_f64(solana, account, bank).await,
        (deposit_amount_initial + deposit_amount - withdraw_amount) as f64
            - (withdraw_amount - deposit_amount_initial) as f64 * loan_origination_fee
    ));

    Ok(())
}

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_margin_trade2() -> Result<(), BanksClientError> {
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
        CreateAccountInstruction {
            account_num: 1,
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
        CreateAccountInstruction {
            account_num: 0,
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
    let withdraw_amount = 2;
    let deposit_amount = 1;
    let send_flash_loan_tx = |solana, withdraw_amount, deposit_amount| async move {
        let temporary_vault_authority = &Keypair::new();

        let mut tx = ClientTransaction::new(solana);
        tx.add_instruction(FlashLoan2BeginInstruction {
            group,
            temporary_vault_authority,
            mango_token_bank: bank,
            mango_token_vault: vault,
            withdraw_amount,
        })
        .await;
        if withdraw_amount > 0 {
            tx.add_instruction_direct(
                spl_token::instruction::transfer(
                    &spl_token::ID,
                    &vault,
                    &margin_account,
                    &temporary_vault_authority.pubkey(),
                    &[&temporary_vault_authority.pubkey()],
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
                    &vault,
                    &payer.pubkey(),
                    &[&payer.pubkey()],
                    deposit_amount,
                )
                .unwrap(),
            );
            tx.add_signer(&payer);
        }
        tx.add_instruction(FlashLoan2EndInstruction {
            account,
            owner,
            mango_token_bank: bank,
            mango_token_vault: vault,
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
    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens.iter_active().count(), 0);

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

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_margin_trade3() -> Result<(), BanksClientError> {
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
        CreateAccountInstruction {
            account_num: 1,
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
        CreateAccountInstruction {
            account_num: 0,
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
        tx.add_instruction(FlashLoan3BeginInstruction {
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
        tx.add_instruction(FlashLoan3EndInstruction {
            account,
            owner,
            mango_token_bank: bank,
            mango_token_vault: vault,
            target_token_account,
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
    let account_data: MangoAccount = solana.get_account(account).await;
    assert_eq!(account_data.tokens.iter_active().count(), 0);

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
