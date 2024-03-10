use anchor_lang::prelude::*;
use anchor_lang::ZeroCopy;

use fixed::types::I80F48;
use serum_dex::state::OpenOrders;

use std::cell::Ref;
use std::collections::HashMap;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::serum3_cpi;
use crate::state::pyth_mainnet_sol_oracle;
use crate::state::pyth_mainnet_usdc_oracle;
use crate::state::OracleAccountInfos;
use crate::state::{Bank, MangoAccountRef, PerpMarket, PerpMarketIndex, TokenIndex};

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
        active_token_position_index: usize,
        token_index: TokenIndex,
    ) -> Result<(&Bank, I80F48)>;

    fn serum_oo(&self, active_serum_oo_index: usize, key: &Pubkey) -> Result<&OpenOrders>;

    fn perp_market_and_oracle_price(
        &self,
        group: &Pubkey,
        active_perp_position_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<(&PerpMarket, I80F48)>;
}

/// Assumes the account infos needed for the health computation follow a strict order.
///
/// 1. n_banks Bank account, in the order of account.token_iter_active()
/// 2. n_banks oracle accounts, one for each bank in the same order
/// 3. PerpMarket accounts, in the order of account.perps.iter_active_accounts()
/// 4. PerpMarket oracle accounts, in the order of the perp market accounts
/// 5. serum3 OpenOrders accounts, in the order of account.serum3.iter_active()
/// 6. fallback oracle accounts, order and existence of accounts is not guaranteed
pub struct FixedOrderAccountRetriever<T: KeyedAccountReader> {
    pub ais: Vec<T>,
    pub n_banks: usize,
    pub n_perps: usize,
    pub begin_perp: usize,
    pub begin_serum3: usize,
    pub staleness_slot: Option<u64>,
    pub begin_fallback_oracles: usize,
    pub usdc_oracle_index: Option<usize>,
    pub sol_oracle_index: Option<usize>,
}

pub fn new_fixed_order_account_retriever<'a, 'info>(
    ais: &'a [AccountInfo<'info>],
    account: &MangoAccountRef,
) -> Result<FixedOrderAccountRetriever<AccountInfoRef<'a, 'info>>> {
    let active_token_len = account.active_token_positions().count();
    let active_serum3_len = account.active_serum3_orders().count();
    let active_perp_len = account.active_perp_positions().count();
    let expected_ais = active_token_len * 2 // banks + oracles
        + active_perp_len * 2 // PerpMarkets + Oracles
        + active_serum3_len; // open_orders
    require_msg_typed!(ais.len() >= expected_ais, MangoError::InvalidHealthAccountCount,
        "received {} accounts but expected {} ({} banks, {} bank oracles, {} perp markets, {} perp oracles, {} serum3 oos)",
        ais.len(), expected_ais,
        active_token_len, active_token_len, active_perp_len, active_perp_len, active_serum3_len
    );
    let usdc_oracle_index = ais[..]
        .iter()
        .position(|o| o.key == &pyth_mainnet_usdc_oracle::ID);
    let sol_oracle_index = ais[..]
        .iter()
        .position(|o| o.key == &pyth_mainnet_sol_oracle::ID);

    Ok(FixedOrderAccountRetriever {
        ais: AccountInfoRef::borrow_slice(ais)?,
        n_banks: active_token_len,
        n_perps: active_perp_len,
        begin_perp: active_token_len * 2,
        begin_serum3: active_token_len * 2 + active_perp_len * 2,
        staleness_slot: Some(Clock::get()?.slot),
        begin_fallback_oracles: expected_ais,
        usdc_oracle_index,
        sol_oracle_index,
    })
}

impl<T: KeyedAccountReader> FixedOrderAccountRetriever<T> {
    fn bank(&self, group: &Pubkey, account_index: usize, token_index: TokenIndex) -> Result<&Bank> {
        let bank = self.ais[account_index].load::<Bank>()?;
        require_keys_eq!(bank.group, *group);
        require_eq!(bank.token_index, token_index);
        Ok(bank)
    }

    fn perp_market(
        &self,
        group: &Pubkey,
        account_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<&PerpMarket> {
        let market_ai = &self.ais[account_index];
        let market = market_ai.load::<PerpMarket>()?;
        require_keys_eq!(market.group, *group);
        require_eq!(market.perp_market_index, perp_market_index);
        Ok(market)
    }

    fn oracle_price_perp(&self, account_index: usize, perp_market: &PerpMarket) -> Result<I80F48> {
        let oracle = &self.ais[account_index];
        let oracle_acc_infos = OracleAccountInfos::from_reader(oracle);
        perp_market.oracle_price(&oracle_acc_infos, self.staleness_slot)
    }

    #[inline(always)]
    fn create_oracle_infos(
        &self,
        oracle_index: usize,
        fallback_key: &Pubkey,
    ) -> OracleAccountInfos<T> {
        let oracle = &self.ais[oracle_index];
        let fallback_opt = self.ais[self.begin_fallback_oracles..]
            .iter()
            .find(|ai| ai.key() == fallback_key);

        OracleAccountInfos {
            oracle,
            fallback_opt,
            usdc_opt: self.usdc_oracle_index.map(|i| &self.ais[i]),
            sol_opt: self.sol_oracle_index.map(|i| &self.ais[i]),
        }
    }
}

impl<T: KeyedAccountReader> AccountRetriever for FixedOrderAccountRetriever<T> {
    fn bank_and_oracle(
        &self,
        group: &Pubkey,
        active_token_position_index: usize,
        token_index: TokenIndex,
    ) -> Result<(&Bank, I80F48)> {
        let bank_account_index = active_token_position_index;
        let bank = self
            .bank(group, bank_account_index, token_index)
            .with_context(|| {
                format!(
                    "loading bank with health account index {}, token index {}, passed account {}",
                    bank_account_index,
                    token_index,
                    self.ais[bank_account_index].key(),
                )
            })?;

        let oracle_index = self.n_banks + active_token_position_index;
        let oracle_acc_infos = &self.create_oracle_infos(oracle_index, &bank.fallback_oracle);
        let oracle_price_result = bank.oracle_price(oracle_acc_infos, self.staleness_slot);
        let oracle_price = oracle_price_result.with_context(|| {
            format!(
                "getting oracle for bank with health account index {} and token index {}, passed account {}",
                bank_account_index,
                token_index,
                self.ais[oracle_index].key(),
            )
        })?;

        Ok((bank, oracle_price))
    }

    fn perp_market_and_oracle_price(
        &self,
        group: &Pubkey,
        active_perp_position_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<(&PerpMarket, I80F48)> {
        let perp_index = self.begin_perp + active_perp_position_index;
        let perp_market = self
            .perp_market(group, perp_index, perp_market_index)
            .with_context(|| {
                format!(
                    "loading perp market with health account index {} and perp market index {}, passed account {}",
                    perp_index,
                    perp_market_index,
                    self.ais[perp_index].key(),
                )
            })?;

        let oracle_index = perp_index + self.n_perps;
        let oracle_price = self.oracle_price_perp(oracle_index, perp_market).with_context(|| {
            format!(
                "getting oracle for perp market with health account index {} and perp market index {}, passed account {}",
                oracle_index,
                perp_market_index,
                self.ais[oracle_index].key(),
            )
        })?;
        Ok((perp_market, oracle_price))
    }

    fn serum_oo(&self, active_serum_oo_index: usize, key: &Pubkey) -> Result<&OpenOrders> {
        let serum_oo_index = self.begin_serum3 + active_serum_oo_index;
        let ai = &self.ais[serum_oo_index];
        (|| {
            require_keys_eq!(*key, *ai.key());
            serum3_cpi::load_open_orders(ai)
        })()
        .with_context(|| {
            format!(
                "loading serum open orders with health account index {}, passed account {}",
                serum_oo_index,
                ai.key(),
            )
        })
    }
}

pub struct ScannedBanksAndOracles<'a, 'info> {
    banks: Vec<AccountInfoRefMut<'a, 'info>>,
    oracles: Vec<AccountInfoRef<'a, 'info>>,
    fallback_oracles: Vec<AccountInfoRef<'a, 'info>>,
    index_map: HashMap<TokenIndex, usize>,
    staleness_slot: Option<u64>,
    /// index in fallback_oracles
    usd_oracle_index: Option<usize>,
    /// index in fallback_oracles
    sol_oracle_index: Option<usize>,
}

impl<'a, 'info> ScannedBanksAndOracles<'a, 'info> {
    #[inline]
    fn bank_index(&self, token_index: TokenIndex) -> Result<usize> {
        Ok(*self.index_map.get(&token_index).ok_or_else(|| {
            error_msg_typed!(
                MangoError::TokenPositionDoesNotExist,
                "token index {} not found",
                token_index
            )
        })?)
    }

    #[allow(clippy::type_complexity)]
    pub fn banks_mut_and_oracles(
        &mut self,
        token_index1: TokenIndex,
        token_index2: TokenIndex,
    ) -> Result<(&mut Bank, I80F48, Option<(&mut Bank, I80F48)>)> {
        if token_index1 == token_index2 {
            let index = self.bank_index(token_index1)?;
            let price = {
                let bank = self.banks[index].load_fully_unchecked::<Bank>()?;
                let oracle_acc_infos = self.create_oracle_infos(index, &bank.fallback_oracle);
                bank.oracle_price(&oracle_acc_infos, self.staleness_slot)?
            };

            let bank = self.banks[index].load_mut_fully_unchecked::<Bank>()?;
            return Ok((bank, price, None));
        }
        let index1 = self.bank_index(token_index1)?;
        let index2 = self.bank_index(token_index2)?;
        let (first, second, swap) = if index1 < index2 {
            (index1, index2, false)
        } else {
            (index2, index1, true)
        };

        let (price1, price2) = {
            let bank1 = self.banks[first].load_fully_unchecked::<Bank>()?;
            let bank2 = self.banks[second].load_fully_unchecked::<Bank>()?;
            let oracle_infos_1 = self.create_oracle_infos(first, &bank1.fallback_oracle);
            let oracle_infos_2 = self.create_oracle_infos(second, &bank2.fallback_oracle);
            let price1 = bank1.oracle_price(&oracle_infos_1, self.staleness_slot)?;
            let price2 = bank2.oracle_price(&oracle_infos_2, self.staleness_slot)?;
            (price1, price2)
        };

        // split_at_mut after the first bank and after the second bank
        let (first_bank_part, second_bank_part) = self.banks.split_at_mut(first + 1);

        let bank1 = first_bank_part[first].load_mut_fully_unchecked::<Bank>()?;
        let bank2 = second_bank_part[second - (first + 1)].load_mut_fully_unchecked::<Bank>()?;
        if swap {
            Ok((bank2, price2, Some((bank1, price1))))
        } else {
            Ok((bank1, price1, Some((bank2, price2))))
        }
    }

    pub fn scanned_bank_and_oracle(&self, token_index: TokenIndex) -> Result<(&Bank, I80F48)> {
        let index = self.bank_index(token_index)?;
        // The account was already loaded successfully during construction
        let bank = self.banks[index].load_fully_unchecked::<Bank>()?;
        let oracle_acc_infos = self.create_oracle_infos(index, &bank.fallback_oracle);
        let price = bank.oracle_price(&oracle_acc_infos, self.staleness_slot)?;

        Ok((bank, price))
    }

    #[inline(always)]
    fn create_oracle_infos(
        &self,
        oracle_index: usize,
        fallback_key: &Pubkey,
    ) -> OracleAccountInfos<AccountInfoRef> {
        let oracle = &self.oracles[oracle_index];
        let fallback_opt = if fallback_key == &Pubkey::default() {
            None
        } else {
            self.fallback_oracles
                .iter()
                .find(|ai| ai.key == fallback_key)
        };
        OracleAccountInfos {
            oracle,
            fallback_opt,
            usdc_opt: self.usd_oracle_index.map(|i| &self.fallback_oracles[i]),
            sol_opt: self.sol_oracle_index.map(|i| &self.fallback_oracles[i]),
        }
    }
}

/// Takes a list of account infos containing
/// - an unknown number of Banks in any order, followed by
/// - the same number of oracles in the same order as the banks, followed by
/// - an unknown number of PerpMarket accounts
/// - the same number of oracles in the same order as the perp markets
/// - an unknown number of serum3 OpenOrders accounts
/// - an unknown number of fallback oracle accounts
/// and retrieves accounts needed for the health computation by doing a linear
/// scan for each request.
pub struct ScanningAccountRetriever<'a, 'info> {
    banks_and_oracles: ScannedBanksAndOracles<'a, 'info>,
    perp_markets: Vec<AccountInfoRef<'a, 'info>>,
    perp_oracles: Vec<AccountInfoRef<'a, 'info>>,
    serum3_oos: Vec<AccountInfoRef<'a, 'info>>,
    perp_index_map: HashMap<PerpMarketIndex, usize>,
}

/// Returns None if `ai` doesn't have the owner or discriminator for T.
/// Forwards "can't borrow" error, so it can be raised immediately.
fn can_load_as<'a, T: ZeroCopy + Owner>(
    (i, ai): (usize, &'a AccountInfo),
) -> Option<(usize, Result<Ref<'a, T>>)> {
    let load_result = ai.load::<T>();
    if load_result.is_anchor_error_with_code(ErrorCode::AccountDiscriminatorMismatch.into())
        || load_result.is_anchor_error_with_code(ErrorCode::AccountDiscriminatorNotFound.into())
        || load_result.is_anchor_error_with_code(ErrorCode::AccountOwnedByWrongProgram.into())
    {
        return None;
    }
    Some((i, load_result))
}

impl<'a, 'info> ScanningAccountRetriever<'a, 'info> {
    pub fn new(ais: &'a [AccountInfo<'info>], group: &Pubkey) -> Result<Self> {
        Self::new_with_staleness(ais, group, Some(Clock::get()?.slot))
    }

    pub fn new_with_staleness(
        ais: &'a [AccountInfo<'info>],
        group: &Pubkey,
        staleness_slot: Option<u64>,
    ) -> Result<Self> {
        // find all Bank accounts
        let mut token_index_map = HashMap::with_capacity(ais.len() / 2);
        ais.iter()
            .enumerate()
            .map_while(can_load_as::<Bank>)
            .try_for_each(|(i, loaded)| {
                (|| {
                    let bank = loaded?;
                    require_keys_eq!(bank.group, *group);
                    let previous = token_index_map.insert(bank.token_index, i);
                    require_msg!(
                        previous.is_none(),
                        "duplicate bank for token index {}",
                        bank.token_index
                    );
                    Ok(())
                })()
                .with_context(|| format!("scanning banks, health account index {}", i))
            })?;
        let n_banks = token_index_map.len();

        // skip all banks and oracles, then find number of PerpMarket accounts
        let perps_start = n_banks * 2;
        let mut perp_index_map = HashMap::with_capacity(ais.len().saturating_sub(perps_start));
        ais[perps_start..]
            .iter()
            .enumerate()
            .map_while(can_load_as::<PerpMarket>)
            .try_for_each(|(i, loaded)| {
                (|| {
                    let perp_market = loaded?;
                    require_keys_eq!(perp_market.group, *group);
                    let previous = perp_index_map.insert(perp_market.perp_market_index, i);
                    require_msg!(
                        previous.is_none(),
                        "duplicate perp market for perp market index {}",
                        perp_market.perp_market_index
                    );
                    Ok(())
                })()
                .with_context(|| {
                    format!(
                        "scanning perp markets, health account index {}",
                        i + perps_start
                    )
                })
            })?;
        let n_perps = perp_index_map.len();
        let perp_oracles_start = perps_start + n_perps;
        let serum3_start = perp_oracles_start + n_perps;
        let n_serum3 = ais[serum3_start..]
            .iter()
            .take_while(|x| {
                x.data_len() == std::mem::size_of::<serum_dex::state::OpenOrders>() + 12
                    && serum3_cpi::has_serum_header(&x.data.borrow())
            })
            .count();
        let fallback_oracles_start = serum3_start + n_serum3;
        let usd_oracle_index = ais[fallback_oracles_start..]
            .iter()
            .position(|o| o.key == &pyth_mainnet_usdc_oracle::ID);
        let sol_oracle_index = ais[fallback_oracles_start..]
            .iter()
            .position(|o| o.key == &pyth_mainnet_sol_oracle::ID);

        Ok(Self {
            banks_and_oracles: ScannedBanksAndOracles {
                banks: AccountInfoRefMut::borrow_slice(&ais[..n_banks])?,
                oracles: AccountInfoRef::borrow_slice(&ais[n_banks..perps_start])?,
                fallback_oracles: AccountInfoRef::borrow_slice(&ais[fallback_oracles_start..])?,
                index_map: token_index_map,
                staleness_slot,
                usd_oracle_index,
                sol_oracle_index,
            },
            perp_markets: AccountInfoRef::borrow_slice(&ais[perps_start..perp_oracles_start])?,
            perp_oracles: AccountInfoRef::borrow_slice(&ais[perp_oracles_start..serum3_start])?,
            serum3_oos: AccountInfoRef::borrow_slice(&ais[serum3_start..fallback_oracles_start])?,
            perp_index_map,
        })
    }

    #[inline]
    fn perp_market_index(&self, perp_market_index: PerpMarketIndex) -> Result<usize> {
        Ok(*self
            .perp_index_map
            .get(&perp_market_index)
            .ok_or_else(|| error_msg!("perp market index {} not found", perp_market_index))?)
    }

    #[allow(clippy::type_complexity)]
    pub fn banks_mut_and_oracles(
        &mut self,
        token_index1: TokenIndex,
        token_index2: TokenIndex,
    ) -> Result<(&mut Bank, I80F48, Option<(&mut Bank, I80F48)>)> {
        self.banks_and_oracles
            .banks_mut_and_oracles(token_index1, token_index2)
    }

    pub fn scanned_bank_and_oracle(&self, token_index: TokenIndex) -> Result<(&Bank, I80F48)> {
        self.banks_and_oracles.scanned_bank_and_oracle(token_index)
    }

    pub fn scanned_perp_market_and_oracle(
        &self,
        perp_market_index: PerpMarketIndex,
    ) -> Result<(&PerpMarket, I80F48)> {
        let index = self.perp_market_index(perp_market_index)?;
        // The account was already loaded successfully during construction
        let perp_market = self.perp_markets[index].load_fully_unchecked::<PerpMarket>()?;
        let oracle_acc = &self.perp_oracles[index];
        let oracle_acc_infos = OracleAccountInfos::from_reader(oracle_acc);
        let price =
            perp_market.oracle_price(&oracle_acc_infos, self.banks_and_oracles.staleness_slot)?;
        Ok((perp_market, price))
    }

    pub fn scanned_serum_oo(&self, key: &Pubkey) -> Result<&OpenOrders> {
        let oo = self
            .serum3_oos
            .iter()
            .find(|ai| ai.key == key)
            .ok_or_else(|| error_msg!("no serum3 open orders for key {}", key))?;
        serum3_cpi::load_open_orders(oo)
    }

    pub fn into_banks_and_oracles(self) -> ScannedBanksAndOracles<'a, 'info> {
        self.banks_and_oracles
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

    fn perp_market_and_oracle_price(
        &self,
        _group: &Pubkey,
        _account_index: usize,
        perp_market_index: PerpMarketIndex,
    ) -> Result<(&PerpMarket, I80F48)> {
        self.scanned_perp_market_and_oracle(perp_market_index)
    }

    fn serum_oo(&self, _account_index: usize, key: &Pubkey) -> Result<&OpenOrders> {
        self.scanned_serum_oo(key)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test::*;
    use super::*;
    use serum_dex::state::OpenOrders;
    use std::convert::identity;

    #[test]
    fn test_scanning_account_retriever() {
        let oracle1_price = 1.0;
        let oracle2_price = 5.0;
        let group = Pubkey::new_unique();

        let (mut bank1, mut oracle1) = mock_bank_and_oracle(group, 1, oracle1_price, 0.2, 0.1);
        let (mut bank2, mut oracle2) = mock_bank_and_oracle(group, 4, oracle2_price, 0.5, 0.3);
        let (mut bank3, _) = mock_bank_and_oracle(group, 5, 1.0, 0.5, 0.3);

        // bank3 reuses the bank2 oracle, to ensure the ScanningAccountRetriever doesn't choke on that
        bank3.data().oracle = oracle2.pubkey;

        let mut oo1 = TestAccount::<OpenOrders>::new_zeroed();
        let oo1key = oo1.pubkey;
        oo1.data().native_pc_total = 20;

        let mut perp1 = mock_perp_market(
            group,
            oracle2.pubkey,
            oracle2_price,
            9,
            (0.2, 0.1),
            (0.05, 0.02),
        );
        let mut perp2 = mock_perp_market(
            group,
            oracle1.pubkey,
            oracle1_price,
            8,
            (0.2, 0.1),
            (0.05, 0.02),
        );

        let oracle1_account_info = oracle1.as_account_info();
        let oracle2_account_info = oracle2.as_account_info();
        let ais = vec![
            bank1.as_account_info(),
            bank2.as_account_info(),
            bank3.as_account_info(),
            oracle1_account_info.clone(),
            oracle2_account_info.clone(),
            oracle2_account_info.clone(),
            perp1.as_account_info(),
            perp2.as_account_info(),
            oracle2_account_info,
            oracle1_account_info,
            oo1.as_account_info(),
        ];

        let mut retriever =
            ScanningAccountRetriever::new_with_staleness(&ais, &group, None).unwrap();

        assert_eq!(retriever.banks_and_oracles.banks.len(), 3);
        assert_eq!(retriever.banks_and_oracles.index_map.len(), 3);
        assert_eq!(retriever.banks_and_oracles.oracles.len(), 3);
        assert_eq!(retriever.perp_markets.len(), 2);
        assert_eq!(retriever.perp_oracles.len(), 2);
        assert_eq!(retriever.perp_index_map.len(), 2);
        assert_eq!(retriever.serum3_oos.len(), 1);

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

        {
            let (b, o) = retriever.scanned_bank_and_oracle(5).unwrap();
            assert_eq!(b.token_index, 5);
            assert_eq!(o, 5 * I80F48::ONE);
        }

        let oo = retriever.serum_oo(0, &oo1key).unwrap();
        assert_eq!(identity(oo.native_pc_total), 20);

        assert!(retriever.serum_oo(1, &Pubkey::default()).is_err());

        let (perp, oracle_price) = retriever
            .perp_market_and_oracle_price(&group, 0, 9)
            .unwrap();
        assert_eq!(identity(perp.perp_market_index), 9);
        assert_eq!(oracle_price, oracle2_price);

        let (perp, oracle_price) = retriever
            .perp_market_and_oracle_price(&group, 1, 8)
            .unwrap();
        assert_eq!(identity(perp.perp_market_index), 8);
        assert_eq!(oracle_price, oracle1_price);

        assert!(retriever
            .perp_market_and_oracle_price(&group, 1, 5)
            .is_err());
    }
}
