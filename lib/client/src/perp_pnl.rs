use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use fixed::types::I80F48;
use solana_sdk::account::ReadableAccount;

use crate::*;
use mango_v4::accounts_zerocopy::KeyedAccountSharedData;
use mango_v4::state::*;

#[derive(Debug, PartialEq)]
pub enum Direction {
    MaxPositive,
    MaxNegative,
}

/// Returns up to `count` accounts with highest abs pnl (by `direction`) in descending order.
/// Note: keep in sync with perp.ts:getSettlePnlCandidates
pub async fn fetch_top(
    context: &crate::context::MangoGroupContext,
    fallback_config: &FallbackOracleConfig,
    account_fetcher: &impl AccountFetcher,
    perp_market_index: PerpMarketIndex,
    direction: Direction,
    count: usize,
) -> anyhow::Result<Vec<(Pubkey, MangoAccountValue, I80F48)>> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_ts: u64 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let perp = context.perp(perp_market_index);
    let perp_market =
        account_fetcher_fetch_anchor_account::<PerpMarket>(account_fetcher, &perp.address).await?;
    let oracle = account_fetcher
        .fetch_raw_account(&perp_market.oracle)
        .await?;
    let oracle_acc = &KeyedAccountSharedData::new(perp.oracle, oracle.into());
    let oracle_price =
        perp_market.oracle_price(&&OracleAccountInfos::from_reader(oracle_acc), None)?;

    let accounts = account_fetcher
        .fetch_program_accounts(&mango_v4::id(), MangoAccount::discriminator())
        .await?;

    let mut accounts_pnl = accounts
        .iter()
        .filter_map(|(pk, acc)| {
            let data = acc.data();
            let mango_acc = MangoAccountValue::from_bytes(&data[8..]);
            if mango_acc.is_err() {
                return None;
            }
            let mango_acc = mango_acc.unwrap();
            if mango_acc.fixed.group != perp_market.group {
                return None;
            }
            let perp_pos = mango_acc.perp_position(perp_market_index);
            if perp_pos.is_err() {
                return None;
            }
            let mut perp_pos = perp_pos.unwrap().clone();
            perp_pos.settle_funding(&perp_market);
            perp_pos.update_settle_limit(&perp_market, now_ts);
            let pnl = perp_pos.unsettled_pnl(&perp_market, oracle_price).unwrap();
            let limited_pnl = perp_pos.apply_pnl_settle_limit(&perp_market, pnl);
            if limited_pnl >= 0 && direction == Direction::MaxNegative
                || limited_pnl <= 0 && direction == Direction::MaxPositive
            {
                return None;
            }
            Some((*pk, mango_acc, limited_pnl))
        })
        .collect::<Vec<_>>();

    // Sort the top accounts to the front
    match direction {
        Direction::MaxPositive => {
            accounts_pnl.sort_by(|a, b| b.2.cmp(&a.2));
        }
        Direction::MaxNegative => {
            accounts_pnl.sort_by(|a, b| a.2.cmp(&b.2));
        }
    }

    // Negative pnl needs to be limited by perp_max_settle.
    // We're doing it in a second step, because it's pretty expensive and we don't
    // want to run this for all accounts.
    if direction == Direction::MaxNegative {
        let mut stable = 0;
        for i in 0..accounts_pnl.len() {
            let (_, acc, pnl) = &accounts_pnl[i];
            let next_pnl = if i + 1 < accounts_pnl.len() {
                accounts_pnl[i + 1].2
            } else {
                I80F48::ZERO
            };
            let perp_max_settle =
                crate::health_cache::new(context, fallback_config, account_fetcher, &acc)
                    .await?
                    .perp_max_settle(perp_market.settle_token_index)?;
            let settleable_pnl = if perp_max_settle > 0 {
                (*pnl).max(-perp_max_settle)
            } else {
                I80F48::ZERO
            };
            accounts_pnl[i].2 = settleable_pnl;

            // if the ordering was unchanged `count` times we know we have the top `count` accounts
            if settleable_pnl <= next_pnl {
                stable += 1;
                if stable >= count {
                    break;
                }
            }
        }
        accounts_pnl.sort_by(|a, b| a.2.cmp(&b.2));
    }

    // return highest abs pnl accounts
    Ok(accounts_pnl.into_iter().take(count).collect::<Vec<_>>())
}
