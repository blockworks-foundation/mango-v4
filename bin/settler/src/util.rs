use mango_v4::accounts_zerocopy::*;
use mango_v4::state::{Bank, MintInfo, PerpMarket};

use solana_sdk::account::AccountSharedData;
use solana_sdk::pubkey::Pubkey;

pub use mango_v4_client::snapshot_source::is_mango_account;

pub fn is_mango_bank<'a>(account: &'a AccountSharedData, group_id: &Pubkey) -> Option<&'a Bank> {
    let bank = account.load::<Bank>().ok()?;
    if bank.group != *group_id {
        return None;
    }
    Some(bank)
}

pub fn is_mint_info<'a>(account: &'a AccountSharedData, group_id: &Pubkey) -> Option<&'a MintInfo> {
    let mint_info = account.load::<MintInfo>().ok()?;
    if mint_info.group != *group_id {
        return None;
    }
    Some(mint_info)
}

pub fn is_perp_market<'a>(
    account: &'a AccountSharedData,
    group_id: &Pubkey,
) -> Option<&'a PerpMarket> {
    let perp_market = account.load::<PerpMarket>().ok()?;
    if perp_market.group != *group_id {
        return None;
    }
    Some(perp_market)
}
