#![allow(dead_code)]

use anchor_lang::prelude::*;
use solana_sdk::signature::Keypair;

use super::mango_client::*;
use super::solana::SolanaCookie;
use super::{send_tx, MintCookie};

pub struct GroupWithTokensConfig<'a> {
    pub admin: &'a Keypair,
    pub payer: &'a Keypair,
    pub mints: &'a [MintCookie],
}

pub struct Token {
    pub index: u16,
    pub mint: MintCookie,
    pub oracle: Pubkey,
    pub bank: Pubkey,
    pub bank1: Pubkey,
    pub vault: Pubkey,
    pub mint_info: Pubkey,
}

pub struct GroupWithTokens {
    pub group: Pubkey,
    pub insurance_vault: Pubkey,
    pub tokens: Vec<Token>,
}

impl<'a> GroupWithTokensConfig<'a> {
    pub async fn create(self, solana: &SolanaCookie) -> GroupWithTokens {
        let GroupWithTokensConfig {
            admin,
            payer,
            mints,
        } = self;
        let create_group_accounts = send_tx(
            solana,
            CreateGroupInstruction {
                admin,
                payer,
                insurance_mint: mints[0].pubkey,
            },
        )
        .await
        .unwrap();
        let group = create_group_accounts.group;
        let insurance_vault = create_group_accounts.insurance_vault;

        let address_lookup_table = solana.create_address_lookup_table(admin, payer).await;

        let mut tokens = vec![];
        for (index, mint) in mints.iter().enumerate() {
            let create_stub_oracle_accounts = send_tx(
                solana,
                StubOracleCreate {
                    group,
                    mint: mint.pubkey,
                    admin,
                    payer,
                },
            )
            .await
            .unwrap();
            let oracle = create_stub_oracle_accounts.oracle;
            send_tx(
                solana,
                StubOracleSetInstruction {
                    group,
                    admin,
                    mint: mint.pubkey,
                    payer,
                    price: "1.0",
                },
            )
            .await
            .unwrap();
            let token_index = index as u16;
            let register_token_accounts = send_tx(
                solana,
                TokenRegisterInstruction {
                    token_index,
                    decimals: mint.decimals,
                    util0: 0.40,
                    rate0: 0.07,
                    util1: 0.80,
                    rate1: 0.9,
                    max_rate: 1.50,
                    loan_origination_fee_rate: 0.0005,
                    loan_fee_rate: 0.0005,
                    maint_asset_weight: 0.8,
                    init_asset_weight: 0.6,
                    maint_liab_weight: 1.2,
                    init_liab_weight: 1.4,
                    liquidation_fee: 0.02,
                    group,
                    admin,
                    mint: mint.pubkey,
                    address_lookup_table,
                    payer,
                },
            )
            .await
            .unwrap();
            let add_bank_accounts = send_tx(
                solana,
                TokenAddBankInstruction {
                    token_index,
                    bank_num: 1,
                    group,
                    admin,
                    address_lookup_table,
                    payer,
                },
            )
            .await
            .unwrap();
            let bank = register_token_accounts.bank;
            let vault = register_token_accounts.vault;
            let mint_info = register_token_accounts.mint_info;

            tokens.push(Token {
                index: token_index,
                mint: mint.clone(),
                oracle,
                bank,
                bank1: add_bank_accounts.bank,
                vault,
                mint_info,
            });
        }

        GroupWithTokens {
            group,
            insurance_vault,
            tokens,
        }
    }
}
