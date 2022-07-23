use anchor_client::{ClientError, Program};
use anchor_lang::Discriminator;

use mango_v4::state::{Bank, MangoAccount, MangoAccountValue, MintInfo, PerpMarket, Serum3Market};

use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_sdk::pubkey::Pubkey;

pub fn fetch_mango_accounts(
    program: &Program,
    group: Pubkey,
    owner: Pubkey,
) -> Result<Vec<(Pubkey, MangoAccountValue)>, ClientError> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::Memcmp(Memcmp {
                offset: 0,
                bytes: MemcmpEncodedBytes::Bytes(MangoAccount::discriminator().to_vec()),
                encoding: None,
            }),
            RpcFilterType::Memcmp(Memcmp {
                offset: 8,
                bytes: MemcmpEncodedBytes::Base58(group.to_string()),
                encoding: None,
            }),
            RpcFilterType::Memcmp(Memcmp {
                offset: 40,
                bytes: MemcmpEncodedBytes::Base58(owner.to_string()),
                encoding: None,
            }),
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
        .map(|(key, account)| Ok((key, MangoAccountValue::try_new(&account.data[8..])?)))
        .collect::<Result<Vec<_>, _>>()
}

pub fn fetch_banks(program: &Program, group: Pubkey) -> Result<Vec<(Pubkey, Bank)>, ClientError> {
    program.accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp {
        offset: 8,
        bytes: MemcmpEncodedBytes::Base58(group.to_string()),
        encoding: None,
    })])
}

pub fn fetch_mint_infos(
    program: &Program,
    group: Pubkey,
) -> Result<Vec<(Pubkey, MintInfo)>, ClientError> {
    program.accounts::<MintInfo>(vec![RpcFilterType::Memcmp(Memcmp {
        offset: 8,
        bytes: MemcmpEncodedBytes::Base58(group.to_string()),
        encoding: None,
    })])
}

pub fn fetch_serum3_markets(
    program: &Program,
    group: Pubkey,
) -> Result<Vec<(Pubkey, Serum3Market)>, ClientError> {
    program.accounts::<Serum3Market>(vec![RpcFilterType::Memcmp(Memcmp {
        offset: 8,
        bytes: MemcmpEncodedBytes::Base58(group.to_string()),
        encoding: None,
    })])
}

pub fn fetch_perp_markets(
    program: &Program,
    group: Pubkey,
) -> Result<Vec<(Pubkey, PerpMarket)>, ClientError> {
    program.accounts::<PerpMarket>(vec![RpcFilterType::Memcmp(Memcmp {
        offset: 8,
        bytes: MemcmpEncodedBytes::Base58(group.to_string()),
        encoding: None,
    })])
}
