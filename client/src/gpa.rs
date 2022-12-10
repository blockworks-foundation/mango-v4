use anchor_client::{ClientError, Program};
use anchor_lang::Discriminator;

use mango_v4::state::{Bank, MangoAccount, MangoAccountValue, MintInfo, PerpMarket, Serum3Market};

use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::pubkey::Pubkey;

pub fn fetch_mango_accounts(
    program: &Program,
    group: Pubkey,
    owner: Pubkey,
) -> Result<Vec<(Pubkey, MangoAccountValue)>, ClientError> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                0,
                &MangoAccount::discriminator(),
            )),
            RpcFilterType::Memcmp(Memcmp::new_base58_encoded(8, &group.to_bytes())),
            RpcFilterType::Memcmp(Memcmp::new_base58_encoded(40, &owner.to_bytes())),
        ]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    program
        .rpc()
        .get_program_accounts_with_config(&program.id(), config)?
        .into_iter()
        .map(|(key, account)| Ok((key, MangoAccountValue::from_bytes(&account.data[8..])?)))
        .collect::<Result<Vec<_>, _>>()
}

pub fn fetch_banks(program: &Program, group: Pubkey) -> Result<Vec<(Pubkey, Bank)>, ClientError> {
    program.accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        8,
        &group.to_bytes(),
    ))])
}

pub fn fetch_mint_infos(
    program: &Program,
    group: Pubkey,
) -> Result<Vec<(Pubkey, MintInfo)>, ClientError> {
    program.accounts::<MintInfo>(vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        8,
        &group.to_bytes(),
    ))])
}

pub fn fetch_serum3_markets(
    program: &Program,
    group: Pubkey,
) -> Result<Vec<(Pubkey, Serum3Market)>, ClientError> {
    program.accounts::<Serum3Market>(vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        8,
        &group.to_bytes(),
    ))])
}

pub fn fetch_perp_markets(
    program: &Program,
    group: Pubkey,
) -> Result<Vec<(Pubkey, PerpMarket)>, ClientError> {
    program.accounts::<PerpMarket>(vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        8,
        &group.to_bytes(),
    ))])
}
