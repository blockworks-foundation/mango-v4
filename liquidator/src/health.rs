use anchor_lang::prelude::*;
use fixed::types::I80F48;
use serum_dex::state::OpenOrders;
use solana_sdk::account::{AccountSharedData, ReadableAccount};

use mango_v4::error::MangoError;
use mango_v4::state::{
    determine_oracle_type, Bank, HealthType, MangoAccount, OracleType, PerpMarket, PerpMarketIndex,
    StubOracle, TokenIndex,
};

use crate::liquidate::load_mango_account;

//
//
//                ░░░░
//
//                                            ██
//                                          ██░░██
//  ░░          ░░                        ██░░░░░░██                            ░░░░
//                                      ██░░░░░░░░░░██
//                                      ██░░░░░░░░░░██
//                                    ██░░░░░░░░░░░░░░██
//                                  ██░░░░░░██████░░░░░░██
//                                  ██░░░░░░██████░░░░░░██
//                                ██░░░░░░░░██████░░░░░░░░██
//                                ██░░░░░░░░██████░░░░░░░░██
//                              ██░░░░░░░░░░██████░░░░░░░░░░██
//                            ██░░░░░░░░░░░░██████░░░░░░░░░░░░██
//                            ██░░░░░░░░░░░░██████░░░░░░░░░░░░██
//                          ██░░░░░░░░░░░░░░██████░░░░░░░░░░░░░░██
//                          ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░██
//                        ██░░░░░░░░░░░░░░░░██████░░░░░░░░░░░░░░░░██
//                        ██░░░░░░░░░░░░░░░░██████░░░░░░░░░░░░░░░░██
//                      ██░░░░░░░░░░░░░░░░░░██████░░░░░░░░░░░░░░░░░░██
//        ░░            ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░██
//                        ██████████████████████████████████████████
//
// Note: This is a copy pasta for development purposes
// ideally we would make health code in program crate generic over AccountSharedData and AccountInfos

#[macro_export]
macro_rules! cm {
    ($x: expr) => {
        checked_math::checked_math!($x).unwrap_or_else(|| panic!("math error"))
    };
}

/// Compute health contribution of two tokens - pure convenience
#[inline(always)]
fn pair_health(
    health_type: HealthType,
    info1: &TokenInfo,
    balance1: I80F48,
    info2: &TokenInfo,
) -> Result<I80F48> {
    let health1 = health_contribution(health_type, info1, balance1)?;
    let health2 = health_contribution(health_type, info2, info2.balance)?;
    Ok(cm!(health1 + health2))
}

/// Compute health contribution for a given balance
/// wart: independent of the balance stored in TokenInfo
/// balance is in health-reference-token native units
#[inline(always)]
fn health_contribution(
    health_type: HealthType,
    info: &TokenInfo,
    balance: I80F48,
) -> Result<I80F48> {
    let weight = if balance.is_negative() {
        info.liab_weight(health_type)
    } else {
        info.asset_weight(health_type)
    };
    Ok(cm!(balance * weight))
}

struct TokenInfo {
    token_index: TokenIndex,
    maint_asset_weight: I80F48,
    init_asset_weight: I80F48,
    maint_liab_weight: I80F48,
    init_liab_weight: I80F48,
    oracle_price: I80F48, // native/native
    // in health-reference-token native units
    balance: I80F48,
}

impl TokenInfo {
    #[inline(always)]
    fn asset_weight(&self, health_type: HealthType) -> I80F48 {
        match health_type {
            HealthType::Init => self.init_asset_weight,
            HealthType::Maint => self.maint_asset_weight,
        }
    }

    #[inline(always)]
    fn liab_weight(&self, health_type: HealthType) -> I80F48 {
        match health_type {
            HealthType::Init => self.init_liab_weight,
            HealthType::Maint => self.maint_liab_weight,
        }
    }
}

struct PerpInfo {
    maint_asset_weight: I80F48,
    init_asset_weight: I80F48,
    maint_liab_weight: I80F48,
    init_liab_weight: I80F48,
    // in health-reference-token native units, needs scaling by asset/liab
    base: I80F48,
    // in health-reference-token native units, no asset/liab factor needed
    quote: I80F48,
}

impl PerpInfo {
    #[inline(always)]
    fn health_contribution(&self, health_type: HealthType) -> I80F48 {
        let factor = match (health_type, self.base.is_negative()) {
            (HealthType::Init, true) => self.init_liab_weight,
            (HealthType::Init, false) => self.init_asset_weight,
            (HealthType::Maint, true) => self.maint_liab_weight,
            (HealthType::Maint, false) => self.maint_asset_weight,
        };
        cm!(self.quote + factor * self.base)
    }
}

pub struct HealthCache {
    token_infos: Vec<TokenInfo>,
    perp_infos: Vec<PerpInfo>,
}

impl HealthCache {
    pub fn health(&self, health_type: HealthType) -> Result<I80F48> {
        let mut health = I80F48::ZERO;
        for token_info in self.token_infos.iter() {
            let contrib = health_contribution(health_type, token_info, token_info.balance)?;
            health = cm!(health + contrib);
        }
        for perp_info in self.perp_infos.iter() {
            let contrib = perp_info.health_contribution(health_type);
            health = cm!(health + contrib);
        }
        Ok(health)
    }

    pub fn adjust_token_balance(&mut self, token_index: TokenIndex, change: I80F48) -> Result<()> {
        let mut entry = self
            .token_infos
            .iter_mut()
            .find(|t| t.token_index == token_index)
            .ok_or_else(|| error!(MangoError::SomeError))?;
        entry.balance = cm!(entry.balance + change * entry.oracle_price);
        Ok(())
    }
}

/// This trait abstracts how to find accounts needed for the health computation.
///
/// There are different ways they are retrieved from remainingAccounts, based
/// on the instruction:
/// - FixedOrderAccountRetriever requires the remainingAccounts to be in a well
///   defined order and is the fastest. It's used where possible.
/// - ScanningAccountRetriever does a linear scan for each account it needs.
///   It needs more compute, but works when a union of bank/oracle/market accounts
///   are passed because health needs to be computed for different baskets in
///   one instruction (such as for liquidation instructions).
pub trait AccountRetrieverForAccountSharedData<'a> {
    fn bank_and_oracle(
        &self,
        group: &Pubkey,
        account_index: usize,
        token_index: TokenIndex,
    ) -> Result<(&'a Bank, &'a AccountSharedData)>;

    fn serum_oo(&self, account_index: usize, key: &Pubkey) -> Result<&'a OpenOrders>;

    fn perp_market(
        &self,
        group: &Pubkey,
        account_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<&'a PerpMarket>;
}

/// Assumes the account infos needed for the health computation follow a strict order.
///
/// 1. n_banks Bank account, in the order of account.tokens.iter_active()
/// 2. n_banks oracle accounts, one for each bank in the same order
/// 3. PerpMarket accounts, in the order of account.perps.iter_active_accounts()
/// 4. serum3 OpenOrders accounts, in the order of account.serum3.iter_active()
pub struct FixedOrderAccountRetrieverForAccountSharedData<'a> {
    pub ais: &'a [(Pubkey, &'a AccountSharedData)],
    pub n_banks: usize,
    pub begin_perp: usize,
    pub begin_serum3: usize,
}

// fn strip_dex_padding<'a>(acc: &'a AccountSharedData) -> Result<'a, [u8]> {
//     // require!(acc.data_len() >= 12, MangoError::SomeError);
//     Ok(Ref::map(acc.try_borrow_data()?, |data| {
//         &data[5..data.len() - 7]
//     }))
// }

pub fn load_open_orders<'a>(acc: &AccountSharedData) -> Result<&serum_dex::state::OpenOrders> {
    Ok(bytemuck::from_bytes(&acc.data()[5..acc.data().len() - 7]))
}

impl<'a> AccountRetrieverForAccountSharedData<'a>
    for FixedOrderAccountRetrieverForAccountSharedData<'a>
{
    fn bank_and_oracle(
        &self,
        _group: &Pubkey,
        account_index: usize,
        _token_index: TokenIndex,
    ) -> Result<(&'a Bank, &'a AccountSharedData)> {
        let bank = load_mango_account::<Bank>(&self.ais[account_index].1).unwrap();
        // require!(&bank.group == group, MangoError::SomeError);
        // require!(bank.token_index == token_index, MangoError::SomeError);
        let oracle = &self.ais[cm!(self.n_banks + account_index)].1;
        // require!(&bank.oracle == oracle.key, MangoError::SomeError);
        Ok((bank, oracle))
    }

    fn perp_market(
        &self,
        _group: &Pubkey,
        account_index: usize,
        _perp_market_index: PerpMarketIndex,
    ) -> Result<&'a PerpMarket> {
        let ai = &self.ais[cm!(self.begin_perp + account_index)].1;
        let market = load_mango_account::<PerpMarket>(ai).unwrap();
        // require!(&market.group == group, MangoError::SomeError);
        // require!(
        //     market.perp_market_index == perp_market_index,
        //     MangoError::SomeError
        // );
        Ok(market)
    }

    fn serum_oo(&self, account_index: usize, _key: &Pubkey) -> Result<&'a OpenOrders> {
        let ai = &self.ais[cm!(self.begin_serum3 + account_index)].1;
        // require!(key == ai.key, MangoError::SomeError);
        load_open_orders(ai)
    }
}

/// The HealthInfo returned from this function is specialized for the health_type
/// unless called with allow_serum3=false.
///
/// The reason is that the health type used can affect the way funds reserved for
/// orders get distributed to the token balances.
pub fn compute_health_detail<'a>(
    account: &MangoAccount,
    retriever: &impl AccountRetrieverForAccountSharedData<'a>,
    health_type: HealthType,
    allow_serum3: bool,
) -> Result<HealthCache> {
    // token contribution from token accounts
    let mut token_infos = vec![];
    for (i, position) in account.tokens.iter_active().enumerate() {
        let (bank, oracle_ai) =
            retriever.bank_and_oracle(&account.group, i, position.token_index)?;

        let oracle_type = determine_oracle_type(&oracle_ai.data()).unwrap();
        let oracle_price = match oracle_type {
            OracleType::Stub => load_mango_account::<StubOracle>(&oracle_ai).unwrap().price,
            OracleType::Pyth => I80F48::from_num(
                pyth_sdk_solana::load_price(&oracle_ai.data())
                    .unwrap()
                    .price,
            ),
        };

        // converts the token value to the basis token value for health computations
        // TODO: health basis token == USDC?
        let native = position.native(&bank);

        token_infos.push(TokenInfo {
            token_index: bank.token_index,
            maint_asset_weight: bank.maint_asset_weight,
            init_asset_weight: bank.init_asset_weight,
            maint_liab_weight: bank.maint_liab_weight,
            init_liab_weight: bank.init_liab_weight,
            oracle_price,
            balance: cm!(native * oracle_price),
        });
    }

    // token contribution from serum accounts
    for (i, serum_account) in account.serum3.iter_active().enumerate() {
        let oo = retriever.serum_oo(i, &serum_account.open_orders)?;
        if !allow_serum3 {
            // require!(
            //     oo.native_coin_total == 0
            //         && oo.native_pc_total == 0
            //         && oo.referrer_rebates_accrued == 0,
            //     MangoError::SomeError
            // );
            continue;
        }

        // find the TokenInfos for the market's base and quote tokens
        let base_index = token_infos
            .iter()
            .position(|ti| ti.token_index == serum_account.base_token_index)
            .ok_or_else(|| error!(MangoError::SomeError))?;
        let quote_index = token_infos
            .iter()
            .position(|ti| ti.token_index == serum_account.quote_token_index)
            .ok_or_else(|| error!(MangoError::SomeError))?;
        let (base_info, quote_info) = if base_index < quote_index {
            let (l, r) = token_infos.split_at_mut(quote_index);
            (&mut l[base_index], &mut r[0])
        } else {
            let (l, r) = token_infos.split_at_mut(base_index);
            (&mut r[0], &mut l[quote_index])
        };

        // add the amounts that are freely settleable
        let base_free = I80F48::from_num(oo.native_coin_free);
        let quote_free = I80F48::from_num(cm!(oo.native_pc_free + oo.referrer_rebates_accrued));
        base_info.balance = cm!(base_info.balance + base_free * base_info.oracle_price);
        quote_info.balance = cm!(quote_info.balance + quote_free * quote_info.oracle_price);

        // for the amounts that are reserved for orders, compute the worst case for health
        // by checking if everything-is-base or everything-is-quote produces worse
        // outcomes
        let reserved_base = I80F48::from_num(cm!(oo.native_coin_total - oo.native_coin_free));
        let reserved_quote = I80F48::from_num(cm!(oo.native_pc_total - oo.native_pc_free));
        let reserved_balance =
            cm!(reserved_base * base_info.oracle_price + reserved_quote * quote_info.oracle_price);
        let all_in_base = cm!(base_info.balance + reserved_balance);
        let all_in_quote = cm!(quote_info.balance + reserved_balance);
        if pair_health(health_type, base_info, all_in_base, quote_info)?
            < pair_health(health_type, quote_info, all_in_quote, base_info)?
        {
            base_info.balance = all_in_base;
        } else {
            quote_info.balance = all_in_quote;
        }
    }

    // health contribution from perp accounts
    let mut perp_infos = Vec::with_capacity(account.perps.iter_active_accounts().count());
    for (i, perp_account) in account.perps.iter_active_accounts().enumerate() {
        let perp_market = retriever.perp_market(&account.group, i, perp_account.market_index)?;

        // find the TokenInfos for the market's base and quote tokens
        let base_index = token_infos
            .iter()
            .position(|ti| ti.token_index == perp_market.base_token_index)
            .ok_or_else(|| error!(MangoError::SomeError))?;
        let base_info = &token_infos[base_index];

        let base_lot_size = I80F48::from(perp_market.base_lot_size);

        let base_lots = cm!(perp_account.base_position_lots + perp_account.taker_base_lots);
        let taker_quote = I80F48::from(cm!(
            perp_account.taker_quote_lots * perp_market.quote_lot_size
        ));
        let quote_current = cm!(perp_account.quote_position_native + taker_quote);

        // Two scenarios:
        // 1. The price goes low and all bids execute, converting to base.
        //    That means the perp position is increased by `bids` and the quote position
        //    is decreased by `bids * base_lot_size * price`.
        //    The health for this case is:
        //        (weighted(base_lots + bids) - bids) * base_lot_size * price + quote
        // 2. The price goes high and all asks execute, converting to quote.
        //    The health for this case is:
        //        (weighted(base_lots - asks) + asks) * base_lot_size * price + quote
        //
        // Comparing these makes it clear we need to pick the worse subfactor
        //    weighted(base_lots + bids) - bids =: scenario1
        // or
        //    weighted(base_lots - asks) + asks =: scenario2
        //
        // Additionally, we want this scenario choice to be the same no matter whether we're
        // computing init or maint health. This can be guaranteed by requiring the weights
        // to satisfy the property (P):
        //
        //     (1 - init_asset_weight) / (init_liab_weight - 1)
        //  == (1 - maint_asset_weight) / (maint_liab_weight - 1)
        //
        // Derivation:
        //   Set asks_net_lots := base_lots - asks, bids_net_lots := base_lots + bids.
        //   Now
        //     scenario1 = weighted(bids_net_lots) - bids_net_lots + base_lots and
        //     scenario2 = weighted(asks_net_lots) - asks_net_lots + base_lots
        //   So with expanding weigthed(a) = weight_factor_for_a * a, the question
        //     scenario1 < scenario2
        //   becomes:
        //     (weight_factor_for_bids_net_lots - 1) * bids_net_lots
        //       < (weight_factor_for_asks_net_lots - 1) * asks_net_lots
        //   Since asks_net_lots < 0 and bids_net_lots > 0 is the only interesting case, (P) follows.
        //
        // We satisfy (P) by requiring
        //   asset_weight = 1 - x and liab_weight = 1 + x
        //
        // And with that assumption the scenario choice condition further simplifies to:
        //            scenario1 < scenario2
        //   iff  abs(bids_net_lots) > abs(asks_net_lots)
        let bids_net_lots = cm!(base_lots + perp_account.bids_base_lots);
        let asks_net_lots = cm!(base_lots - perp_account.asks_base_lots);

        let lots_to_quote = base_lot_size * base_info.oracle_price;
        let base;
        let quote;
        if cm!(bids_net_lots.abs()) > cm!(asks_net_lots.abs()) {
            let bids_net_lots = I80F48::from(bids_net_lots);
            let bids_base_lots = I80F48::from(perp_account.bids_base_lots);
            base = cm!(bids_net_lots * lots_to_quote);
            quote = cm!(quote_current - bids_base_lots * lots_to_quote);
        } else {
            let asks_net_lots = I80F48::from(asks_net_lots);
            let asks_base_lots = I80F48::from(perp_account.asks_base_lots);
            base = cm!(asks_net_lots * lots_to_quote);
            quote = cm!(quote_current + asks_base_lots * lots_to_quote);
        };

        perp_infos.push(PerpInfo {
            init_asset_weight: perp_market.init_asset_weight,
            init_liab_weight: perp_market.init_liab_weight,
            maint_asset_weight: perp_market.maint_asset_weight,
            maint_liab_weight: perp_market.maint_liab_weight,
            base,
            quote,
        });
    }

    Ok(HealthCache {
        token_infos,
        perp_infos,
    })
}
