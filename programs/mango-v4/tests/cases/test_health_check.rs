use crate::cases::{
    create_funded_account, mango_setup, send_tx, tokio, HealthAccountSkipping,
    HealthCheckInstruction, TestContext, TestKeypair, TokenWithdrawInstruction,
};
use crate::send_tx_expect_error;
use mango_v4::accounts_ix::{HealthCheck, HealthCheckKind};
use mango_v4::error::MangoError;
use solana_sdk::transport::TransportError;

#[tokio::test]
async fn test_health_check() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let payer_token_accounts = &context.users[1].token_accounts;
    let mints = &context.mints[0..3];

    let mango_setup::GroupWithTokens { group, tokens, .. } = mango_setup::GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        zero_token_is_quote: true,
        ..mango_setup::GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    // Funding to fill the vaults
    create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        &mints,
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
        1000,
        0,
    )
    .await;

    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 775,
            allow_borrow: true,
            account,
            owner,
            token_account: payer_token_accounts[2],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // TEST (Health is about 93% with all banks, 7% without banks 1)
    //

    send_tx(
        solana,
        HealthCheckInstruction {
            account,
            owner,
            min_health_value: 20.0,
            check_kind: HealthCheckKind::MaintRatio,
        },
    )
    .await
    .unwrap();

    send_tx(
        solana,
        HealthCheckInstruction {
            account,
            owner,
            min_health_value: 500.0,
            check_kind: HealthCheckKind::Init,
        },
    )
    .await
    .unwrap();

    send_tx_expect_error!(
        solana,
        HealthCheckInstruction {
            owner,
            account,
            min_health_value: 600.0,
            check_kind: HealthCheckKind::Init,
        },
        MangoError::InvalidHealth
    );

    send_tx(
        solana,
        HealthCheckInstruction {
            account,
            owner,
            min_health_value: 800.0,
            check_kind: HealthCheckKind::Maint,
        },
    )
    .await
    .unwrap();

    send_tx_expect_error!(
        solana,
        HealthCheckInstruction {
            owner,
            account,
            min_health_value: 100.0,
            check_kind: HealthCheckKind::MaintRatio,
        },
        MangoError::InvalidHealth
    );

    send_tx(
        solana,
        HealthAccountSkipping {
            inner: HealthCheckInstruction {
                owner,
                account,
                min_health_value: 5.0,
                check_kind: HealthCheckKind::MaintRatio,
            },
            skip_banks: vec![tokens[1].bank],
        },
    )
    .await
    .unwrap();

    send_tx_expect_error!(
        solana,
        HealthAccountSkipping {
            inner: HealthCheckInstruction {
                owner,
                account,
                min_health_value: 10.0,
                check_kind: HealthCheckKind::MaintRatio,
            },
            skip_banks: vec![tokens[1].bank],
        },
        MangoError::InvalidHealth
    );

    send_tx_expect_error!(
        solana,
        HealthAccountSkipping {
            inner: HealthCheckInstruction {
                owner,
                account,
                min_health_value: 10.0,
                check_kind: HealthCheckKind::MaintRatio,
            },
            skip_banks: vec![tokens[2].bank],
        },
        MangoError::InvalidBank
    );

    Ok(())
}
