use anchor_lang::Discriminator;
use arrayref::array_ref;

use mango_v4::state::{Bank, MangoAccount, MangoAccountAccWithHeader, MintInfo, PerpMarket};

use solana_sdk::account::{AccountSharedData, ReadableAccount};
use solana_sdk::pubkey::Pubkey;

pub fn is_mango_account<'a>(
    account: &'a AccountSharedData,
    program_id: &Pubkey,
    group_id: &Pubkey,
) -> Option<MangoAccountAccWithHeader<'a>> {
    let data = account.data();
    if account.owner() != program_id || data.is_empty() {
        return None;
    }

    let disc_bytes = array_ref![data, 0, 8];
    if disc_bytes != &MangoAccount::discriminator() {
        return None;
    }

    let mango_account = MangoAccountAccWithHeader::from_bytes(&data[8..]).expect("always ok");
    if mango_account.fixed.group != *group_id {
        return None;
    }
    Some(mango_account)
}

pub fn is_mango_bank<'a>(
    account: &'a AccountSharedData,
    program_id: &Pubkey,
    group_id: &Pubkey,
) -> Option<&'a Bank> {
    let data = account.data();
    if account.owner() != program_id || data.is_empty() {
        return None;
    }

    let disc_bytes = array_ref![data, 0, 8];
    if disc_bytes != &Bank::discriminator() {
        return None;
    }
    if data.len() != 8 + std::mem::size_of::<Bank>() {
        return None;
    }
    let bank: &Bank = bytemuck::try_from_bytes(&data[8..]).expect("always Ok");
    if bank.group != *group_id {
        return None;
    }
    Some(bank)
}

pub fn is_mint_info<'a>(
    account: &'a AccountSharedData,
    program_id: &Pubkey,
    group_id: &Pubkey,
) -> Option<&'a MintInfo> {
    let data = account.data();
    if account.owner() != program_id || data.is_empty() {
        return None;
    }

    let disc_bytes = array_ref![data, 0, 8];
    if disc_bytes != &MintInfo::discriminator() {
        return None;
    }
    if data.len() != 8 + std::mem::size_of::<MintInfo>() {
        return None;
    }
    let mint_info: &MintInfo = bytemuck::try_from_bytes(&data[8..]).expect("always Ok");
    if mint_info.group != *group_id {
        return None;
    }
    Some(mint_info)
}

pub fn is_perp_market<'a>(
    account: &'a AccountSharedData,
    program_id: &Pubkey,
    group_id: &Pubkey,
) -> Option<&'a PerpMarket> {
    let data = account.data();
    if account.owner() != program_id || data.is_empty() {
        return None;
    }

    let disc_bytes = array_ref![data, 0, 8];
    if disc_bytes != &PerpMarket::discriminator() {
        return None;
    }
    if data.len() != 8 + std::mem::size_of::<PerpMarket>() {
        return None;
    }
    let perp_market: &PerpMarket = bytemuck::try_from_bytes(&data[8..]).expect("always Ok");
    if perp_market.group != *group_id {
        return None;
    }
    Some(perp_market)
}
