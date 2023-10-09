use mango_v4::accounts_zerocopy::*;
use mango_v4::state::{Bank, MangoAccountValue, MintInfo, PerpMarket, TokenIndex};

use anyhow::Context;
use fixed::types::I80F48;

use solana_sdk::account::AccountSharedData;
use solana_sdk::pubkey::Pubkey;

pub use mango_v4_client::snapshot_source::is_mango_account;
use mango_v4_client::{chain_data, MangoClient};

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

/// Convenience wrapper for getting max swap amounts for a token pair
pub fn max_swap_source(
    client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    account: &MangoAccountValue,
    source: TokenIndex,
    target: TokenIndex,
    price: I80F48,
    min_health_ratio: I80F48,
) -> anyhow::Result<I80F48> {
    let mut account = account.clone();

    // Ensure the tokens are activated, so they appear in the health cache and
    // max_swap_source() will work.
    account.ensure_token_position(source)?;
    account.ensure_token_position(target)?;

    let health_cache =
        mango_v4_client::health_cache::new_sync(&client.context, account_fetcher, &account)
            .expect("always ok");

    let source_bank: Bank =
        account_fetcher.fetch(&client.context.mint_info(source).first_bank())?;
    let target_bank: Bank =
        account_fetcher.fetch(&client.context.mint_info(target).first_bank())?;

    let source_price = health_cache.token_info(source).unwrap().prices.oracle;

    let amount = health_cache
        .max_swap_source_for_health_ratio(
            &account,
            &source_bank,
            source_price,
            &target_bank,
            price,
            min_health_ratio,
        )
        .context("getting max_swap_source")?;
    Ok(amount)
}

/// Convenience wrapper for getting max swap amounts for a token pair
pub fn max_swap_source_ignore_net_borrows(
    client: &MangoClient,
    account_fetcher: &chain_data::AccountFetcher,
    account: &MangoAccountValue,
    source: TokenIndex,
    target: TokenIndex,
    price: I80F48,
    min_health_ratio: I80F48,
) -> anyhow::Result<I80F48> {
    let mut account = account.clone();

    // Ensure the tokens are activated, so they appear in the health cache and
    // max_swap_source() will work.
    account.ensure_token_position(source)?;
    account.ensure_token_position(target)?;

    let health_cache =
        mango_v4_client::health_cache::new_sync(&client.context, account_fetcher, &account)
            .expect("always ok");

    let mut source_bank: Bank =
        account_fetcher.fetch(&client.context.mint_info(source).first_bank())?;
    source_bank.net_borrow_limit_per_window_quote = -1;
    let mut target_bank: Bank =
        account_fetcher.fetch(&client.context.mint_info(target).first_bank())?;
    target_bank.net_borrow_limit_per_window_quote = -1;

    let source_price = health_cache.token_info(source).unwrap().prices.oracle;

    let amount = health_cache
        .max_swap_source_for_health_ratio(
            &account,
            &source_bank,
            source_price,
            &target_bank,
            price,
            min_health_ratio,
        )
        .context("getting max_swap_source")?;
    Ok(amount)
}
