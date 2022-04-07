use anchor_lang::prelude::*;
use fixed::types::I80F48;
use serum_dex::state::OpenOrders;
use std::cell::{Ref, RefMut};
use std::cmp::min;
use std::collections::HashMap;

use crate::error::MangoError;
use crate::serum3_cpi;
use crate::state::{oracle_price, Bank, MangoAccount, PerpMarket, PerpMarketIndex, TokenIndex};
use crate::util::checked_math as cm;
use crate::util::LoadZeroCopy;

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
pub trait AccountRetriever<'a, 'b> {
    fn bank_and_oracle(
        &self,
        group: &Pubkey,
        account_index: usize,
        token_index: TokenIndex,
    ) -> Result<(Ref<'a, Bank>, &'a AccountInfo<'b>)>;

    fn serum_oo(&self, account_index: usize, key: &Pubkey) -> Result<Ref<'a, OpenOrders>>;

    fn perp_market(
        &self,
        group: &Pubkey,
        account_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<Ref<'a, PerpMarket>>;
}

/// Assumes the account infos needed for the health computation follow a strict order.
///
/// 1. n_banks Bank account, in the order of account.tokens.iter_active()
/// 2. n_banks oracle accounts, one for each bank in the same order
/// 3. PerpMarket accounts, in the order of account.perps.iter_active_accounts()
/// 4. serum3 OpenOrders accounts, in the order of account.serum3.iter_active()
pub struct FixedOrderAccountRetriever<'a, 'b> {
    ais: &'a [AccountInfo<'b>],
    n_banks: usize,
    begin_perp: usize,
    begin_serum3: usize,
}

impl<'a, 'b> AccountRetriever<'a, 'b> for FixedOrderAccountRetriever<'a, 'b> {
    fn bank_and_oracle(
        &self,
        group: &Pubkey,
        account_index: usize,
        token_index: TokenIndex,
    ) -> Result<(Ref<'a, Bank>, &'a AccountInfo<'b>)> {
        let bank = self.ais[account_index].load::<Bank>()?;
        require!(&bank.group == group, MangoError::SomeError);
        require!(bank.token_index == token_index, MangoError::SomeError);
        let oracle = &self.ais[cm!(self.n_banks + account_index)];
        require!(&bank.oracle == oracle.key, MangoError::SomeError);
        Ok((bank, oracle))
    }

    fn perp_market(
        &self,
        group: &Pubkey,
        account_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<Ref<'a, PerpMarket>> {
        let ai = &self.ais[cm!(self.begin_perp + account_index)];
        let market = ai.load::<PerpMarket>()?;
        require!(&market.group == group, MangoError::SomeError);
        require!(
            market.perp_market_index == perp_market_index,
            MangoError::SomeError
        );
        Ok(market)
    }

    fn serum_oo(&self, account_index: usize, key: &Pubkey) -> Result<Ref<'a, OpenOrders>> {
        let ai = &self.ais[cm!(self.begin_serum3 + account_index)];
        require!(key == ai.key, MangoError::SomeError);
        serum3_cpi::load_open_orders(ai)
    }
}

/// Takes a list of account infos containing
/// - an unknown number of Banks in any order, followed by
/// - the same number of oracles in the same order as the banks, followed by
/// - an unknown number of PerpMarket accounts
/// - an unknown number of serum3 OpenOrders accounts
/// and retrieves accounts needed for the health computation by doing a linear
/// scan for each request.
pub struct ScanningAccountRetriever<'a, 'b> {
    ais: &'a [AccountInfo<'b>],
    token_index_map: HashMap<TokenIndex, usize>,
    perp_index_map: HashMap<PerpMarketIndex, usize>,
}

impl<'a, 'b> ScanningAccountRetriever<'a, 'b> {
    pub fn new(ais: &'a [AccountInfo<'b>], group: &Pubkey) -> Result<Self> {
        let mut token_index_map = HashMap::with_capacity(ais.len() / 2);
        for (i, ai) in ais.iter().enumerate() {
            match ai.load::<Bank>() {
                Ok(bank) => {
                    require!(&bank.group == group, MangoError::SomeError);
                    token_index_map.insert(bank.token_index, i);
                }
                Err(Error::AnchorError(error))
                    if error.error_code_number
                        == ErrorCode::AccountDiscriminatorMismatch as u32 =>
                {
                    break;
                }
                Err(error) => return Err(error),
            };
        }

        // skip all banks and oracles
        let skip = token_index_map.len() * 2;
        let mut perp_index_map = HashMap::with_capacity(ais.len() - skip);
        for (i, ai) in ais[skip..].iter().enumerate() {
            match ai.load::<PerpMarket>() {
                Ok(perp_market) => {
                    require!(&perp_market.group == group, MangoError::SomeError);
                    perp_index_map.insert(perp_market.perp_market_index, cm!(skip + i));
                }
                Err(Error::AnchorError(error))
                    if error.error_code_number
                        == ErrorCode::AccountDiscriminatorMismatch as u32 =>
                {
                    break;
                }
                Err(error) => return Err(error),
            };
        }

        Ok(Self {
            ais,
            token_index_map,
            perp_index_map,
        })
    }

    fn n_banks(&self) -> usize {
        self.token_index_map.len()
    }

    fn begin_serum3(&self) -> usize {
        2 * self.token_index_map.len() + self.perp_index_map.len()
    }

    #[inline]
    fn bank_index(&self, token_index: TokenIndex) -> Result<usize> {
        Ok(*self
            .token_index_map
            .get(&token_index)
            .ok_or_else(|| error!(MangoError::SomeError))?)
    }

    #[inline]
    fn perp_market_index(&self, perp_market_index: PerpMarketIndex) -> Result<usize> {
        Ok(*self
            .perp_index_map
            .get(&perp_market_index)
            .ok_or_else(|| error!(MangoError::SomeError))?)
    }

    pub fn bank_mut_and_oracle(
        &self,
        token_index: TokenIndex,
    ) -> Result<(RefMut<'a, Bank>, &'a AccountInfo<'b>)> {
        let index = self.bank_index(token_index)?;
        let bank = self.ais[index].load_mut_fully_unchecked::<Bank>()?;
        let oracle = &self.ais[cm!(self.n_banks() + index)];
        require!(&bank.oracle == oracle.key, MangoError::SomeError);
        Ok((bank, oracle))
    }
}

impl<'a, 'b> AccountRetriever<'a, 'b> for ScanningAccountRetriever<'a, 'b> {
    fn bank_and_oracle(
        &self,
        _group: &Pubkey,
        _account_index: usize,
        token_index: TokenIndex,
    ) -> Result<(Ref<'a, Bank>, &'a AccountInfo<'b>)> {
        let index = self.bank_index(token_index)?;
        let bank = self.ais[index].load_fully_unchecked::<Bank>()?;
        let oracle = &self.ais[cm!(self.n_banks() + index)];
        require!(&bank.oracle == oracle.key, MangoError::SomeError);
        Ok((bank, oracle))
    }

    fn perp_market(
        &self,
        _group: &Pubkey,
        _account_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<Ref<'a, PerpMarket>> {
        let index = self.perp_market_index(perp_market_index)?;
        self.ais[index].load_fully_unchecked::<PerpMarket>()
    }

    fn serum_oo(&self, _account_index: usize, key: &Pubkey) -> Result<Ref<'a, OpenOrders>> {
        let oo = self.ais[self.begin_serum3()..]
            .iter()
            .find(|ai| ai.key == key)
            .unwrap();
        serum3_cpi::load_open_orders(oo)
    }
}

/// There are two types of health, initial health used for opening new positions and maintenance
/// health used for liquidations. They are both calculated as a weighted sum of the assets
/// minus the liabilities but the maint. health uses slightly larger weights for assets and
/// slightly smaller weights for the liabilities. Zero is used as the bright line for both
/// i.e. if your init health falls below zero, you cannot open new positions and if your maint. health
/// falls below zero you will be liquidated.
#[derive(PartialEq, Copy, Clone)]
pub enum HealthType {
    Init,
    Maint,
}

/// Computes health for a mango account given a set of account infos
///
/// These account infos must fit the fixed layout defined by FixedOrderAccountRetriever.
pub fn compute_health_from_fixed_accounts(
    account: &MangoAccount,
    health_type: HealthType,
    ais: &[AccountInfo],
) -> Result<I80F48> {
    let active_token_len = account.tokens.iter_active().count();
    let active_serum3_len = account.serum3.iter_active().count();
    let active_perp_len = account.perps.iter_active_accounts().count();
    let expected_ais = cm!(active_token_len * 2 // banks + oracles
        + active_serum3_len // open_orders
        + active_perp_len); // PerpMarkets
    require!(ais.len() == expected_ais, MangoError::SomeError);

    let retriever = FixedOrderAccountRetriever {
        ais,
        n_banks: active_token_len,
        begin_serum3: cm!(active_token_len * 2),
        begin_perp: cm!(active_token_len * 2 + active_serum3_len),
    };
    compute_health_detail(account, &retriever, health_type, true)?.health(health_type)
}

/// Compute health with an arbitrary AccountRetriever
pub fn compute_health<'a, 'b: 'a>(
    account: &MangoAccount,
    health_type: HealthType,
    retriever: &impl AccountRetriever<'a, 'b>,
) -> Result<I80F48> {
    compute_health_detail(account, retriever, health_type, true)?.health(health_type)
}

/// Compute health for a liqee.
///
/// This has the advantage of returning a HealthCache, allowing for health
/// to be recomputed after token balance changes due to liquidation.
///
/// However, this only works if the serum3 open orders accounts have been
/// fully settled (like via serum3_liq_force_cancel_orders).
pub fn health_cache_for_liqee<'a, 'b: 'a>(
    account: &MangoAccount,
    retriever: &impl AccountRetriever<'a, 'b>,
) -> Result<HealthCache> {
    compute_health_detail(account, retriever, HealthType::Init, false)
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

pub struct HealthCache {
    token_infos: Vec<TokenInfo>,
}

impl HealthCache {
    pub fn health(&self, health_type: HealthType) -> Result<I80F48> {
        let mut health = I80F48::ZERO;
        for token_info in self.token_infos.iter() {
            let contrib = health_contribution(health_type, token_info, token_info.balance)?;
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

/// Weigh a perp base balance (in lots) with the appropriate health weight
#[inline(always)]
fn health_weighted_perp_base_lots(
    health_type: HealthType,
    market: &PerpMarket,
    lots: i64,
) -> Result<I80F48> {
    let weight = if lots.is_negative() {
        match health_type {
            HealthType::Init => market.init_liab_weight,
            HealthType::Maint => market.maint_liab_weight,
        }
    } else {
        match health_type {
            HealthType::Init => market.init_asset_weight,
            HealthType::Maint => market.maint_asset_weight,
        }
    };
    let lots = I80F48::from(lots);
    Ok(cm!(weight * lots))
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

/// The HealthInfo returned from this function is specialized for the health_type
/// unless called with allow_serum3=false.
///
/// The reason is that the health type used can affect the way funds reserved for
/// orders get distributed to the token balances.
fn compute_health_detail<'a, 'b: 'a>(
    account: &MangoAccount,
    retriever: &impl AccountRetriever<'a, 'b>,
    health_type: HealthType,
    allow_serum3: bool,
) -> Result<HealthCache> {
    // token contribution from token accounts
    let mut token_infos = vec![];
    for (i, position) in account.tokens.iter_active().enumerate() {
        let (bank, oracle_ai) =
            retriever.bank_and_oracle(&account.group, i, position.token_index)?;
        let oracle_price = oracle_price(oracle_ai)?;

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
            require!(
                oo.native_coin_total == 0
                    && oo.native_pc_total == 0
                    && oo.referrer_rebates_accrued == 0,
                MangoError::SomeError
            );
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
    for (i, perp_account) in account.perps.iter_active_accounts().enumerate() {
        let perp_market = retriever.perp_market(&account.group, i, perp_account.market_index)?;

        // find the TokenInfos for the market's base and quote tokens
        let base_index = token_infos
            .iter()
            .position(|ti| ti.token_index == perp_market.base_token_index)
            .ok_or_else(|| error!(MangoError::SomeError))?;
        let base_info = &token_infos[base_index];

        let base_lots = cm!(perp_account.base_position_lots + perp_account.taker_base_lots);
        let taker_quote = I80F48::from(cm!(
            perp_account.taker_quote_lots * perp_market.quote_lot_size
        ));
        let quote = cm!(perp_account.quote_position_native + taker_quote);

        // Two scenarios:
        // 1. The price goes low and all bids execute, converting to base.
        //    The health for this case is:
        //        (weighted(base_lots + bids) - bids) * base_lots * price + quote
        // 2. The price goes high and all asks execute, converting to quote.
        //    The health for this case is:
        //        (weighted(base_lots - asks) + asks) * base_lots * price + quote
        //
        // Comparing these makes it clear we need to pick the worse subfactor
        //    weighted(base_lots + bids) - bids
        // or
        //    weighted(base_lots - asks) + asks
        let weighted_base_lots_bids = health_weighted_perp_base_lots(
            health_type,
            &perp_market,
            cm!(base_lots + perp_account.bids_base_lots),
        )?;
        let bids_base_lots = I80F48::from(perp_account.bids_base_lots);
        let scenario1 = cm!(weighted_base_lots_bids - bids_base_lots);
        let weighted_base_lots_asks = health_weighted_perp_base_lots(
            health_type,
            &perp_market,
            cm!(base_lots - perp_account.asks_base_lots),
        )?;
        let asks_base_lots = I80F48::from(perp_account.asks_base_lots);
        let scenario2 = cm!(weighted_base_lots_asks + asks_base_lots);
        let worse_scenario = min(scenario1, scenario2);
        let base_lot_size = I80F48::from(perp_market.base_lot_size);
        let _health = cm!(worse_scenario * base_lot_size * base_info.oracle_price + quote);

        // The above choice between scenario1 and 2 depends on the asset_weight and
        // liab weight. Thus it needs to be redone for init and maint health.
        //
        // The condition for the choice to be the same is:
        //     (1 - init_asset_weight) / (init_liab_weight - 1)
        //  == (1 - maint_asset_weight) / (maint_liab_weight - 1)
        //
        // Which can be derived by noticing that health for both scenarios is
        //    weighted(x) + y - x
        // and that the only interesting case is
        //     asks_net = base_lots - asks < 0 and
        //     bids_net = base_lots + bids > 0.
        // Then
        //       health_bids_scenario < health_asks_scenario
        //   iff (asset_weight - 1) * bids_net < (liab_weight - 1) * asks_net
        //   iff (1 - asset_weightt) / (liab_weight - 1) bids_net > abs(asks_net)

        // Probably the resolution here is to go to v3's assumption that there's an x
        // such that asset_weight = 1-x and liab_weight = 1+x.
        // This is ok as long as perp markets are strictly isolated.
    }

    Ok(HealthCache { token_infos })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use fixed::types::I80F48;

    #[test]
    fn test_precision() {
        // I80F48 can only represent until 1/2^48
        assert_ne!(
            I80F48::from_num(1_u128) / I80F48::from_num(2_u128.pow(48)),
            0
        );
        assert_eq!(
            I80F48::from_num(1_u128) / I80F48::from_num(2_u128.pow(49)),
            0
        );

        // I80F48 can only represent until 14 decimal points
        assert_ne!(
            I80F48::from_str(format!("0.{}1", "0".repeat(13)).as_str()).unwrap(),
            0
        );
        assert_eq!(
            I80F48::from_str(format!("0.{}1", "0".repeat(14)).as_str()).unwrap(),
            0
        );
    }
}
