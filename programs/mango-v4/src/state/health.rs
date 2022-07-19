use anchor_lang::prelude::*;
use anchor_lang::ZeroCopy;

use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use serum_dex::state::OpenOrders;

use std::cell::Ref;
use std::collections::HashMap;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::serum3_cpi;
use crate::state::{oracle_price, Bank, MangoAccount, PerpMarket, PerpMarketIndex, TokenIndex};
use crate::util::checked_math as cm;

const BANKRUPTCY_DUST_THRESHOLD: I80F48 = I80F48!(0.000001);

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
pub trait AccountRetriever {
    fn bank_and_oracle(
        &self,
        group: &Pubkey,
        account_index: usize,
        token_index: TokenIndex,
    ) -> Result<(&Bank, I80F48)>;

    fn serum_oo(&self, account_index: usize, key: &Pubkey) -> Result<&OpenOrders>;

    fn perp_market(
        &self,
        group: &Pubkey,
        account_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<&PerpMarket>;
}

/// Assumes the account infos needed for the health computation follow a strict order.
///
/// 1. n_banks Bank account, in the order of account.tokens.iter_active()
/// 2. n_banks oracle accounts, one for each bank in the same order
/// 3. PerpMarket accounts, in the order of account.perps.iter_active_accounts()
/// 4. serum3 OpenOrders accounts, in the order of account.serum3.iter_active()
pub struct FixedOrderAccountRetriever<T: KeyedAccountReader> {
    pub ais: Vec<T>,
    pub n_banks: usize,
    pub begin_perp: usize,
    pub begin_serum3: usize,
}

pub fn new_fixed_order_account_retriever<'a, 'info>(
    ais: &'a [AccountInfo<'info>],
    account: &MangoAccount,
) -> Result<FixedOrderAccountRetriever<AccountInfoRef<'a, 'info>>> {
    let active_token_len = account.tokens.iter_active().count();
    let active_serum3_len = account.serum3.iter_active().count();
    let active_perp_len = account.perps.iter_active_accounts().count();
    let expected_ais = cm!(active_token_len * 2 // banks + oracles
        + active_perp_len // PerpMarkets
        + active_serum3_len); // open_orders
    require_eq!(ais.len(), expected_ais);

    Ok(FixedOrderAccountRetriever {
        ais: ais
            .into_iter()
            .map(|ai| AccountInfoRef::borrow(ai))
            .collect::<Result<Vec<_>>>()?,
        n_banks: active_token_len,
        begin_perp: cm!(active_token_len * 2),
        begin_serum3: cm!(active_token_len * 2 + active_perp_len),
    })
}

impl<T: KeyedAccountReader> FixedOrderAccountRetriever<T> {
    fn bank(&self, group: &Pubkey, account_index: usize, token_index: TokenIndex) -> Result<&Bank> {
        let bank = self.ais[account_index].load::<Bank>()?;
        require_keys_eq!(bank.group, *group);
        require_eq!(bank.token_index, token_index);
        Ok(bank)
    }

    fn oracle_price(&self, account_index: usize, bank: &Bank) -> Result<I80F48> {
        let oracle = &self.ais[cm!(self.n_banks + account_index)];
        require_keys_eq!(bank.oracle, *oracle.key());
        Ok(oracle_price(
            oracle,
            bank.oracle_config.conf_filter,
            bank.mint_decimals,
        )?)
    }
}

impl<T: KeyedAccountReader> AccountRetriever for FixedOrderAccountRetriever<T> {
    fn bank_and_oracle(
        &self,
        group: &Pubkey,
        account_index: usize,
        token_index: TokenIndex,
    ) -> Result<(&Bank, I80F48)> {
        let bank = self
            .bank(group, account_index, token_index)
            .with_context(|| {
                format!(
                    "loading bank with health account index {}, token index {}, passed account {}",
                    account_index,
                    token_index,
                    self.ais[account_index].key(),
                )
            })?;

        let oracle_price = self.oracle_price(account_index, bank).with_context(|| {
            format!(
                "getting oracle for bank with health account index {} and token index {}, passed account {}",
                account_index,
                token_index,
                self.ais[self.n_banks + account_index].key(),
            )
        })?;

        Ok((bank, oracle_price))
    }

    fn perp_market(
        &self,
        group: &Pubkey,
        account_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<&PerpMarket> {
        let ai = &self.ais[cm!(self.begin_perp + account_index)];
        (|| {
            let market = ai.load::<PerpMarket>()?;
            require_keys_eq!(market.group, *group);
            require_eq!(market.perp_market_index, perp_market_index);
            Ok(market)
        })()
        .with_context(|| {
            format!(
                "loading perp market with health account index {} and perp market index {}, passed account {}",
                account_index,
                perp_market_index,
                ai.key(),
            )
        })
    }

    fn serum_oo(&self, account_index: usize, key: &Pubkey) -> Result<&OpenOrders> {
        let ai = &self.ais[cm!(self.begin_serum3 + account_index)];
        (|| {
            require_keys_eq!(*key, *ai.key());
            serum3_cpi::load_open_orders(ai)
        })()
        .with_context(|| {
            format!(
                "loading serum open orders with health account index {}, passed account {}",
                account_index,
                ai.key(),
            )
        })
    }
}

/// Takes a list of account infos containing
/// - an unknown number of Banks in any order, followed by
/// - the same number of oracles in the same order as the banks, followed by
/// - an unknown number of PerpMarket accounts
/// - an unknown number of serum3 OpenOrders accounts
/// and retrieves accounts needed for the health computation by doing a linear
/// scan for each request.
pub struct ScanningAccountRetriever<'a, 'info> {
    ais: Vec<AccountInfoRefMut<'a, 'info>>,
    token_index_map: HashMap<TokenIndex, usize>,
    perp_index_map: HashMap<PerpMarketIndex, usize>,
}

// Returns None if `ai` doesn't have the owner or discriminator for T
fn can_load_as<'a, T: ZeroCopy + Owner>(
    (i, ai): (usize, &'a AccountInfo),
) -> Option<(usize, Result<Ref<'a, T>>)> {
    let load_result = ai.load::<T>();
    match load_result {
        Err(Error::AnchorError(error))
            if error.error_code_number == ErrorCode::AccountDiscriminatorMismatch as u32
                || error.error_code_number == ErrorCode::AccountOwnedByWrongProgram as u32 =>
        {
            return None;
        }
        _ => {}
    };
    Some((i, load_result))
}

impl<'a, 'info> ScanningAccountRetriever<'a, 'info> {
    pub fn new(ais: &'a [AccountInfo<'info>], group: &Pubkey) -> Result<Self> {
        // find all Bank accounts
        let mut token_index_map = HashMap::with_capacity(ais.len() / 2);
        ais.iter()
            .enumerate()
            .map_while(can_load_as::<Bank>)
            .try_for_each(|(i, loaded)| {
                (|| {
                    let bank = loaded?;
                    require_keys_eq!(bank.group, *group);
                    token_index_map.insert(bank.token_index, i);
                    Ok(())
                })()
                .with_context(|| format!("scanning banks, health account index {}", i))
            })?;

        // skip all banks and oracles, then find number of PerpMarket accounts
        let skip = token_index_map.len() * 2;
        let mut perp_index_map = HashMap::with_capacity(ais.len() - skip);
        ais[skip..]
            .iter()
            .enumerate()
            .map_while(can_load_as::<PerpMarket>)
            .try_for_each(|(i, loaded)| {
                (|| {
                    let perp_market = loaded?;
                    require_keys_eq!(perp_market.group, *group);
                    perp_index_map.insert(perp_market.perp_market_index, cm!(skip + i));
                    Ok(())
                })()
                .with_context(|| {
                    format!("scanning perp markets, health account index {}", i + skip)
                })
            })?;

        Ok(Self {
            ais: ais
                .into_iter()
                .map(|ai| AccountInfoRefMut::borrow(ai))
                .collect::<Result<Vec<_>>>()?,
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
            .ok_or_else(|| error_msg!("token index {} not found", token_index))?)
    }

    #[inline]
    fn perp_market_index(&self, perp_market_index: PerpMarketIndex) -> Result<usize> {
        Ok(*self
            .perp_index_map
            .get(&perp_market_index)
            .ok_or_else(|| error_msg!("perp market index {} not found", perp_market_index))?)
    }

    pub fn banks_mut_and_oracles(
        &mut self,
        token_index1: TokenIndex,
        token_index2: TokenIndex,
    ) -> Result<(&mut Bank, I80F48, Option<(&mut Bank, I80F48)>)> {
        let n_banks = self.n_banks();
        if token_index1 == token_index2 {
            let index = self.bank_index(token_index1)?;
            let (bank_part, oracle_part) = self.ais.split_at_mut(index + 1);
            let bank = bank_part[index].load_mut_fully_unchecked::<Bank>()?;
            let oracle = &oracle_part[n_banks - 1];
            require_keys_eq!(bank.oracle, *oracle.key);
            let price = oracle_price(oracle, bank.oracle_config.conf_filter, bank.mint_decimals)?;
            return Ok((bank, price, None));
        }
        let index1 = self.bank_index(token_index1)?;
        let index2 = self.bank_index(token_index2)?;
        let (first, second, swap) = if index1 < index2 {
            (index1, index2, false)
        } else {
            (index2, index1, true)
        };

        // split_at_mut after the first bank and after the second bank
        let (first_bank_part, second_part) = self.ais.split_at_mut(first + 1);
        let (second_bank_part, oracles_part) = second_part.split_at_mut(second - first);

        let bank1 = first_bank_part[first].load_mut_fully_unchecked::<Bank>()?;
        let bank2 = second_bank_part[second - (first + 1)].load_mut_fully_unchecked::<Bank>()?;
        let oracle1 = &oracles_part[cm!(n_banks + first - (second + 1))];
        let oracle2 = &oracles_part[cm!(n_banks + second - (second + 1))];
        require_keys_eq!(bank1.oracle, *oracle1.key);
        require_keys_eq!(bank2.oracle, *oracle2.key);
        let mint_decimals1 = bank1.mint_decimals;
        let mint_decimals2 = bank2.mint_decimals;
        let price1 = oracle_price(oracle1, bank1.oracle_config.conf_filter, mint_decimals1)?;
        let price2 = oracle_price(oracle2, bank2.oracle_config.conf_filter, mint_decimals2)?;
        if swap {
            Ok((bank2, price2, Some((bank1, price1))))
        } else {
            Ok((bank1, price1, Some((bank2, price2))))
        }
    }

    pub fn scanned_bank_and_oracle(&self, token_index: TokenIndex) -> Result<(&Bank, I80F48)> {
        let index = self.bank_index(token_index)?;
        let bank = self.ais[index].load_fully_unchecked::<Bank>()?;
        let oracle = &self.ais[cm!(self.n_banks() + index)];
        require_keys_eq!(bank.oracle, *oracle.key);
        Ok((
            bank,
            oracle_price(oracle, bank.oracle_config.conf_filter, bank.mint_decimals)?,
        ))
    }

    pub fn scanned_perp_market(&self, perp_market_index: PerpMarketIndex) -> Result<&PerpMarket> {
        let index = self.perp_market_index(perp_market_index)?;
        self.ais[index].load_fully_unchecked::<PerpMarket>()
    }

    pub fn scanned_serum_oo(&self, key: &Pubkey) -> Result<&OpenOrders> {
        let oo = self.ais[self.begin_serum3()..]
            .iter()
            .find(|ai| ai.key == key)
            .ok_or_else(|| error_msg!("no serum3 open orders for key {}", key))?;
        serum3_cpi::load_open_orders(oo)
    }
}

impl<'a, 'info> AccountRetriever for ScanningAccountRetriever<'a, 'info> {
    fn bank_and_oracle(
        &self,
        _group: &Pubkey,
        _account_index: usize,
        token_index: TokenIndex,
    ) -> Result<(&Bank, I80F48)> {
        self.scanned_bank_and_oracle(token_index)
    }

    fn perp_market(
        &self,
        _group: &Pubkey,
        _account_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<&PerpMarket> {
        self.scanned_perp_market(perp_market_index)
    }

    fn serum_oo(&self, _account_index: usize, key: &Pubkey) -> Result<&OpenOrders> {
        self.scanned_serum_oo(key)
    }
}

/// There are two types of health, initial health used for opening new positions and maintenance
/// health used for liquidations. They are both calculated as a weighted sum of the assets
/// minus the liabilities but the maint. health uses slightly larger weights for assets and
/// slightly smaller weights for the liabilities. Zero is used as the bright line for both
/// i.e. if your init health falls below zero, you cannot open new positions and if your maint. health
/// falls below zero you will be liquidated.
#[derive(PartialEq, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
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
        + active_perp_len // PerpMarkets
        + active_serum3_len); // open_orders
    require_eq!(ais.len(), expected_ais);

    let retriever = FixedOrderAccountRetriever {
        ais: ais
            .into_iter()
            .map(|ai| AccountInfoRef::borrow(ai))
            .collect::<Result<Vec<_>>>()?,
        n_banks: active_token_len,
        begin_perp: cm!(active_token_len * 2),
        begin_serum3: cm!(active_token_len * 2 + active_perp_len),
    };
    Ok(new_health_cache(account, &retriever)?.health(health_type))
}

/// Compute health with an arbitrary AccountRetriever
pub fn compute_health(
    account: &MangoAccount,
    health_type: HealthType,
    retriever: &impl AccountRetriever,
) -> Result<I80F48> {
    Ok(new_health_cache(account, retriever)?.health(health_type))
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize)]
pub struct TokenInfo {
    token_index: TokenIndex,
    maint_asset_weight: I80F48,
    init_asset_weight: I80F48,
    maint_liab_weight: I80F48,
    init_liab_weight: I80F48,
    oracle_price: I80F48, // native/native
    // in health-reference-token native units
    balance: I80F48,
    // in health-reference-token native units
    serum3_max_reserved: I80F48,
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

    #[inline(always)]
    fn health_contribution(&self, health_type: HealthType) -> I80F48 {
        let weight = if self.balance.is_negative() {
            self.liab_weight(health_type)
        } else {
            self.asset_weight(health_type)
        };
        cm!(self.balance * weight)
    }
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize)]
pub struct Serum3Info {
    reserved: I80F48,
    base_index: usize,
    quote_index: usize,
}

impl Serum3Info {
    #[inline(always)]
    fn health_contribution(&self, health_type: HealthType, token_infos: &[TokenInfo]) -> I80F48 {
        let base_info = &token_infos[self.base_index];
        let quote_info = &token_infos[self.quote_index];
        let reserved = self.reserved;

        if reserved.is_zero() {
            return I80F48::ZERO;
        }

        // How much the health would increase if the reserved balance were applied to the passed
        // token info?
        let compute_health_effect = |token_info: &TokenInfo| {
            // This balance includes all possible reserved funds from markets that relate to the
            // token, including this market itself: `reserved` is already included in `max_balance`.
            let max_balance = cm!(token_info.balance + token_info.serum3_max_reserved);

            // Assuming `reserved` was added to `max_balance` last (because that gives the smallest
            // health effects): how much did health change because of it?
            let (asset_part, liab_part) = if max_balance >= reserved {
                (reserved, I80F48::ZERO)
            } else if max_balance.is_negative() {
                (I80F48::ZERO, reserved)
            } else {
                (max_balance, cm!(reserved - max_balance))
            };

            let asset_weight = token_info.asset_weight(health_type);
            let liab_weight = token_info.liab_weight(health_type);
            cm!(asset_weight * asset_part + liab_weight * liab_part)
        };

        let reserved_as_base = compute_health_effect(base_info);
        let reserved_as_quote = compute_health_effect(quote_info);
        reserved_as_base.min(reserved_as_quote)
    }
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize)]
pub struct PerpInfo {
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
    /// Total health contribution from perp balances
    ///
    /// Due to isolation of perp markets, users may never borrow against perp
    /// positions without settling first: perp health is capped at zero.
    ///
    /// Users need to settle their perp pnl with other perp market participants
    /// in order to realize their gains if they want to use them as collateral.
    ///
    /// This is because we don't trust the perp's base price to not suddenly jump to
    /// zero (if users could borrow against their perp balances they might now
    /// be bankrupt) or suddenly increase a lot (if users could borrow against perp
    /// balances they could now borrow other assets).
    #[inline(always)]
    fn health_contribution(&self, health_type: HealthType) -> I80F48 {
        let weight = match (health_type, self.base.is_negative()) {
            (HealthType::Init, true) => self.init_liab_weight,
            (HealthType::Init, false) => self.init_asset_weight,
            (HealthType::Maint, true) => self.maint_liab_weight,
            (HealthType::Maint, false) => self.maint_asset_weight,
        };

        // FUTURE: Allow v3-style "reliable" markets where we can return
        // `self.quote + weight * self.base` here
        cm!(self.quote + weight * self.base).min(I80F48::ZERO)
    }
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize)]
pub struct HealthCache {
    token_infos: Vec<TokenInfo>,
    serum3_infos: Vec<Serum3Info>,
    perp_infos: Vec<PerpInfo>,
}

impl HealthCache {
    pub fn health(&self, health_type: HealthType) -> I80F48 {
        let mut health = I80F48::ZERO;
        let sum = |contrib| {
            health = cm!(health + contrib);
        };
        self.health_sum(health_type, sum);
        health
    }

    pub fn adjust_token_balance(&mut self, token_index: TokenIndex, change: I80F48) -> Result<()> {
        let mut entry = self
            .token_infos
            .iter_mut()
            .find(|t| t.token_index == token_index)
            .ok_or_else(|| error_msg!("token index {} not found", token_index))?;
        entry.balance = cm!(entry.balance + change * entry.oracle_price);
        Ok(())
    }

    pub fn has_liquidatable_assets(&self) -> bool {
        let spot_liquidatable = self.token_infos.iter().any(|ti| {
            ti.balance > BANKRUPTCY_DUST_THRESHOLD || ti.serum3_max_reserved.is_positive()
        });
        let perp_liquidatable = self
            .perp_infos
            .iter()
            .any(|p| p.base != 0 || p.quote > BANKRUPTCY_DUST_THRESHOLD);
        spot_liquidatable || perp_liquidatable
    }

    pub fn has_borrows(&self) -> bool {
        // AUDIT: Can we really guarantee that liquidation/bankruptcy resolution always leaves
        //        non-negative balances?
        let spot_borrows = self.token_infos.iter().any(|ti| ti.balance.is_negative());
        let perp_borrows = self
            .perp_infos
            .iter()
            .any(|p| p.quote.is_negative() || p.base != 0);
        spot_borrows || perp_borrows
    }

    fn health_sum(&self, health_type: HealthType, mut action: impl FnMut(I80F48)) {
        for token_info in self.token_infos.iter() {
            let contrib = token_info.health_contribution(health_type);
            action(contrib);
        }
        for serum3_info in self.serum3_infos.iter() {
            let contrib = serum3_info.health_contribution(health_type, &self.token_infos);
            action(contrib);
        }
        for perp_info in self.perp_infos.iter() {
            let contrib = perp_info.health_contribution(health_type);
            action(contrib);
        }
    }

    /// Sum of only the positive health components (assets) and
    /// sum of absolute values of all negative health components (liabs, always >= 0)
    pub fn health_assets_and_liabs(&self, health_type: HealthType) -> (I80F48, I80F48) {
        let mut assets = I80F48::ZERO;
        let mut liabs = I80F48::ZERO;
        let sum = |contrib| {
            if contrib > 0 {
                assets = cm!(assets + contrib);
            } else {
                liabs = cm!(liabs - contrib);
            }
        };
        self.health_sum(health_type, sum);
        (assets, liabs)
    }

    /// The health ratio is
    /// - 0 if health is 0 - meaning assets = liabs
    /// - 100 if there's 2x as many assets as liabs
    /// - 200 if there's 3x as many assets as liabs
    /// - MAX if liabs = 0
    ///
    /// Maybe talking about the collateralization ratio assets/liabs is more intuitive?
    pub fn health_ratio(&self, health_type: HealthType) -> I80F48 {
        let (assets, liabs) = self.health_assets_and_liabs(health_type);
        let hundred = I80F48::from(100);
        if liabs > 0 {
            cm!(hundred * (assets - liabs) / liabs)
        } else {
            I80F48::MAX
        }
    }

    /// How much source native tokens may be swapped for target tokens while staying
    /// above the min_ratio health ratio.
    ///
    /// TODO: Add slippage/fees.
    pub fn max_swap_source_for_health_ratio(
        &self,
        source: TokenIndex,
        target: TokenIndex,
        min_ratio: I80F48,
    ) -> Result<I80F48> {
        // The health_ratio is a nonlinear based on swap amount.
        // For large swap amounts the slope is guaranteed to be negative, but small amounts
        // can have positive slope (e.g. using source deposits to pay back target borrows).
        //
        // That means:
        // - even if the initial ratio is < min_ratio it can be useful to swap to *increase* health
        // - be careful about finding the min_ratio point: the function isn't convex

        let initial_ratio = self.health_ratio(HealthType::Init);
        if initial_ratio < 0 {
            return Ok(I80F48::ZERO);
        }

        let source_index = find_token_info_index(&self.token_infos, source)?;
        let target_index = find_token_info_index(&self.token_infos, target)?;
        let source = &self.token_infos[source_index];
        let target = &self.token_infos[target_index];

        // There are two key slope changes: Assume source.balance > 0 and target.balance < 0. Then
        // initially health ratio goes up. When one of balances flips sign, the health ratio slope
        // may be positive or negative for a bit, until both balances have flipped and the slope is
        // negative.
        // The maximum will be at one of these points (ignoring serum3 effects).
        let cache_after_swap = |amount| {
            let mut adjusted_cache = self.clone();
            adjusted_cache.token_infos[source_index].balance -= amount;
            adjusted_cache.token_infos[target_index].balance += amount;
            adjusted_cache
        };
        let health_ratio_after_swap =
            |amount| cache_after_swap(amount).health_ratio(HealthType::Init);

        let point0_amount = source.balance.min(-target.balance).max(I80F48::ZERO);
        let point1_amount = source.balance.max(-target.balance).max(I80F48::ZERO);
        let point0_ratio = health_ratio_after_swap(point0_amount);
        let (point1_ratio, point1_health) = {
            let cache = cache_after_swap(point1_amount);
            (
                cache.health_ratio(HealthType::Init),
                cache.health(HealthType::Init),
            )
        };

        let binary_approximation_search =
            |mut left,
             left_ratio: I80F48,
             mut right,
             mut right_ratio: I80F48,
             target_ratio: I80F48| {
                let max_iterations = 20;
                let target_error = I80F48::ONE;
                require_msg!(
                    (left_ratio - target_ratio).signum() * (right_ratio - target_ratio).signum()
                        != I80F48::ONE,
                    "internal error: left {} and right {} don't contain the target value {}",
                    left_ratio,
                    right_ratio,
                    target_ratio
                );
                for _ in 0..max_iterations {
                    let new = I80F48::from_num(0.5) * (left + right);
                    let new_ratio = health_ratio_after_swap(new);
                    let error = new_ratio - target_ratio;
                    if error > 0 && error < target_error {
                        return Ok(new);
                    }

                    if (new_ratio > target_ratio) ^ (right_ratio > target_ratio) {
                        left = new;
                    } else {
                        right = new;
                        right_ratio = new_ratio;
                    }
                }
                Err(error_msg!("binary search iterations exhausted"))
            };

        let amount =
            if initial_ratio <= min_ratio && point0_ratio < min_ratio && point1_ratio < min_ratio {
                // If we have to stay below the target ratio, pick the highest one
                if point0_ratio > initial_ratio {
                    if point1_ratio > point0_ratio {
                        point1_amount
                    } else {
                        point0_amount
                    }
                } else if point1_ratio > initial_ratio {
                    point1_amount
                } else {
                    I80F48::ZERO
                }
            } else if point1_ratio >= min_ratio {
                // If point1_ratio is still bigger than min_ratio, the target amount must be >point1_amount
                // search to the right of point1_amount: but how far?
                // At point1, source.balance < 0 and target.balance > 0, so use a simple estimation for
                // zero health: health - source_liab_weight * a + target_asset_weight * a = 0.
                if point1_health <= 0 {
                    return Ok(I80F48::ZERO);
                }
                let zero_health_amount = point1_amount
                    + point1_health / (source.init_liab_weight - target.init_asset_weight);
                let zero_health_ratio = health_ratio_after_swap(zero_health_amount);
                binary_approximation_search(
                    point1_amount,
                    point1_ratio,
                    zero_health_amount,
                    zero_health_ratio,
                    min_ratio,
                )?
            } else if point0_ratio >= min_ratio {
                // Must be between point0_amount and point1_amount.
                binary_approximation_search(
                    point0_amount,
                    point0_ratio,
                    point1_amount,
                    point1_ratio,
                    min_ratio,
                )?
            } else {
                // can't happen because slope between 0 and point0_amount is positive!
                return Err(error_msg!(
                    "internal error: assert that init ratio {} <= point0 ratio {}",
                    initial_ratio,
                    point0_ratio
                ));
            };

        Ok(amount / source.oracle_price)
    }
}

fn find_token_info_index(infos: &[TokenInfo], token_index: TokenIndex) -> Result<usize> {
    infos
        .iter()
        .position(|ti| ti.token_index == token_index)
        .ok_or_else(|| error_msg!("token index {} not found", token_index))
}

/// Generate a HealthCache for an account and its health accounts.
pub fn new_health_cache(
    account: &MangoAccount,
    retriever: &impl AccountRetriever,
) -> Result<HealthCache> {
    // token contribution from token accounts
    let mut token_infos = vec![];

    for (i, position) in account.tokens.iter_active().enumerate() {
        let (bank, oracle_price) =
            retriever.bank_and_oracle(&account.group, i, position.token_index)?;

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
            serum3_max_reserved: I80F48::ZERO,
        });
    }

    // Fill the TokenInfo balance with free funds in serum3 oo accounts, and fill
    // the serum3_max_reserved with their reserved funds. Also build Serum3Infos.
    let mut serum3_infos = vec![];
    for (i, serum_account) in account.serum3.iter_active().enumerate() {
        let oo = retriever.serum_oo(i, &serum_account.open_orders)?;

        // find the TokenInfos for the market's base and quote tokens
        let base_index = find_token_info_index(&token_infos, serum_account.base_token_index)?;
        let quote_index = find_token_info_index(&token_infos, serum_account.quote_token_index)?;
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

        // add the reserved amount to both sides, to have the worst-case covered
        let reserved_base = I80F48::from_num(cm!(oo.native_coin_total - oo.native_coin_free));
        let reserved_quote = I80F48::from_num(cm!(oo.native_pc_total - oo.native_pc_free));
        let reserved_balance =
            cm!(reserved_base * base_info.oracle_price + reserved_quote * quote_info.oracle_price);
        base_info.serum3_max_reserved = cm!(base_info.serum3_max_reserved + reserved_balance);
        quote_info.serum3_max_reserved = cm!(quote_info.serum3_max_reserved + reserved_balance);

        serum3_infos.push(Serum3Info {
            reserved: reserved_balance,
            base_index,
            quote_index,
        });
    }

    // TODO: also account for perp funding in health
    // health contribution from perp accounts
    let mut perp_infos = Vec::with_capacity(account.perps.iter_active_accounts().count());
    for (i, perp_account) in account.perps.iter_active_accounts().enumerate() {
        let perp_market = retriever.perp_market(&account.group, i, perp_account.market_index)?;

        // find the TokenInfos for the market's base and quote tokens
        let base_index = find_token_info_index(&token_infos, perp_market.base_token_index)?;
        // TODO: base_index could be unset
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
        serum3_infos,
        perp_infos,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::oracle::StubOracle;
    use std::cell::RefCell;
    use std::convert::identity;
    use std::mem::size_of;
    use std::rc::Rc;
    use std::str::FromStr;

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

    // Implementing TestAccount directly for ZeroCopy + Owner leads to a conflict
    // because OpenOrders may add impls for those in the future.
    trait MyZeroCopy: anchor_lang::ZeroCopy + Owner {}
    impl MyZeroCopy for StubOracle {}
    impl MyZeroCopy for Bank {}
    impl MyZeroCopy for PerpMarket {}

    struct TestAccount<T> {
        bytes: Vec<u8>,
        pubkey: Pubkey,
        owner: Pubkey,
        lamports: u64,
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> TestAccount<T> {
        fn new(bytes: Vec<u8>, owner: Pubkey) -> Self {
            Self {
                bytes,
                owner,
                pubkey: Pubkey::new_unique(),
                lamports: 0,
                _phantom: std::marker::PhantomData,
            }
        }

        fn as_account_info(&mut self) -> AccountInfo {
            AccountInfo {
                key: &self.pubkey,
                owner: &self.owner,
                lamports: Rc::new(RefCell::new(&mut self.lamports)),
                data: Rc::new(RefCell::new(&mut self.bytes)),
                is_signer: false,
                is_writable: false,
                executable: false,
                rent_epoch: 0,
            }
        }
    }

    impl<T: MyZeroCopy> TestAccount<T> {
        fn new_zeroed() -> Self {
            let mut bytes = vec![0u8; 8 + size_of::<T>()];
            bytes[0..8].copy_from_slice(&T::discriminator());
            Self::new(bytes, T::owner())
        }

        fn data(&mut self) -> &mut T {
            bytemuck::from_bytes_mut(&mut self.bytes[8..])
        }
    }

    impl TestAccount<OpenOrders> {
        fn new_zeroed() -> Self {
            let mut bytes = vec![0u8; 12 + size_of::<OpenOrders>()];
            bytes[0..5].copy_from_slice(b"serum");
            Self::new(bytes, Pubkey::new_unique())
        }

        fn data(&mut self) -> &mut OpenOrders {
            bytemuck::from_bytes_mut(&mut self.bytes[5..5 + size_of::<OpenOrders>()])
        }
    }

    fn mock_bank_and_oracle(
        group: Pubkey,
        token_index: TokenIndex,
        price: f64,
        init_weights: f64,
        maint_weights: f64,
    ) -> (TestAccount<Bank>, TestAccount<StubOracle>) {
        let mut oracle = TestAccount::<StubOracle>::new_zeroed();
        oracle.data().price = I80F48::from_num(price);
        let mut bank = TestAccount::<Bank>::new_zeroed();
        bank.data().token_index = token_index;
        bank.data().group = group;
        bank.data().oracle = oracle.pubkey;
        bank.data().deposit_index = I80F48::from(1_000_000);
        bank.data().borrow_index = I80F48::from(1_000_000);
        bank.data().init_asset_weight = I80F48::from_num(1.0 - init_weights);
        bank.data().init_liab_weight = I80F48::from_num(1.0 + init_weights);
        bank.data().maint_asset_weight = I80F48::from_num(1.0 - maint_weights);
        bank.data().maint_liab_weight = I80F48::from_num(1.0 + maint_weights);
        (bank, oracle)
    }

    // Run a health test that includes all the side values (like referrer_rebates_accrued)
    #[test]
    fn test_health0() {
        let mut account = MangoAccount::default();
        let group = Pubkey::new_unique();

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 1, 1.0, 0.2, 0.1);
        let (mut bank2, mut oracle2) = mock_bank_and_oracle(group, 4, 5.0, 0.5, 0.3);
        bank1
            .data()
            .deposit(
                account.tokens.get_mut_or_create(1).unwrap().0,
                I80F48::from(100),
            )
            .unwrap();
        bank2
            .data()
            .withdraw_without_fee(
                account.tokens.get_mut_or_create(4).unwrap().0,
                I80F48::from(10),
            )
            .unwrap();

        let mut oo1 = TestAccount::<OpenOrders>::new_zeroed();
        let serum3account = account.serum3.create(2).unwrap();
        serum3account.open_orders = oo1.pubkey;
        serum3account.base_token_index = 4;
        serum3account.quote_token_index = 1;
        oo1.data().native_pc_total = 21;
        oo1.data().native_coin_total = 18;
        oo1.data().native_pc_free = 1;
        oo1.data().native_coin_free = 3;
        oo1.data().referrer_rebates_accrued = 2;

        let mut perp1 = TestAccount::<PerpMarket>::new_zeroed();
        perp1.data().group = group;
        perp1.data().perp_market_index = 9;
        perp1.data().base_token_index = 4;
        perp1.data().init_asset_weight = I80F48::from_num(1.0 - 0.2f64);
        perp1.data().init_liab_weight = I80F48::from_num(1.0 + 0.2f64);
        perp1.data().maint_asset_weight = I80F48::from_num(1.0 - 0.1f64);
        perp1.data().maint_liab_weight = I80F48::from_num(1.0 + 0.1f64);
        perp1.data().quote_lot_size = 100;
        perp1.data().base_lot_size = 10;
        let perpaccount = account.perps.get_account_mut_or_create(9).unwrap().0;
        perpaccount.base_position_lots = 3;
        perpaccount.quote_position_native = -I80F48::from(310u16);
        perpaccount.bids_base_lots = 7;
        perpaccount.asks_base_lots = 11;
        perpaccount.taker_base_lots = 1;
        perpaccount.taker_quote_lots = 2;

        let ais = vec![
            bank1.as_account_info(),
            bank2.as_account_info(),
            oracle1.as_account_info(),
            oracle2.as_account_info(),
            perp1.as_account_info(),
            oo1.as_account_info(),
        ];

        let retriever = ScanningAccountRetriever::new(&ais, &group).unwrap();

        let health_eq = |a: I80F48, b: f64| {
            if (a - I80F48::from_num(b)).abs() < 0.001 {
                true
            } else {
                println!("health is {}, but expected {}", a, b);
                false
            }
        };

        // for bank1/oracle1, including open orders (scenario: bids execute)
        let health1 = (100.0 + 1.0 + 2.0 + (20.0 + 15.0 * 5.0)) * 0.8;
        // for bank2/oracle2
        let health2 = (-10.0 + 3.0) * 5.0 * 1.5;
        // for perp (scenario: bids execute)
        let health3 =
            (3.0 + 7.0 + 1.0) * 10.0 * 5.0 * 0.8 + (-310.0 + 2.0 * 100.0 - 7.0 * 10.0 * 5.0);
        assert!(health_eq(
            compute_health(&account, HealthType::Init, &retriever).unwrap(),
            health1 + health2 + health3
        ));
    }

    #[test]
    fn test_scanning_account_retriever() {
        let group = Pubkey::new_unique();

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 1, 1.0, 0.2, 0.1);
        let (mut bank2, mut oracle2) = mock_bank_and_oracle(group, 4, 5.0, 0.5, 0.3);

        let mut oo1 = TestAccount::<OpenOrders>::new_zeroed();
        let oo1key = oo1.pubkey;
        oo1.data().native_pc_total = 20;

        let mut perp1 = TestAccount::<PerpMarket>::new_zeroed();
        perp1.data().group = group;
        perp1.data().perp_market_index = 9;

        let ais = vec![
            bank1.as_account_info(),
            bank2.as_account_info(),
            oracle1.as_account_info(),
            oracle2.as_account_info(),
            perp1.as_account_info(),
            oo1.as_account_info(),
        ];

        let mut retriever = ScanningAccountRetriever::new(&ais, &group).unwrap();

        assert_eq!(retriever.n_banks(), 2);
        assert_eq!(retriever.begin_serum3(), 5);
        assert_eq!(retriever.perp_index_map.len(), 1);

        {
            let (b1, o1, opt_b2o2) = retriever.banks_mut_and_oracles(1, 4).unwrap();
            let (b2, o2) = opt_b2o2.unwrap();
            assert_eq!(b1.token_index, 1);
            assert_eq!(o1, I80F48::ONE);
            assert_eq!(b2.token_index, 4);
            assert_eq!(o2, 5 * I80F48::ONE);
        }

        {
            let (b1, o1, opt_b2o2) = retriever.banks_mut_and_oracles(4, 1).unwrap();
            let (b2, o2) = opt_b2o2.unwrap();
            assert_eq!(b1.token_index, 4);
            assert_eq!(o1, 5 * I80F48::ONE);
            assert_eq!(b2.token_index, 1);
            assert_eq!(o2, I80F48::ONE);
        }

        {
            let (b1, o1, opt_b2o2) = retriever.banks_mut_and_oracles(4, 4).unwrap();
            assert!(opt_b2o2.is_none());
            assert_eq!(b1.token_index, 4);
            assert_eq!(o1, 5 * I80F48::ONE);
        }

        retriever.banks_mut_and_oracles(4, 2).unwrap_err();

        let oo = retriever.serum_oo(0, &oo1key).unwrap();
        assert_eq!(identity(oo.native_pc_total), 20);

        assert!(retriever.serum_oo(1, &Pubkey::default()).is_err());

        let perp = retriever.perp_market(&group, 0, 9).unwrap();
        assert_eq!(identity(perp.perp_market_index), 9);

        assert!(retriever.perp_market(&group, 1, 5).is_err());
    }

    #[derive(Default)]
    struct TestHealth1Case {
        token1: i64,
        token2: i64,
        token3: i64,
        oo_1_2: (u64, u64),
        oo_1_3: (u64, u64),
        perp1: (i64, i64, i64, i64),
        expected_health: f64,
    }
    fn test_health1_runner(testcase: &TestHealth1Case) {
        let mut account = MangoAccount::default();
        let group = Pubkey::new_unique();

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 1, 1.0, 0.2, 0.1);
        let (mut bank2, mut oracle2) = mock_bank_and_oracle(group, 4, 5.0, 0.5, 0.3);
        let (mut bank3, mut oracle3) = mock_bank_and_oracle(group, 5, 10.0, 0.5, 0.3);
        bank1
            .data()
            .change_without_fee(
                account.tokens.get_mut_or_create(1).unwrap().0,
                I80F48::from(testcase.token1),
            )
            .unwrap();
        bank2
            .data()
            .change_without_fee(
                account.tokens.get_mut_or_create(4).unwrap().0,
                I80F48::from(testcase.token2),
            )
            .unwrap();
        bank3
            .data()
            .change_without_fee(
                account.tokens.get_mut_or_create(5).unwrap().0,
                I80F48::from(testcase.token3),
            )
            .unwrap();

        let mut oo1 = TestAccount::<OpenOrders>::new_zeroed();
        let serum3account1 = account.serum3.create(2).unwrap();
        serum3account1.open_orders = oo1.pubkey;
        serum3account1.base_token_index = 4;
        serum3account1.quote_token_index = 1;
        oo1.data().native_pc_total = testcase.oo_1_2.0;
        oo1.data().native_coin_total = testcase.oo_1_2.1;

        let mut oo2 = TestAccount::<OpenOrders>::new_zeroed();
        let serum3account2 = account.serum3.create(3).unwrap();
        serum3account2.open_orders = oo2.pubkey;
        serum3account2.base_token_index = 5;
        serum3account2.quote_token_index = 1;
        oo2.data().native_pc_total = testcase.oo_1_3.0;
        oo2.data().native_coin_total = testcase.oo_1_3.1;

        let mut perp1 = TestAccount::<PerpMarket>::new_zeroed();
        perp1.data().group = group;
        perp1.data().perp_market_index = 9;
        perp1.data().base_token_index = 4;
        perp1.data().init_asset_weight = I80F48::from_num(1.0 - 0.2f64);
        perp1.data().init_liab_weight = I80F48::from_num(1.0 + 0.2f64);
        perp1.data().maint_asset_weight = I80F48::from_num(1.0 - 0.1f64);
        perp1.data().maint_liab_weight = I80F48::from_num(1.0 + 0.1f64);
        perp1.data().quote_lot_size = 100;
        perp1.data().base_lot_size = 10;
        let perpaccount = account.perps.get_account_mut_or_create(9).unwrap().0;
        perpaccount.base_position_lots = testcase.perp1.0;
        perpaccount.quote_position_native = I80F48::from(testcase.perp1.1);
        perpaccount.bids_base_lots = testcase.perp1.2;
        perpaccount.asks_base_lots = testcase.perp1.3;

        let ais = vec![
            bank1.as_account_info(),
            bank2.as_account_info(),
            bank3.as_account_info(),
            oracle1.as_account_info(),
            oracle2.as_account_info(),
            oracle3.as_account_info(),
            perp1.as_account_info(),
            oo1.as_account_info(),
            oo2.as_account_info(),
        ];

        let retriever = ScanningAccountRetriever::new(&ais, &group).unwrap();

        let health_eq = |a: I80F48, b: f64| {
            if (a - I80F48::from_num(b)).abs() < 0.001 {
                true
            } else {
                println!("health is {}, but expected {}", a, b);
                false
            }
        };

        assert!(health_eq(
            compute_health(&account, HealthType::Init, &retriever).unwrap(),
            testcase.expected_health
        ));
    }

    // Check some specific health constellations
    #[test]
    fn test_health1() {
        let base_price = 5.0;
        let base_lots_to_quote = 10.0 * base_price;
        let testcases = vec![
            TestHealth1Case { // 0
                token1: 100,
                token2: -10,
                oo_1_2: (20, 15),
                perp1: (3, -131, 7, 11),
                expected_health:
                    // for token1, including open orders (scenario: bids execute)
                    (100.0 + (20.0 + 15.0 * base_price)) * 0.8
                    // for token2
                    - 10.0 * base_price * 1.5
                    // for perp (scenario: bids execute)
                    + (3.0 + 7.0) * base_lots_to_quote * 0.8 + (-131.0 - 7.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case { // 1
                token1: -100,
                token2: 10,
                oo_1_2: (20, 15),
                perp1: (-10, -131, 7, 11),
                expected_health:
                    // for token1
                    -100.0 * 1.2
                    // for token2, including open orders (scenario: asks execute)
                    + (10.0 * base_price + (20.0 + 15.0 * base_price)) * 0.5
                    // for perp (scenario: asks execute)
                    + (-10.0 - 11.0) * base_lots_to_quote * 1.2 + (-131.0 + 11.0 * base_lots_to_quote),
                ..Default::default()
            },
            TestHealth1Case {
                // 2
                perp1: (-1, 100, 0, 0),
                expected_health: 0.0,
                ..Default::default()
            },
            TestHealth1Case {
                // 3
                perp1: (1, -100, 0, 0),
                expected_health: -100.0 + 0.8 * 1.0 * base_lots_to_quote,
                ..Default::default()
            },
            TestHealth1Case {
                // 4
                perp1: (10, 100, 0, 0),
                expected_health: 0.0,
                ..Default::default()
            },
            TestHealth1Case {
                // 5
                perp1: (30, -100, 0, 0),
                expected_health: 0.0,
                ..Default::default()
            },
            TestHealth1Case { // 6, reserved oo funds
                token1: -100,
                token2: -10,
                token3: -10,
                oo_1_2: (1, 1),
                oo_1_3: (1, 1),
                expected_health:
                    // tokens
                    -100.0 * 1.2 - 10.0 * 5.0 * 1.5 - 10.0 * 10.0 * 1.5
                    // oo_1_2 (-> token1)
                    + (1.0 + 5.0) * 1.2
                    // oo_1_3 (-> token1)
                    + (1.0 + 10.0) * 1.2,
                ..Default::default()
            },
            TestHealth1Case { // 7, reserved oo funds cross the zero balance level
                token1: -14,
                token2: -10,
                token3: -10,
                oo_1_2: (1, 1),
                oo_1_3: (1, 1),
                expected_health:
                    // tokens
                    -14.0 * 1.2 - 10.0 * 5.0 * 1.5 - 10.0 * 10.0 * 1.5
                    // oo_1_2 (-> token1)
                    + 3.0 * 1.2 + 3.0 * 0.8
                    // oo_1_3 (-> token1)
                    + 8.0 * 1.2 + 3.0 * 0.8,
                ..Default::default()
            },
            TestHealth1Case { // 8, reserved oo funds in a non-quote currency
                token1: -100,
                token2: -100,
                token3: -1,
                oo_1_2: (0, 0),
                oo_1_3: (10, 1),
                expected_health:
                    // tokens
                    -100.0 * 1.2 - 100.0 * 5.0 * 1.5 - 10.0 * 1.5
                    // oo_1_3 (-> token3)
                    + 10.0 * 1.5 + 10.0 * 0.5,
                ..Default::default()
            },
            TestHealth1Case { // 9, like 8 but oo_1_2 flips the oo_1_3 target
                token1: -100,
                token2: -100,
                token3: -1,
                oo_1_2: (100, 0),
                oo_1_3: (10, 1),
                expected_health:
                    // tokens
                    -100.0 * 1.2 - 100.0 * 5.0 * 1.5 - 10.0 * 1.5
                    // oo_1_2 (-> token1)
                    + 80.0 * 1.2 + 20.0 * 0.8
                    // oo_1_3 (-> token1)
                    + 20.0 * 0.8,
                ..Default::default()
            },
        ];

        for (i, testcase) in testcases.iter().enumerate() {
            println!("checking testcase {}", i);
            test_health1_runner(&testcase);
        }
    }

    #[test]
    fn test_max_swap() {
        let default_token_info = |x| TokenInfo {
            token_index: 0,
            maint_asset_weight: I80F48::from_num(1.0 - x),
            init_asset_weight: I80F48::from_num(1.0 - x),
            maint_liab_weight: I80F48::from_num(1.0 + x),
            init_liab_weight: I80F48::from_num(1.0 + x),
            oracle_price: I80F48::from_num(2.0),
            balance: I80F48::ZERO,
            serum3_max_reserved: I80F48::ZERO,
        };

        let health_cache = HealthCache {
            token_infos: vec![
                TokenInfo {
                    token_index: 0,
                    oracle_price: I80F48::from_num(2.0),
                    balance: I80F48::ZERO,
                    ..default_token_info(0.1)
                },
                TokenInfo {
                    token_index: 1,
                    oracle_price: I80F48::from_num(3.0),
                    balance: I80F48::ZERO,
                    ..default_token_info(0.2)
                },
                TokenInfo {
                    token_index: 2,
                    oracle_price: I80F48::from_num(4.0),
                    balance: I80F48::ZERO,
                    ..default_token_info(0.3)
                },
            ],
            serum3_infos: vec![],
            perp_infos: vec![],
        };

        assert_eq!(health_cache.health(HealthType::Init), I80F48::ZERO);
        assert_eq!(health_cache.health_ratio(HealthType::Init), I80F48::MAX);
        assert_eq!(
            health_cache
                .max_swap_source_for_health_ratio(0, 1, I80F48::from_num(50.0))
                .unwrap(),
            I80F48::ZERO
        );

        let adjust_by_usdc = |c: &mut HealthCache, ti: TokenIndex, usdc: f64| {
            let ti = &mut c.token_infos[ti as usize];
            ti.balance += I80F48::from_num(usdc);
        };
        let find_max_swap_actual =
            |c: &HealthCache, source: TokenIndex, target: TokenIndex, ratio: f64| {
                let source_amount = c
                    .max_swap_source_for_health_ratio(source, target, I80F48::from_num(ratio))
                    .unwrap();
                let mut c = c.clone();
                let source_price = c.token_infos[source as usize].oracle_price;
                let target_price = c.token_infos[target as usize].oracle_price;
                c.adjust_token_balance(source, -source_amount).unwrap();
                c.adjust_token_balance(target, source_amount * source_price / target_price)
                    .unwrap();
                (
                    source_amount.to_num::<f64>(),
                    c.health_ratio(HealthType::Init).to_num::<f64>(),
                )
            };
        let check_max_swap_result =
            |c: &HealthCache, source: TokenIndex, target: TokenIndex, ratio: f64| {
                let (source_amount, actual_ratio) = find_max_swap_actual(c, source, target, ratio);
                println!(
                    "checking {} to {} for target ratio {}: actual ratio: {}, amount: {}",
                    source, target, ratio, actual_ratio, source_amount
                );
                assert!((ratio - actual_ratio).abs() < 1.0);
            };

        {
            println!("test 0");
            let mut health_cache = health_cache.clone();
            adjust_by_usdc(&mut health_cache, 1, 100.0);
            check_max_swap_result(&health_cache, 0, 1, 50.0);
            check_max_swap_result(&health_cache, 1, 0, 50.0);
            check_max_swap_result(&health_cache, 0, 2, 50.0);
        }

        {
            println!("test 1");
            let mut health_cache = health_cache.clone();
            adjust_by_usdc(&mut health_cache, 0, -20.0);
            adjust_by_usdc(&mut health_cache, 1, 100.0);
            check_max_swap_result(&health_cache, 0, 1, 50.0);
            check_max_swap_result(&health_cache, 1, 0, 50.0);
            check_max_swap_result(&health_cache, 0, 2, 50.0);
            check_max_swap_result(&health_cache, 2, 0, 50.0);
        }

        {
            println!("test 2");
            let mut health_cache = health_cache.clone();
            adjust_by_usdc(&mut health_cache, 0, -50.0);
            adjust_by_usdc(&mut health_cache, 1, 100.0);
            // possible even though the init ratio is <100
            check_max_swap_result(&health_cache, 1, 0, 100.0);
        }

        {
            println!("test 3");
            let mut health_cache = health_cache.clone();
            adjust_by_usdc(&mut health_cache, 0, -30.0);
            adjust_by_usdc(&mut health_cache, 1, 100.0);
            adjust_by_usdc(&mut health_cache, 2, -30.0);

            // swapping with a high ratio advises paying back all liabs
            // and then swapping even more because increasing assets in 0 has better asset weight
            let init_ratio = health_cache.health_ratio(HealthType::Init);
            let (amount, actual_ratio) = find_max_swap_actual(&health_cache, 1, 0, 100.0);
            println!(
                "init {}, after {}, amount {}",
                init_ratio, actual_ratio, amount
            );
            assert!(actual_ratio / 2.0 > init_ratio);
            assert!((amount - 100.0 / 3.0).abs() < 1.0);
        }

        {
            println!("test 4");
            let mut health_cache = health_cache.clone();
            adjust_by_usdc(&mut health_cache, 0, 100.0);
            adjust_by_usdc(&mut health_cache, 1, -2.0);
            adjust_by_usdc(&mut health_cache, 2, -65.0);

            let init_ratio = health_cache.health_ratio(HealthType::Init);
            assert!(init_ratio > 3 && init_ratio < 4);

            check_max_swap_result(&health_cache, 0, 1, 1.0);
            check_max_swap_result(&health_cache, 0, 1, 3.0);
            check_max_swap_result(&health_cache, 0, 1, 4.0);
        }
    }
}
