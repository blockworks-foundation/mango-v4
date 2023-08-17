use mango_v4::accounts_zerocopy::*;
use mango_v4::state::{Bank, MangoAccountValue, MintInfo, PerpMarket, TokenIndex};

use anyhow::Context;
use fixed::types::I80F48;

use solana_sdk::account::AccountSharedData;
use solana_sdk::pubkey::Pubkey;

pub use mango_v4_client::snapshot_source::is_mango_account;
use mango_v4_client::{chain_data, JupiterSwapMode, MangoClient};

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

/// A wrapper that can mock the response
pub async fn jupiter_route(
    mango_client: &MangoClient,
    input_mint: Pubkey,
    output_mint: Pubkey,
    amount: u64,
    slippage: u64,
    swap_mode: JupiterSwapMode,
    only_direct_routes: bool,
    mock: bool,
) -> anyhow::Result<mango_v4_client::jupiter::Quote> {
    if !mock {
        return mango_client
            .jupiter_route(
                input_mint,
                output_mint,
                amount,
                slippage,
                swap_mode,
                only_direct_routes,
            )
            .await;
    }

    // TODO: elevate this mock to client.rs
    let input_price = mango_client
        .bank_oracle_price(mango_client.context.token_by_mint(&input_mint)?.token_index)
        .await?;
    let output_price = mango_client
        .bank_oracle_price(
            mango_client
                .context
                .token_by_mint(&output_mint)?
                .token_index,
        )
        .await?;
    let in_amount: u64;
    let out_amount: u64;
    let other_amount_threshold: u64;
    let swap_mode_str;
    match swap_mode {
        JupiterSwapMode::ExactIn => {
            in_amount = amount;
            out_amount = (I80F48::from(amount) * input_price / output_price).to_num();
            other_amount_threshold = out_amount;
            swap_mode_str = "ExactIn".to_string();
        }
        JupiterSwapMode::ExactOut => {
            in_amount = (I80F48::from(amount) * output_price / input_price).to_num();
            out_amount = amount;
            other_amount_threshold = in_amount;
            swap_mode_str = "ExactOut".to_string();
        }
    }

    Ok(mango_v4_client::jupiter::QueryRoute {
        in_amount: in_amount.to_string(),
        out_amount: out_amount.to_string(),
        price_impact_pct: 0.1,
        market_infos: vec![],
        amount: amount.to_string(),
        slippage_bps: 1,
        other_amount_threshold: other_amount_threshold.to_string(),
        swap_mode: swap_mode_str,
        fees: None,
    })
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
