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
pub fn fetch_top(
    context: &crate::context::MangoGroupContext,
    account_fetcher: &impl AccountFetcher,
    perp_market_index: PerpMarketIndex,
    perp_market_address: &Pubkey,
    direction: Direction,
    count: usize,
) -> anyhow::Result<Vec<(Pubkey, MangoAccountValue, I80F48)>> {
    let perp_market =
        account_fetcher_fetch_anchor_account::<PerpMarket>(account_fetcher, perp_market_address)?;
    let oracle_acc = account_fetcher.fetch_raw_account(&perp_market.oracle)?;
    let oracle_price =
        perp_market.oracle_price(&KeyedAccountSharedData::new(perp_market.oracle, oracle_acc))?;

    let accounts =
        account_fetcher.fetch_program_accounts(&mango_v4::id(), MangoAccount::discriminator())?;

    let mut accounts_pnl = accounts
        .iter()
        .filter_map(|(pk, acc)| {
            let data = acc.data();
            let mango_acc = MangoAccountValue::from_bytes(&data[8..]);
            if mango_acc.is_err() {
                return None;
            }
            let mango_acc = mango_acc.unwrap();
            let perp_pos = mango_acc.perp_position(perp_market_index);
            if perp_pos.is_err() {
                return None;
            }
            let perp_pos = perp_pos.unwrap();
            let pnl = perp_pos.base_position_native(&perp_market) * oracle_price
                + perp_pos.quote_position_native();
            if pnl >= 0 && direction == Direction::MaxNegative
                || pnl <= 0 && direction == Direction::MaxPositive
            {
                return None;
            }
            Some((*pk, mango_acc, pnl))
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

    // Negative pnl needs to be limited by spot_health.
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
            let spot_health = crate::health_cache::new(context, account_fetcher, &acc)?
                .spot_health(HealthType::Maint);
            let settleable_pnl = if spot_health > 0 && !acc.being_liquidated() {
                (*pnl).max(-spot_health)
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
    Ok(accounts_pnl[0..count].to_vec())
}
