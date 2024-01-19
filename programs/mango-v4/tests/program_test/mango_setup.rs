#![allow(dead_code)]

use anchor_lang::prelude::*;

use super::mango_client::*;
use super::solana::SolanaCookie;
use super::{send_tx, MintCookie, TestKeypair, UserCookie};

#[derive(Default)]
pub struct GroupWithTokensConfig {
    pub admin: TestKeypair,
    pub payer: TestKeypair,
    pub mints: Vec<MintCookie>,
    pub zero_token_is_quote: bool,
}

#[derive(Clone)]
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
    pub admin: TestKeypair,
    pub insurance_vault: Pubkey,
    pub tokens: Vec<Token>,
}

impl<'a> GroupWithTokensConfig {
    pub async fn create(self, solana: &SolanaCookie) -> GroupWithTokens {
        let GroupWithTokensConfig {
            admin,
            payer,
            mints,
            zero_token_is_quote,
        } = self;
        let create_group_accounts = send_tx(
            solana,
            GroupCreateInstruction {
                creator: admin,
                payer,
                insurance_mint: mints[0].pubkey,
            },
        )
        .await
        .unwrap();
        let group = create_group_accounts.group;
        let insurance_vault = create_group_accounts.insurance_vault;

        let mut tokens = vec![];
        for (index, mint) in mints.iter().enumerate() {
            let create_stub_oracle_accounts = send_tx(
                solana,
                StubOracleCreate {
                    oracle: TestKeypair::new(),
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
                    price: 1.0,
                    oracle,
                },
            )
            .await
            .unwrap();
            let token_index = index as u16;
            let (iaw, maw, mlw, ilw) = if token_index == 0 && zero_token_is_quote {
                (1.0, 1.0, 1.0, 1.0)
            } else {
                (0.6, 0.8, 1.2, 1.4)
            };
            let register_token_accounts = send_tx(
                solana,
                TokenRegisterInstruction {
                    token_index,
                    decimals: mint.decimals,
                    adjustment_factor: 0.01,
                    util0: 0.40,
                    rate0: 0.07,
                    util1: 0.80,
                    rate1: 0.9,
                    max_rate: 1.50,
                    loan_origination_fee_rate: 0.0005,
                    loan_fee_rate: 0.0005,
                    maint_asset_weight: maw,
                    init_asset_weight: iaw,
                    maint_liab_weight: mlw,
                    init_liab_weight: ilw,
                    liquidation_fee: 0.02,
                    group,
                    admin,
                    oracle,
                    mint: mint.pubkey,
                    payer,
                    min_vault_to_deposits_ratio: 0.2,
                    net_borrow_limit_per_window_quote: 1_000_000_000_000,
                    net_borrow_limit_window_size_ts: 24 * 60 * 60,
                    platform_liquidation_fee: 0.0,
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
            admin,
            insurance_vault,
            tokens,
        }
    }
}

pub async fn create_funded_account(
    solana: &SolanaCookie,
    group: Pubkey,
    owner: TestKeypair,
    account_num: u32,
    payer: &UserCookie,
    mints: &[MintCookie],
    amounts: u64,
    bank_index: usize,
) -> Pubkey {
    let account = send_tx(
        solana,
        AccountCreateInstruction {
            account_num,
            group,
            owner,
            payer: payer.key,
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .account;

    for mint in mints {
        send_tx(
            solana,
            TokenDepositInstruction {
                amount: amounts,
                reduce_only: false,
                account,
                owner,
                token_account: payer.token_accounts[mint.index],
                token_authority: payer.key,
                bank_index,
            },
        )
        .await
        .unwrap();
    }

    account
}
