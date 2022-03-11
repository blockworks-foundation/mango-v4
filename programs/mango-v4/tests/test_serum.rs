#![cfg(feature = "test-bpf")]

use anchor_lang::InstructionData;
use fixed::types::I80F48;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::Signer;
use solana_sdk::{signature::Keypair, transport::TransportError};
use std::str::FromStr;

use mango_v4::state::*;
use program_test::*;

mod program_test;

#[tokio::test]
async fn test_serum() -> Result<(), TransportError> {
    let context = TestContext::new(Option::None, Option::None, Option::None, Option::None).await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let owner = &context.users[0].key;
    let payer = &context.users[1].key;
    let mint0 = &context.mints[0];
    let mint1 = &context.mints[1];
    let payer_mint0_account = context.users[1].token_accounts[0];

    //
    // SETUP: Create a group
    //

    let group = send_tx(solana, CreateGroupInstruction { admin, payer })
        .await
        .unwrap()
        .group;

    //
    // SETUP: Register mints (and make oracles for them)
    //

    let register_mint = |mint: MintCookie, address_lookup_table: Pubkey| async move {
        let create_stub_oracle_accounts = send_tx(
            solana,
            CreateStubOracle {
                mint: mint.pubkey,
                payer,
            },
        )
        .await
        .unwrap();
        let oracle = create_stub_oracle_accounts.oracle;
        send_tx(
            solana,
            SetStubOracle {
                mint: mint.pubkey,
                payer,
                price: "1.0",
            },
        )
        .await
        .unwrap();
        let register_token_accounts = send_tx(
            solana,
            RegisterTokenInstruction {
                decimals: mint.decimals,
                maint_asset_weight: 0.9,
                init_asset_weight: 0.8,
                maint_liab_weight: 1.1,
                init_liab_weight: 1.2,
                group,
                admin,
                mint: mint.pubkey,
                address_lookup_table,
                payer,
            },
        )
        .await
        .unwrap();
        let bank = register_token_accounts.bank;

        (oracle, bank)
    };

    let address_lookup_table = solana.create_address_lookup_table(admin, payer).await;
    let (oracle0, bank0) = register_mint(mint0.clone(), address_lookup_table).await;
    let (oracle1, bank1) = register_mint(mint1.clone(), address_lookup_table).await;

    //
    // TEST: Register a serum market
    //
    send_tx(
        solana,
        RegisterSerumMarketInstruction {
            group,
            admin,
            serum_program: Pubkey::default(),
            serum_market_external: Pubkey::default(),
            base_token_index: 0, // TODO: better way of getting these numbers
            quote_token_index: 1,
            payer,
        },
    )
    .await
    .unwrap();

    Ok(())
}
