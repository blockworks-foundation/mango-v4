use anchor_lang::prelude::*;
use fixed::types::I80F48;
use serum_dex::state::OpenOrders;
use std::cell::{Ref, RefMut};
use std::collections::HashMap;

use crate::error::MangoError;
use crate::serum3_cpi;
use crate::state::{oracle_price, Bank, MangoAccount, TokenIndex};
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
}

/// Assumes the account infos needed for the health computation follow a strict order.
///
/// 1. n_banks Bank account, in the order of account.token_account_map.iter_active()
/// 2. n_banks oracle accounts, one for each bank in the same order
/// 3. serum3 OpenOrders accounts, in the order of account.serum3_account_map.iter_active()
pub struct FixedOrderAccountRetriever<'a, 'b> {
    ais: &'a [AccountInfo<'b>],
    n_banks: usize,
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
        let oracle = &self.ais[self.n_banks + account_index];
        require!(&bank.oracle == oracle.key, MangoError::SomeError);
        Ok((bank, oracle))
    }

    fn serum_oo(&self, account_index: usize, key: &Pubkey) -> Result<Ref<'a, OpenOrders>> {
        let ai = &self.ais[2 * self.n_banks + account_index];
        require!(key == ai.key, MangoError::SomeError);
        serum3_cpi::load_open_orders(ai)
    }
}

/// Takes a list of account infos containing
/// - an unknown number of Banks in any order, followed by
/// - the same number of oracles in the same order as the banks, followed by
/// - an unknown number of serum3 OpenOrders accounts
/// and retrieves accounts needed for the health computation by doing a linear
/// scan for each request.
pub struct ScanningAccountRetriever<'a, 'b> {
    ais: &'a [AccountInfo<'b>],
    token_index_map: HashMap<TokenIndex, usize>,
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
        Ok(Self {
            ais,
            token_index_map,
        })
    }

    fn n_banks(&self) -> usize {
        self.token_index_map.len()
    }

    #[inline]
    fn bank_index(&self, token_index: TokenIndex) -> Result<usize> {
        Ok(*self
            .token_index_map
            .get(&token_index)
            .ok_or_else(|| error!(MangoError::SomeError))?)
    }

    pub fn bank_mut_and_oracle(
        &self,
        token_index: TokenIndex,
    ) -> Result<(RefMut<'a, Bank>, &'a AccountInfo<'b>)> {
        let index = self.bank_index(token_index)?;
        let bank = self.ais[index].load_mut_fully_unchecked::<Bank>()?;
        let oracle = &self.ais[self.n_banks() + index];
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
        let oracle = &self.ais[self.n_banks() + index];
        require!(&bank.oracle == oracle.key, MangoError::SomeError);
        Ok((bank, oracle))
    }

    fn serum_oo(&self, _account_index: usize, key: &Pubkey) -> Result<Ref<'a, OpenOrders>> {
        let oo = self.ais[2 * self.n_banks()..]
            .iter()
            .find(|ai| ai.key == key)
            .unwrap();
        serum3_cpi::load_open_orders(oo)
    }
}

pub fn compute_health_from_fixed_accounts<'a, 'b>(
    account: &MangoAccount,
    ais: &'a [AccountInfo<'b>],
) -> Result<I80F48> {
    let active_token_len = account.token_account_map.iter_active().count();
    let active_serum_len = account.serum3_account_map.iter_active().count();
    let expected_ais = active_token_len * 2 // banks + oracles
        + active_serum_len; // open_orders
    require!(ais.len() == expected_ais, MangoError::SomeError);

    let retriever = FixedOrderAccountRetriever {
        ais,
        n_banks: active_token_len,
    };
    compute_health_detail(account, &retriever)
}

pub fn compute_health<'a, 'b: 'a>(
    account: &MangoAccount,
    retriever: &impl AccountRetriever<'a, 'b>,
) -> Result<I80F48> {
    compute_health_detail(account, retriever)
}

struct TokenInfo<'a> {
    bank: Ref<'a, Bank>,
    oracle_price: I80F48, // native/native
    // in native tokens, summing token deposits/borrows and serum open orders
    balance: I80F48,

    // optimization to avoid computing these multiplications multiple times
    price_liab_cache: I80F48,
    price_asset_cache: I80F48,
    price_inv_cache: I80F48,
}

impl<'a> TokenInfo<'a> {
    #[inline(always)]
    fn price_liab(&mut self) -> I80F48 {
        if self.price_liab_cache.is_zero() {
            self.price_liab_cache = self.oracle_price * self.bank.init_liab_weight;
        }
        self.price_liab_cache
    }

    #[inline(always)]
    fn price_asset(&mut self) -> I80F48 {
        if self.price_asset_cache.is_zero() {
            self.price_asset_cache = self.oracle_price * self.bank.init_asset_weight;
        }
        self.price_asset_cache
    }

    #[inline(always)]
    fn price_inv(&mut self) -> I80F48 {
        if self.price_inv_cache.is_zero() {
            self.price_inv_cache = I80F48::ONE / self.oracle_price;
        }
        self.price_inv_cache
    }
}

/// Compute health contribution for a given balance
/// wart: independent of the balance stored in TokenInfo
#[inline(always)]
fn health_contribution(info: &mut TokenInfo, balance: I80F48) -> Result<I80F48> {
    Ok(if balance.is_negative() {
        cm!(balance * info.price_liab())
    } else {
        cm!(balance * info.price_asset())
    })
}

/// Compute health contribution of two tokens - pure convenience
#[inline(always)]
fn pair_health(
    info1: &mut TokenInfo,
    balance1: I80F48,
    info2: &mut TokenInfo,
    balance2: I80F48,
) -> Result<I80F48> {
    let health1 = health_contribution(info1, balance1)?;
    let health2 = health_contribution(info2, balance2)?;
    Ok(cm!(health1 + health2))
}

fn compute_health_detail<'a, 'b: 'a>(
    account: &MangoAccount,
    retriever: &impl AccountRetriever<'a, 'b>,
) -> Result<I80F48> {
    // token contribution from token accounts
    let mut token_infos = vec![];
    for (i, position) in account.token_account_map.iter_active().enumerate() {
        let (bank, oracle_ai) =
            retriever.bank_and_oracle(&account.group, i, position.token_index)?;
        let oracle_price = oracle_price(oracle_ai)?;

        // converts the token value to the basis token value for health computations
        // TODO: health basis token == USDC?
        let native = position.native(&bank);

        token_infos.push(TokenInfo {
            bank,
            oracle_price,
            balance: native,
            price_asset_cache: I80F48::ZERO,
            price_liab_cache: I80F48::ZERO,
            price_inv_cache: I80F48::ZERO,
        });
    }

    // token contribution from serum accounts
    for (i, serum_account) in account.serum3_account_map.iter_active().enumerate() {
        let oo = retriever.serum_oo(i, &serum_account.open_orders)?;

        // find the TokenInfos for the market's base and quote tokens
        let base_index = token_infos
            .iter()
            .position(|ti| ti.bank.token_index == serum_account.base_token_index)
            .ok_or_else(|| error!(MangoError::SomeError))?;
        let quote_index = token_infos
            .iter()
            .position(|ti| ti.bank.token_index == serum_account.quote_token_index)
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
        base_info.balance = cm!(base_info.balance + base_free);
        quote_info.balance = cm!(quote_info.balance + quote_free);

        // for the amounts that are reserved for orders, compute the worst case for health
        // by checking if everything-is-base or everything-is-quote produces worse
        // outcomes
        let reserved_base = I80F48::from_num(cm!(oo.native_coin_total - oo.native_coin_free));
        let reserved_quote = I80F48::from_num(cm!(oo.native_pc_total - oo.native_pc_free));
        let all_in_base = cm!(base_info.balance
            + reserved_base
            + reserved_quote * quote_info.oracle_price * base_info.price_inv());
        let all_in_quote = cm!(quote_info.balance
            + reserved_quote
            + reserved_base * base_info.oracle_price * quote_info.price_inv());
        if pair_health(base_info, all_in_base, quote_info, quote_info.balance)?
            < pair_health(base_info, base_info.balance, quote_info, all_in_quote)?
        {
            base_info.balance = all_in_base;
        } else {
            quote_info.balance = all_in_quote;
        }
    }

    // convert the token balance to health
    let mut health = I80F48::ZERO;
    for token_info in token_infos.iter_mut() {
        let contrib = health_contribution(token_info, token_info.balance)?;
        health = cm!(health + contrib);
    }

    Ok(health)
}
