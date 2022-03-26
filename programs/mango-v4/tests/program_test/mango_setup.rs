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
    pub vault: Pubkey,
}

pub struct GroupWithTokens {
    pub group: Pubkey,
    pub tokens: Vec<Token>,
}

impl<'a> GroupWithTokensConfig<'a> {
    pub async fn create(self, solana: &SolanaCookie) -> GroupWithTokens {
        let GroupWithTokensConfig {
            admin,
            payer,
            mints,
        } = self;
        let group = send_tx(solana, CreateGroupInstruction { admin, payer })
            .await
            .unwrap()
            .group;

        let address_lookup_table = solana.create_address_lookup_table(admin, payer).await;

        let mut tokens = vec![];
        for (index, mint) in mints.iter().enumerate() {
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
            let token_index = index as u16;
            let register_token_accounts = send_tx(
                solana,
                RegisterTokenInstruction {
                    token_index,
                    decimals: mint.decimals,
                    maint_asset_weight: 0.9,
                    init_asset_weight: 0.8,
                    maint_liab_weight: 1.1,
                    init_liab_weight: 1.2,
                    liquidation_fee: 0.0,
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
            let vault = register_token_accounts.vault;

            tokens.push(Token {
                index: token_index,
                mint: mint.clone(),
                oracle,
                bank,
                vault,
            });
        }

        GroupWithTokens { group, tokens }
    }
}
