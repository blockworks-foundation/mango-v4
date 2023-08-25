use std::cell::{Ref, RefMut};
use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use arrayref::array_ref;

use fixed::types::I80F48;

use solana_program::program_memory::sol_memmove;
use static_assertions::const_assert_eq;

use crate::error::*;
use crate::health::{HealthCache, HealthType};
use crate::logs::{DeactivatePerpPositionLog, DeactivateTokenPositionLog};

use super::BookSideOrderTree;
use super::FillEvent;
use super::LeafNode;
use super::PerpMarket;
use super::PerpMarketIndex;
use super::PerpOpenOrder;
use super::Serum3MarketIndex;
use super::TokenConditionalSwap;
use super::TokenIndex;
use super::FREE_ORDER_SLOT;
use super::{dynamic_account::*, Group};
use super::{PerpPosition, Serum3Orders, TokenPosition};
use super::{Side, SideAndOrderTree};

type BorshVecLength = u32;
const BORSH_VEC_PADDING_BYTES: usize = 4;
const BORSH_VEC_SIZE_BYTES: usize = 4;
const DEFAULT_MANGO_ACCOUNT_VERSION: u8 = 1;

// Return variants for check_liquidatable method, should be wrapped in a Result
// for a future possiblity of returning any error
#[derive(PartialEq)]
pub enum CheckLiquidatable {
    NotLiquidatable,
    Liquidatable,
    BecameNotLiquidatable,
}

// Mango Account
// This struct definition is only for clients e.g. typescript, so that they can easily use out of the box
// deserialization and not have to do custom deserialization
// On chain, we would prefer zero-copying to optimize for compute
#[account]
pub struct MangoAccount {
    // fixed
    // note: keep MangoAccountFixed in sync with changes here
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,

    // ABI: Clients rely on this being at offset 40
    pub owner: Pubkey,

    pub name: [u8; 32],

    // Alternative authority/signer of transactions for a mango account
    pub delegate: Pubkey,

    pub account_num: u32,

    /// Tracks that this account should be liquidated until init_health >= 0.
    ///
    /// Normally accounts can not be liquidated while maint_health >= 0. But when an account
    /// reaches maint_health < 0, liquidators will call a liquidation instruction and thereby
    /// set this flag. Now the account may be liquidated until init_health >= 0.
    ///
    /// Many actions should be disabled while the account is being liquidated, even if
    /// its maint health has recovered to positive. Creating new open orders would, for example,
    /// confuse liquidators.
    pub being_liquidated: u8,

    /// The account is currently inside a health region marked by HealthRegionBegin...HealthRegionEnd.
    ///
    /// Must never be set after a transaction ends.
    pub in_health_region: u8,

    pub bump: u8,

    pub padding: [u8; 1],

    // (Display only)
    // Cumulative (deposits - withdraws)
    // using USD prices at the time of the deposit/withdraw
    // in USD units with 6 decimals
    pub net_deposits: i64,
    // (Display only)
    // Cumulative transfers from perp to spot positions
    pub perp_spot_transfers: i64,

    /// Init health as calculated during HealthReginBegin, rounded up.
    pub health_region_begin_init_health: i64,

    pub frozen_until: u64,

    /// Fees usable with the "fees buyback" feature.
    /// This tracks the ones that accrued in the current expiry interval.
    pub buyback_fees_accrued_current: u64,
    /// Fees buyback amount from the previous expiry interval.
    pub buyback_fees_accrued_previous: u64,
    /// End timestamp of the current expiry interval of the buyback fees amount.
    pub buyback_fees_expiry_timestamp: u64,

    /// Next id to use when adding a token condition swap
    pub next_token_conditional_swap_id: u64,

    pub reserved: [u8; 200],

    // dynamic
    pub header_version: u8,
    pub padding3: [u8; 7],
    // note: padding is required for TokenPosition, etc. to be aligned
    pub padding4: u32,
    // Maps token_index -> deposit/borrow account for each token
    // that is active on this MangoAccount.
    pub tokens: Vec<TokenPosition>,
    pub padding5: u32,
    // Maps serum_market_index -> open orders for each serum market
    // that is active on this MangoAccount.
    pub serum3: Vec<Serum3Orders>,
    pub padding6: u32,
    pub perps: Vec<PerpPosition>,
    pub padding7: u32,
    pub perp_open_orders: Vec<PerpOpenOrder>,
    // WARNING: This does not have further fields, like tcs, intentionally:
    // There are existing accounts that don't have them and adding them here
    // would break backwards compatibility.
}

impl MangoAccount {
    pub fn default_for_tests() -> Self {
        Self {
            name: Default::default(),
            group: Pubkey::default(),
            owner: Pubkey::default(),
            delegate: Pubkey::default(),
            being_liquidated: 0,
            in_health_region: 0,
            account_num: 0,
            bump: 0,
            padding: Default::default(),
            net_deposits: 0,
            health_region_begin_init_health: 0,
            frozen_until: 0,
            buyback_fees_accrued_current: 0,
            buyback_fees_accrued_previous: 0,
            buyback_fees_expiry_timestamp: 0,
            next_token_conditional_swap_id: 0,
            reserved: [0; 200],
            header_version: DEFAULT_MANGO_ACCOUNT_VERSION,
            padding3: Default::default(),
            padding4: Default::default(),
            tokens: vec![TokenPosition::default(); 3],
            padding5: Default::default(),
            serum3: vec![Serum3Orders::default(); 5],
            padding6: Default::default(),
            perps: vec![PerpPosition::default(); 4],
            padding7: Default::default(),
            perp_open_orders: vec![PerpOpenOrder::default(); 6],
            perp_spot_transfers: 0,
        }
    }

    /// Number of bytes needed for the MangoAccount, including the discriminator
    pub fn space(
        token_count: u8,
        serum3_count: u8,
        perp_count: u8,
        perp_oo_count: u8,
        token_conditional_swap_count: u8,
    ) -> usize {
        8 + size_of::<MangoAccountFixed>()
            + Self::dynamic_size(
                token_count,
                serum3_count,
                perp_count,
                perp_oo_count,
                token_conditional_swap_count,
            )
    }

    pub fn dynamic_token_vec_offset() -> usize {
        8 // header version + padding
            + BORSH_VEC_PADDING_BYTES
    }

    pub fn dynamic_serum3_vec_offset(token_count: u8) -> usize {
        Self::dynamic_token_vec_offset()
            + (BORSH_VEC_SIZE_BYTES + size_of::<TokenPosition>() * usize::from(token_count))
            + BORSH_VEC_PADDING_BYTES
    }

    pub fn dynamic_perp_vec_offset(token_count: u8, serum3_count: u8) -> usize {
        Self::dynamic_serum3_vec_offset(token_count)
            + (BORSH_VEC_SIZE_BYTES + size_of::<Serum3Orders>() * usize::from(serum3_count))
            + BORSH_VEC_PADDING_BYTES
    }

    pub fn dynamic_perp_oo_vec_offset(token_count: u8, serum3_count: u8, perp_count: u8) -> usize {
        Self::dynamic_perp_vec_offset(token_count, serum3_count)
            + (BORSH_VEC_SIZE_BYTES + size_of::<PerpPosition>() * usize::from(perp_count))
            + BORSH_VEC_PADDING_BYTES
    }

    pub fn dynamic_token_conditional_swap_vec_offset(
        token_count: u8,
        serum3_count: u8,
        perp_count: u8,
        perp_oo_count: u8,
    ) -> usize {
        Self::dynamic_perp_oo_vec_offset(token_count, serum3_count, perp_count)
            + (BORSH_VEC_SIZE_BYTES + size_of::<PerpOpenOrder>() * usize::from(perp_oo_count))
            + BORSH_VEC_PADDING_BYTES
    }

    pub fn dynamic_size(
        token_count: u8,
        serum3_count: u8,
        perp_count: u8,
        perp_oo_count: u8,
        token_conditional_swap_count: u8,
    ) -> usize {
        Self::dynamic_token_conditional_swap_vec_offset(
            token_count,
            serum3_count,
            perp_count,
            perp_oo_count,
        ) + (BORSH_VEC_SIZE_BYTES
            + size_of::<TokenConditionalSwap>() * usize::from(token_conditional_swap_count))
    }
}

// Mango Account fixed part for easy zero copy deserialization
#[zero_copy]
pub struct MangoAccountFixed {
    pub group: Pubkey,
    pub owner: Pubkey,
    pub name: [u8; 32],
    pub delegate: Pubkey,
    pub account_num: u32,
    being_liquidated: u8,
    in_health_region: u8,
    pub bump: u8,
    pub padding: [u8; 1],
    pub net_deposits: i64,
    pub perp_spot_transfers: i64,
    pub health_region_begin_init_health: i64,
    pub frozen_until: u64,
    pub buyback_fees_accrued_current: u64,
    pub buyback_fees_accrued_previous: u64,
    pub buyback_fees_expiry_timestamp: u64,
    pub next_token_conditional_swap_id: u64,
    pub temporary_delegate: Pubkey,
    pub temporary_delegate_expiry: u64,
    pub reserved: [u8; 160],
}
const_assert_eq!(
    size_of::<MangoAccountFixed>(),
    32 * 4 + 8 + 8 * 8 + 32 + 8 + 160
);
const_assert_eq!(size_of::<MangoAccountFixed>(), 400);
const_assert_eq!(size_of::<MangoAccountFixed>() % 8, 0);

impl MangoAccountFixed {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn is_operational(&self) -> bool {
        let now_ts: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
        self.frozen_until < now_ts
    }

    pub fn is_owner_or_delegate(&self, ix_signer: Pubkey) -> bool {
        self.owner == ix_signer || self.is_delegate(ix_signer)
    }

    pub fn is_delegate(&self, ix_signer: Pubkey) -> bool {
        if self.delegate == ix_signer {
            return true;
        }

        let now_ts: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
        if now_ts > self.temporary_delegate_expiry {
            return false;
        }

        self.temporary_delegate == ix_signer
    }

    pub fn being_liquidated(&self) -> bool {
        self.being_liquidated == 1
    }

    pub fn set_being_liquidated(&mut self, b: bool) {
        self.being_liquidated = u8::from(b);
    }

    pub fn is_in_health_region(&self) -> bool {
        self.in_health_region == 1
    }

    pub fn set_in_health_region(&mut self, b: bool) {
        self.in_health_region = u8::from(b);
    }

    pub fn maybe_recover_from_being_liquidated(&mut self, liq_end_health: I80F48) -> bool {
        // This is used as threshold to flip flag instead of 0 because of dust issues
        let one_native_usdc = I80F48::ONE;
        if self.being_liquidated() && liq_end_health > -one_native_usdc {
            self.set_being_liquidated(false);
            true
        } else {
            false
        }
    }

    /// Updates the buyback_fees_* fields for staggered expiry of available amounts.
    pub fn expire_buyback_fees(&mut self, now_ts: u64, interval: u64) {
        if interval == 0 || now_ts < self.buyback_fees_expiry_timestamp {
            return;
        } else if now_ts < self.buyback_fees_expiry_timestamp + interval {
            self.buyback_fees_accrued_previous = self.buyback_fees_accrued_current;
        } else {
            self.buyback_fees_accrued_previous = 0;
        }
        self.buyback_fees_accrued_current = 0;
        self.buyback_fees_expiry_timestamp = (now_ts / interval + 1) * interval;
    }

    /// The total buyback fees amount that the account can make use of.
    pub fn buyback_fees_accrued(&self) -> u64 {
        self.buyback_fees_accrued_current
            .saturating_add(self.buyback_fees_accrued_previous)
    }

    /// Add new fees that are usable with the buyback fees feature.
    ///
    /// Any call to this should be preceeded by a call to expire_buyback_fees earlier
    /// in the same instruction.
    pub fn accrue_buyback_fees(&mut self, amount: u64) {
        self.buyback_fees_accrued_current =
            self.buyback_fees_accrued_current.saturating_add(amount);
    }

    /// Reduce the available buyback fees amount because it was used up.
    ///
    /// Panics if `amount` exceeds the available accrued amount
    pub fn reduce_buyback_fees_accrued(&mut self, amount: u64) {
        if amount > self.buyback_fees_accrued_previous {
            let remaining_amount = amount - self.buyback_fees_accrued_previous;
            assert!(remaining_amount <= self.buyback_fees_accrued_current);
            self.buyback_fees_accrued_current -= remaining_amount;
            self.buyback_fees_accrued_previous = 0;
        } else {
            self.buyback_fees_accrued_previous -= amount;
        }
    }
}

impl Owner for MangoAccountFixed {
    fn owner() -> Pubkey {
        MangoAccount::owner()
    }
}

impl Discriminator for MangoAccountFixed {
    const DISCRIMINATOR: [u8; 8] = MangoAccount::DISCRIMINATOR;
}

impl anchor_lang::ZeroCopy for MangoAccountFixed {}

#[derive(Clone)]
pub struct MangoAccountDynamicHeader {
    pub token_count: u8,
    pub serum3_count: u8,
    pub perp_count: u8,
    pub perp_oo_count: u8,
    pub token_conditional_swap_count: u8,
}

impl DynamicHeader for MangoAccountDynamicHeader {
    fn from_bytes(dynamic_data: &[u8]) -> Result<Self> {
        let header_version = u8::from_le_bytes(*array_ref![dynamic_data, 0, size_of::<u8>()]);

        match header_version {
            1 => {
                let token_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
                    dynamic_data,
                    MangoAccount::dynamic_token_vec_offset(),
                    BORSH_VEC_SIZE_BYTES
                ]))
                .unwrap();

                let serum3_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
                    dynamic_data,
                    MangoAccount::dynamic_serum3_vec_offset(token_count),
                    BORSH_VEC_SIZE_BYTES
                ]))
                .unwrap();

                let perp_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
                    dynamic_data,
                    MangoAccount::dynamic_perp_vec_offset(token_count, serum3_count),
                    BORSH_VEC_SIZE_BYTES
                ]))
                .unwrap();

                let perp_oo_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
                    dynamic_data,
                    MangoAccount::dynamic_perp_oo_vec_offset(token_count, serum3_count, perp_count),
                    BORSH_VEC_SIZE_BYTES
                ]))
                .unwrap();

                let token_conditional_swap_vec_offset =
                    MangoAccount::dynamic_token_conditional_swap_vec_offset(
                        token_count,
                        serum3_count,
                        perp_count,
                        perp_oo_count,
                    );
                let token_conditional_swap_count = if dynamic_data.len()
                    > token_conditional_swap_vec_offset + BORSH_VEC_SIZE_BYTES
                {
                    u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
                        dynamic_data,
                        token_conditional_swap_vec_offset,
                        BORSH_VEC_SIZE_BYTES
                    ]))
                    .unwrap()
                } else {
                    0
                };

                Ok(Self {
                    token_count,
                    serum3_count,
                    perp_count,
                    perp_oo_count,
                    token_conditional_swap_count,
                })
            }
            _ => err!(MangoError::NotImplementedError).context("unexpected header version number"),
        }
    }

    fn initialize(dynamic_data: &mut [u8]) -> Result<()> {
        let dst: &mut [u8] = &mut dynamic_data[0..1];
        dst.copy_from_slice(&DEFAULT_MANGO_ACCOUNT_VERSION.to_le_bytes());
        Ok(())
    }
}

fn get_helper<T: bytemuck::Pod>(data: &[u8], index: usize) -> &T {
    bytemuck::from_bytes(&data[index..index + size_of::<T>()])
}

fn get_helper_mut<T: bytemuck::Pod>(data: &mut [u8], index: usize) -> &mut T {
    bytemuck::from_bytes_mut(&mut data[index..index + size_of::<T>()])
}

impl MangoAccountDynamicHeader {
    // offset into dynamic data where 1st TokenPosition would be found
    // todo make fn private
    pub fn token_offset(&self, raw_index: usize) -> usize {
        MangoAccount::dynamic_token_vec_offset()
            + BORSH_VEC_SIZE_BYTES
            + raw_index * size_of::<TokenPosition>()
    }

    // offset into dynamic data where 1st Serum3Orders would be found
    // todo make fn private
    pub fn serum3_offset(&self, raw_index: usize) -> usize {
        MangoAccount::dynamic_serum3_vec_offset(self.token_count)
            + BORSH_VEC_SIZE_BYTES
            + raw_index * size_of::<Serum3Orders>()
    }

    // offset into dynamic data where 1st PerpPosition would be found
    fn perp_offset(&self, raw_index: usize) -> usize {
        MangoAccount::dynamic_perp_vec_offset(self.token_count, self.serum3_count)
            + BORSH_VEC_SIZE_BYTES
            + raw_index * size_of::<PerpPosition>()
    }

    fn perp_oo_offset(&self, raw_index: usize) -> usize {
        MangoAccount::dynamic_perp_oo_vec_offset(
            self.token_count,
            self.serum3_count,
            self.perp_count,
        ) + BORSH_VEC_SIZE_BYTES
            + raw_index * size_of::<PerpOpenOrder>()
    }

    fn token_conditional_swap_offset(&self, raw_index: usize) -> usize {
        MangoAccount::dynamic_token_conditional_swap_vec_offset(
            self.token_count,
            self.serum3_count,
            self.perp_count,
            self.perp_oo_count,
        ) + BORSH_VEC_SIZE_BYTES
            + raw_index * size_of::<TokenConditionalSwap>()
    }

    pub fn token_count(&self) -> usize {
        self.token_count.into()
    }
    pub fn serum3_count(&self) -> usize {
        self.serum3_count.into()
    }
    pub fn perp_count(&self) -> usize {
        self.perp_count.into()
    }
    pub fn perp_oo_count(&self) -> usize {
        self.perp_oo_count.into()
    }
    pub fn token_conditional_swap_count(&self) -> usize {
        self.token_conditional_swap_count.into()
    }

    pub fn zero() -> Self {
        Self {
            token_count: 0,
            serum3_count: 0,
            perp_count: 0,
            perp_oo_count: 0,
            token_conditional_swap_count: 0,
        }
    }

    fn expected_health_accounts(&self) -> usize {
        self.token_count() * 2 + self.serum3_count() + self.perp_count() * 2
    }

    /// Error if this header isn't a valid resize from `prev`
    ///
    /// - Check that dynamic fields can only increase in size
    /// - Check that if something increases, it is bounded by the limits
    /// - If a field doesn't change, don't error if it exceeds the limits
    ///   (might have been expanded earlier when it was valid to do)
    /// - Check that the total health accounts stay limited
    pub fn check_resize_from(&self, prev: &Self) -> Result<()> {
        require_gte!(self.token_count, prev.token_count);
        if self.token_count > prev.token_count {
            require_gte!(8, self.token_count);
        }

        require_gte!(self.serum3_count, prev.serum3_count);
        if self.serum3_count > prev.serum3_count {
            require_gte!(4, self.serum3_count);
        }

        require_gte!(self.perp_count, prev.perp_count);
        if self.perp_count > prev.perp_count {
            require_gte!(4, self.perp_count);
        }

        require_gte!(self.perp_oo_count, prev.perp_oo_count);
        if self.perp_oo_count > prev.perp_oo_count {
            require_gte!(64, self.perp_oo_count);
        }

        require_gte!(
            self.token_conditional_swap_count,
            prev.token_conditional_swap_count
        );
        if self.token_conditional_swap_count > prev.token_conditional_swap_count {
            require_gte!(64, self.token_conditional_swap_count);
        }

        let new_health_accounts = self.expected_health_accounts();
        let prev_health_accounts = prev.expected_health_accounts();
        if new_health_accounts > prev_health_accounts {
            require_gte!(28, new_health_accounts);
        }

        Ok(())
    }
}

/// Fully owned MangoAccount, useful for tests
pub type MangoAccountValue = DynamicAccount<MangoAccountDynamicHeader, MangoAccountFixed, Vec<u8>>;

/// Full reference type, useful for borrows
pub type MangoAccountRef<'a> =
    DynamicAccount<&'a MangoAccountDynamicHeader, &'a MangoAccountFixed, &'a [u8]>;
/// Full reference type, useful for borrows
pub type MangoAccountRefMut<'a> =
    DynamicAccount<&'a mut MangoAccountDynamicHeader, &'a mut MangoAccountFixed, &'a mut [u8]>;

/// Useful when loading from bytes
pub type MangoAccountLoadedRef<'a> =
    DynamicAccount<MangoAccountDynamicHeader, &'a MangoAccountFixed, &'a [u8]>;
/// Useful when loading from RefCell, like from AccountInfo
pub type MangoAccountLoadedRefCell<'a> =
    DynamicAccount<MangoAccountDynamicHeader, Ref<'a, MangoAccountFixed>, Ref<'a, [u8]>>;
/// Useful when loading from RefCell, like from AccountInfo
pub type MangoAccountLoadedRefCellMut<'a> =
    DynamicAccount<MangoAccountDynamicHeader, RefMut<'a, MangoAccountFixed>, RefMut<'a, [u8]>>;

impl MangoAccountValue {
    // bytes without discriminator
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (fixed, dynamic) = bytes.split_at(size_of::<MangoAccountFixed>());
        Ok(Self {
            fixed: *bytemuck::from_bytes(fixed),
            header: MangoAccountDynamicHeader::from_bytes(dynamic)?,
            dynamic: dynamic.to_vec(),
        })
    }
}

impl<'a> MangoAccountLoadedRef<'a> {
    // bytes without discriminator
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        let (fixed, dynamic) = bytes.split_at(size_of::<MangoAccountFixed>());
        Ok(Self {
            fixed: bytemuck::from_bytes(fixed),
            header: MangoAccountDynamicHeader::from_bytes(dynamic)?,
            dynamic,
        })
    }
}

// This generic impl covers MangoAccountRef, MangoAccountRefMut and other
// DynamicAccount variants that allow read access.
impl<
        Header: DerefOrBorrow<MangoAccountDynamicHeader>,
        Fixed: DerefOrBorrow<MangoAccountFixed>,
        Dynamic: DerefOrBorrow<[u8]>,
    > DynamicAccount<Header, Fixed, Dynamic>
{
    fn header(&self) -> &MangoAccountDynamicHeader {
        self.header.deref_or_borrow()
    }

    pub fn header_version(&self) -> &u8 {
        get_helper(self.dynamic(), 0)
    }

    fn fixed(&self) -> &MangoAccountFixed {
        self.fixed.deref_or_borrow()
    }

    fn dynamic(&self) -> &[u8] {
        self.dynamic.deref_or_borrow()
    }

    /// Returns
    /// - the position
    /// - the raw index into the token positions list (for use with get_raw/deactivate)
    pub fn token_position_and_raw_index(
        &self,
        token_index: TokenIndex,
    ) -> Result<(&TokenPosition, usize)> {
        self.all_token_positions()
            .enumerate()
            .find_map(|(raw_index, p)| p.is_active_for_token(token_index).then_some((p, raw_index)))
            .ok_or_else(|| {
                error_msg_typed!(
                    MangoError::TokenPositionDoesNotExist,
                    "position for token index {} not found",
                    token_index
                )
            })
    }

    pub fn token_position(&self, token_index: TokenIndex) -> Result<&TokenPosition> {
        self.token_position_and_raw_index(token_index)
            .map(|(p, _)| p)
    }

    pub(crate) fn token_position_by_raw_index_unchecked(&self, raw_index: usize) -> &TokenPosition {
        get_helper(self.dynamic(), self.header().token_offset(raw_index))
    }

    pub fn token_position_by_raw_index(&self, raw_index: usize) -> Result<&TokenPosition> {
        require_gt!(self.header().token_count(), raw_index);
        Ok(self.token_position_by_raw_index_unchecked(raw_index))
    }

    // get iter over all TokenPositions (including inactive)
    pub fn all_token_positions(&self) -> impl Iterator<Item = &TokenPosition> + '_ {
        (0..self.header().token_count()).map(|i| self.token_position_by_raw_index_unchecked(i))
    }

    // get iter over all active TokenPositions
    pub fn active_token_positions(&self) -> impl Iterator<Item = &TokenPosition> + '_ {
        self.all_token_positions().filter(|token| token.is_active())
    }

    pub fn serum3_orders(&self, market_index: Serum3MarketIndex) -> Result<&Serum3Orders> {
        self.all_serum3_orders()
            .find(|p| p.is_active_for_market(market_index))
            .ok_or_else(|| error_msg!("serum3 orders for market index {} not found", market_index))
    }

    pub(crate) fn serum3_orders_by_raw_index_unchecked(&self, raw_index: usize) -> &Serum3Orders {
        get_helper(self.dynamic(), self.header().serum3_offset(raw_index))
    }

    pub fn serum3_orders_by_raw_index(&self, raw_index: usize) -> Result<&Serum3Orders> {
        require_gt!(self.header().serum3_count(), raw_index);
        Ok(self.serum3_orders_by_raw_index_unchecked(raw_index))
    }

    pub fn all_serum3_orders(&self) -> impl Iterator<Item = &Serum3Orders> + '_ {
        (0..self.header().serum3_count()).map(|i| self.serum3_orders_by_raw_index_unchecked(i))
    }

    pub fn active_serum3_orders(&self) -> impl Iterator<Item = &Serum3Orders> + '_ {
        self.all_serum3_orders()
            .filter(|serum3_order| serum3_order.is_active())
    }

    pub fn perp_position(&self, market_index: PerpMarketIndex) -> Result<&PerpPosition> {
        self.all_perp_positions()
            .find(|p| p.is_active_for_market(market_index))
            .ok_or_else(|| error!(MangoError::PerpPositionDoesNotExist))
    }

    pub(crate) fn perp_position_by_raw_index_unchecked(&self, raw_index: usize) -> &PerpPosition {
        get_helper(self.dynamic(), self.header().perp_offset(raw_index))
    }

    pub fn perp_position_by_raw_index(&self, raw_index: usize) -> Result<&PerpPosition> {
        require_gt!(self.header().perp_count(), raw_index);
        Ok(self.perp_position_by_raw_index_unchecked(raw_index))
    }

    pub fn all_perp_positions(&self) -> impl Iterator<Item = &PerpPosition> {
        (0..self.header().perp_count()).map(|i| self.perp_position_by_raw_index_unchecked(i))
    }

    pub fn active_perp_positions(&self) -> impl Iterator<Item = &PerpPosition> {
        self.all_perp_positions().filter(|p| p.is_active())
    }

    pub(crate) fn perp_order_by_raw_index_unchecked(&self, raw_index: usize) -> &PerpOpenOrder {
        get_helper(self.dynamic(), self.header().perp_oo_offset(raw_index))
    }

    pub fn perp_order_by_raw_index(&self, raw_index: usize) -> Result<&PerpOpenOrder> {
        require_gt!(self.header().perp_oo_count(), raw_index);
        Ok(self.perp_order_by_raw_index_unchecked(raw_index))
    }

    pub fn all_perp_orders(&self) -> impl Iterator<Item = &PerpOpenOrder> {
        (0..self.header().perp_oo_count()).map(|i| self.perp_order_by_raw_index_unchecked(i))
    }

    pub fn perp_next_order_slot(&self) -> Result<usize> {
        self.all_perp_orders()
            .position(|&oo| oo.market == FREE_ORDER_SLOT)
            .ok_or_else(|| error_msg!("no free perp order index"))
    }

    pub fn perp_find_order_with_client_order_id(
        &self,
        market_index: PerpMarketIndex,
        client_order_id: u64,
    ) -> Option<&PerpOpenOrder> {
        self.all_perp_orders()
            .find(|&oo| oo.is_active_for_market(market_index) && oo.client_id == client_order_id)
    }

    pub fn perp_find_order_with_order_id(
        &self,
        market_index: PerpMarketIndex,
        order_id: u128,
    ) -> Option<&PerpOpenOrder> {
        self.all_perp_orders()
            .find(|&oo| oo.is_active_for_market(market_index) && oo.id == order_id)
    }

    pub fn being_liquidated(&self) -> bool {
        self.fixed().being_liquidated()
    }

    fn token_conditional_swap_by_index_unchecked(&self, index: usize) -> &TokenConditionalSwap {
        get_helper(
            self.dynamic(),
            self.header().token_conditional_swap_offset(index),
        )
    }

    pub fn token_conditional_swap_by_index(&self, index: usize) -> Result<&TokenConditionalSwap> {
        require_gt!(self.header().token_conditional_swap_count(), index);
        Ok(self.token_conditional_swap_by_index_unchecked(index))
    }

    pub fn token_conditional_swap_by_id(&self, id: u64) -> Result<(usize, &TokenConditionalSwap)> {
        let index = self
            .all_token_conditional_swaps()
            .position(|tcs| tcs.has_data() && tcs.id == id)
            .ok_or_else(|| error_msg!("token conditional swap with id {} not found", id))?;
        Ok((index, self.token_conditional_swap_by_index_unchecked(index)))
    }

    pub fn all_token_conditional_swaps(&self) -> impl Iterator<Item = &TokenConditionalSwap> {
        (0..self.header().token_conditional_swap_count())
            .map(|i| self.token_conditional_swap_by_index_unchecked(i))
    }

    pub fn active_token_conditional_swaps(&self) -> impl Iterator<Item = &TokenConditionalSwap> {
        self.all_token_conditional_swaps().filter(|p| p.has_data())
    }

    pub fn token_conditional_swap_free_index(&self) -> Result<usize> {
        self.all_token_conditional_swaps()
            .position(|&v| !v.has_data())
            .ok_or_else(|| error_msg!("no free token conditional swap index"))
    }

    pub fn borrow(&self) -> MangoAccountRef {
        MangoAccountRef {
            header: self.header(),
            fixed: self.fixed(),
            dynamic: self.dynamic(),
        }
    }
}

impl<
        Header: DerefOrBorrowMut<MangoAccountDynamicHeader> + DerefOrBorrow<MangoAccountDynamicHeader>,
        Fixed: DerefOrBorrowMut<MangoAccountFixed> + DerefOrBorrow<MangoAccountFixed>,
        Dynamic: DerefOrBorrowMut<[u8]> + DerefOrBorrow<[u8]>,
    > DynamicAccount<Header, Fixed, Dynamic>
{
    fn header_mut(&mut self) -> &mut MangoAccountDynamicHeader {
        self.header.deref_or_borrow_mut()
    }
    fn fixed_mut(&mut self) -> &mut MangoAccountFixed {
        self.fixed.deref_or_borrow_mut()
    }
    fn dynamic_mut(&mut self) -> &mut [u8] {
        self.dynamic.deref_or_borrow_mut()
    }

    pub fn borrow_mut(&mut self) -> MangoAccountRefMut {
        MangoAccountRefMut {
            header: self.header.deref_or_borrow_mut(),
            fixed: self.fixed.deref_or_borrow_mut(),
            dynamic: self.dynamic.deref_or_borrow_mut(),
        }
    }

    /// Returns
    /// - the position
    /// - the raw index into the token positions list (for use with get_raw/deactivate)
    pub fn token_position_mut(
        &mut self,
        token_index: TokenIndex,
    ) -> Result<(&mut TokenPosition, usize)> {
        let raw_index = self
            .all_token_positions()
            .enumerate()
            .find_map(|(raw_index, p)| p.is_active_for_token(token_index).then_some(raw_index))
            .ok_or_else(|| {
                error_msg_typed!(
                    MangoError::TokenPositionDoesNotExist,
                    "position for token index {} not found",
                    token_index
                )
            })?;
        Ok((self.token_position_mut_by_raw_index(raw_index), raw_index))
    }

    // get mut TokenPosition at raw_index
    pub fn token_position_mut_by_raw_index(&mut self, raw_index: usize) -> &mut TokenPosition {
        let offset = self.header().token_offset(raw_index);
        get_helper_mut(self.dynamic_mut(), offset)
    }

    /// Creates or retrieves a TokenPosition for the token_index.
    /// Returns:
    /// - the position
    /// - the raw index into the token positions list (for use with get_raw)
    /// - the active index, for use with FixedOrderAccountRetriever
    pub fn ensure_token_position(
        &mut self,
        token_index: TokenIndex,
    ) -> Result<(&mut TokenPosition, usize, usize)> {
        let mut active_index = 0;
        let mut match_or_free = None;
        for (raw_index, position) in self.all_token_positions().enumerate() {
            if position.is_active_for_token(token_index) {
                // Can't return early because of lifetimes
                match_or_free = Some((raw_index, active_index));
                break;
            }
            if position.is_active() {
                active_index += 1;
            } else if match_or_free.is_none() {
                match_or_free = Some((raw_index, active_index));
            }
        }
        if let Some((raw_index, bank_index)) = match_or_free {
            let v = self.token_position_mut_by_raw_index(raw_index);
            if !v.is_active_for_token(token_index) {
                *v = TokenPosition {
                    indexed_position: I80F48::ZERO,
                    token_index,
                    in_use_count: 0,
                    cumulative_deposit_interest: 0.0,
                    cumulative_borrow_interest: 0.0,
                    previous_index: I80F48::ZERO,
                    padding: Default::default(),
                    reserved: [0; 128],
                };
            }
            Ok((v, raw_index, bank_index))
        } else {
            err!(MangoError::NoFreeTokenPositionIndex)
                .context(format!("when looking for token index {}", token_index))
        }
    }

    pub fn deactivate_token_position(&mut self, raw_index: usize) {
        assert!(self.token_position_mut_by_raw_index(raw_index).in_use_count == 0);
        self.token_position_mut_by_raw_index(raw_index).token_index = TokenIndex::MAX;
    }

    pub fn deactivate_token_position_and_log(
        &mut self,
        raw_index: usize,
        mango_account_pubkey: Pubkey,
    ) {
        let mango_group = self.fixed().group;
        let token_position = self.token_position_mut_by_raw_index(raw_index);
        assert!(token_position.in_use_count == 0);
        emit!(DeactivateTokenPositionLog {
            mango_group,
            mango_account: mango_account_pubkey,
            token_index: token_position.token_index,
            cumulative_deposit_interest: token_position.cumulative_deposit_interest,
            cumulative_borrow_interest: token_position.cumulative_borrow_interest,
        });
        self.token_position_mut_by_raw_index(raw_index).token_index = TokenIndex::MAX;
    }

    /// Decrements the in_use_count for the token position for the bank.
    ///
    /// If it goes to 0, the position may be dusted (if between 0 and 1 native tokens)
    /// and closed.
    pub fn token_decrement_dust_deactivate(
        &mut self,
        bank: &mut crate::state::Bank,
        now_ts: u64,
        mango_account_pubkey: Pubkey,
    ) -> Result<()> {
        let token_result = self.token_position_mut(bank.token_index);
        if token_result.is_anchor_error_with_code(MangoError::TokenPositionDoesNotExist.into()) {
            // Already deactivated is ok
            return Ok(());
        }
        let (position, raw_index) = token_result?;

        position.decrement_in_use();
        let active = bank.dust_if_possible(position, now_ts)?;
        if !active {
            self.deactivate_token_position_and_log(raw_index, mango_account_pubkey);
        }

        Ok(())
    }

    // get mut Serum3Orders at raw_index
    pub fn serum3_orders_mut_by_raw_index(&mut self, raw_index: usize) -> &mut Serum3Orders {
        let offset = self.header().serum3_offset(raw_index);
        get_helper_mut(self.dynamic_mut(), offset)
    }

    pub fn create_serum3_orders(
        &mut self,
        market_index: Serum3MarketIndex,
    ) -> Result<&mut Serum3Orders> {
        if self.serum3_orders(market_index).is_ok() {
            return err!(MangoError::Serum3OpenOrdersExistAlready);
        }

        let raw_index_opt = self.all_serum3_orders().position(|p| !p.is_active());
        if let Some(raw_index) = raw_index_opt {
            *(self.serum3_orders_mut_by_raw_index(raw_index)) = Serum3Orders {
                market_index: market_index as Serum3MarketIndex,
                ..Serum3Orders::default()
            };
            Ok(self.serum3_orders_mut_by_raw_index(raw_index))
        } else {
            err!(MangoError::NoFreeSerum3OpenOrdersIndex)
        }
    }

    pub fn deactivate_serum3_orders(&mut self, market_index: Serum3MarketIndex) -> Result<()> {
        let raw_index = self
            .all_serum3_orders()
            .position(|p| p.is_active_for_market(market_index))
            .ok_or_else(|| error_msg!("serum3 open orders index {} not found", market_index))?;
        self.serum3_orders_mut_by_raw_index(raw_index).market_index = Serum3MarketIndex::MAX;
        Ok(())
    }

    pub fn serum3_orders_mut(
        &mut self,
        market_index: Serum3MarketIndex,
    ) -> Result<&mut Serum3Orders> {
        let raw_index_opt = self
            .all_serum3_orders()
            .position(|p| p.is_active_for_market(market_index));
        raw_index_opt
            .map(|raw_index| self.serum3_orders_mut_by_raw_index(raw_index))
            .ok_or_else(|| error_msg!("serum3 orders for market index {} not found", market_index))
    }

    // get mut PerpPosition at raw_index
    pub fn perp_position_mut_by_raw_index(&mut self, raw_index: usize) -> &mut PerpPosition {
        let offset = self.header().perp_offset(raw_index);
        get_helper_mut(self.dynamic_mut(), offset)
    }

    pub fn perp_order_mut_by_raw_index(&mut self, raw_index: usize) -> &mut PerpOpenOrder {
        let offset = self.header().perp_oo_offset(raw_index);
        get_helper_mut(self.dynamic_mut(), offset)
    }

    pub fn perp_position_mut(
        &mut self,
        market_index: PerpMarketIndex,
    ) -> Result<&mut PerpPosition> {
        let raw_index_opt = self
            .all_perp_positions()
            .position(|p| p.is_active_for_market(market_index));
        raw_index_opt
            .map(|raw_index| self.perp_position_mut_by_raw_index(raw_index))
            .ok_or_else(|| error!(MangoError::PerpPositionDoesNotExist))
    }

    pub fn ensure_perp_position(
        &mut self,
        perp_market_index: PerpMarketIndex,
        settle_token_index: TokenIndex,
    ) -> Result<(&mut PerpPosition, usize)> {
        let mut raw_index_opt = self
            .all_perp_positions()
            .position(|p| p.is_active_for_market(perp_market_index));
        if raw_index_opt.is_none() {
            raw_index_opt = self.all_perp_positions().position(|p| !p.is_active());
            if let Some(raw_index) = raw_index_opt {
                let perp_position = self.perp_position_mut_by_raw_index(raw_index);
                *perp_position = PerpPosition::default();
                perp_position.market_index = perp_market_index;

                let settle_token_position = self.ensure_token_position(settle_token_index)?.0;
                settle_token_position.increment_in_use();
            }
        }
        if let Some(raw_index) = raw_index_opt {
            Ok((self.perp_position_mut_by_raw_index(raw_index), raw_index))
        } else {
            err!(MangoError::NoFreePerpPositionIndex)
        }
    }

    pub fn deactivate_perp_position(
        &mut self,
        perp_market_index: PerpMarketIndex,
        settle_token_index: TokenIndex,
    ) -> Result<()> {
        self.perp_position_mut(perp_market_index)?.market_index = PerpMarketIndex::MAX;

        let settle_token_position = self.token_position_mut(settle_token_index)?.0;
        settle_token_position.decrement_in_use();

        Ok(())
    }

    pub fn deactivate_perp_position_and_log(
        &mut self,
        perp_market_index: PerpMarketIndex,
        settle_token_index: TokenIndex,
        mango_account_pubkey: Pubkey,
    ) -> Result<()> {
        let mango_group = self.fixed().group;
        let perp_position = self.perp_position_mut(perp_market_index)?;

        emit!(DeactivatePerpPositionLog {
            mango_group,
            mango_account: mango_account_pubkey,
            market_index: perp_market_index,
            cumulative_long_funding: perp_position.cumulative_long_funding,
            cumulative_short_funding: perp_position.cumulative_short_funding,
            maker_volume: perp_position.maker_volume,
            taker_volume: perp_position.taker_volume,
            perp_spot_transfers: perp_position.perp_spot_transfers,
        });

        perp_position.market_index = PerpMarketIndex::MAX;

        let settle_token_position = self.token_position_mut(settle_token_index)?.0;
        settle_token_position.decrement_in_use();

        Ok(())
    }

    pub fn add_perp_order(
        &mut self,
        perp_market_index: PerpMarketIndex,
        side: Side,
        order_tree: BookSideOrderTree,
        order: &LeafNode,
        client_order_id: u64,
    ) -> Result<()> {
        let mut perp_account = self.perp_position_mut(perp_market_index)?;
        match side {
            Side::Bid => {
                perp_account.bids_base_lots += order.quantity;
            }
            Side::Ask => {
                perp_account.asks_base_lots += order.quantity;
            }
        };
        let slot = order.owner_slot as usize;

        let mut oo = self.perp_order_mut_by_raw_index(slot);
        oo.market = perp_market_index;
        oo.side_and_tree = SideAndOrderTree::new(side, order_tree).into();
        oo.id = order.key;
        oo.client_id = client_order_id;
        Ok(())
    }

    pub fn remove_perp_order(&mut self, slot: usize, quantity: i64) -> Result<()> {
        {
            let oo = self.perp_order_mut_by_raw_index(slot);
            require_neq!(oo.market, FREE_ORDER_SLOT);
            let order_side = oo.side_and_tree().side();
            let perp_market_index = oo.market;
            let perp_account = self.perp_position_mut(perp_market_index)?;

            // accounting
            match order_side {
                Side::Bid => {
                    perp_account.bids_base_lots -= quantity;
                }
                Side::Ask => {
                    perp_account.asks_base_lots -= quantity;
                }
            }
        }

        // release space
        let oo = self.perp_order_mut_by_raw_index(slot);
        oo.market = FREE_ORDER_SLOT;
        oo.side_and_tree = SideAndOrderTree::BidFixed.into();
        oo.id = 0;
        oo.client_id = 0;
        Ok(())
    }

    pub fn execute_perp_maker(
        &mut self,
        perp_market_index: PerpMarketIndex,
        perp_market: &mut PerpMarket,
        fill: &FillEvent,
        group: &Group,
    ) -> Result<()> {
        let side = fill.taker_side().invert_side();
        let (base_change, quote_change) = fill.base_quote_change(side);
        let quote = I80F48::from(perp_market.quote_lot_size) * I80F48::from(quote_change);
        let fees = quote.abs() * I80F48::from_num(fill.maker_fee);
        if fees.is_positive() {
            let f = self.fixed_mut();
            let now_ts = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
            f.expire_buyback_fees(now_ts, group.buyback_fees_expiry_interval);
            f.accrue_buyback_fees(fees.floor().to_num::<u64>());
        }
        let pa = self.perp_position_mut(perp_market_index)?;
        pa.settle_funding(perp_market);
        pa.record_trading_fee(fees);
        pa.record_trade(perp_market, base_change, quote);

        pa.maker_volume += quote.abs().to_num::<u64>();

        if fill.maker_out() {
            self.remove_perp_order(fill.maker_slot as usize, base_change.abs())
        } else {
            match side {
                Side::Bid => {
                    pa.bids_base_lots -= base_change.abs();
                }
                Side::Ask => {
                    pa.asks_base_lots -= base_change.abs();
                }
            }
            Ok(())
        }
    }

    pub fn execute_perp_taker(
        &mut self,
        perp_market_index: PerpMarketIndex,
        perp_market: &mut PerpMarket,
        fill: &FillEvent,
    ) -> Result<()> {
        let pa = self.perp_position_mut(perp_market_index)?;
        pa.settle_funding(perp_market);

        let (base_change, quote_change) = fill.base_quote_change(fill.taker_side());
        pa.remove_taker_trade(base_change, quote_change);
        // fees are assessed at time of trade; no need to assess fees here
        let quote_change_native =
            I80F48::from(perp_market.quote_lot_size) * I80F48::from(quote_change);
        pa.record_trade(perp_market, base_change, quote_change_native);

        pa.taker_volume += quote_change_native.abs().to_num::<u64>();

        Ok(())
    }

    pub fn token_conditional_swap_mut_by_index(
        &mut self,
        index: usize,
    ) -> Result<&mut TokenConditionalSwap> {
        let count: usize = self.header().token_conditional_swap_count.into();
        require_gt!(count, index);
        let offset = self.header().token_conditional_swap_offset(index);
        Ok(get_helper_mut(self.dynamic_mut(), offset))
    }

    pub fn free_token_conditional_swap_mut(&mut self) -> Result<&mut TokenConditionalSwap> {
        let index = self.token_conditional_swap_free_index()?;
        let tcs = self.token_conditional_swap_mut_by_index(index)?;
        Ok(tcs)
    }

    pub fn check_health_pre(&mut self, health_cache: &HealthCache) -> Result<I80F48> {
        let pre_init_health = health_cache.health(HealthType::Init);
        msg!("pre_init_health: {}", pre_init_health);
        self.check_health_pre_checks(health_cache, pre_init_health)?;
        Ok(pre_init_health)
    }

    pub fn check_health_pre_checks(
        &mut self,
        health_cache: &HealthCache,
        pre_init_health: I80F48,
    ) -> Result<()> {
        // We can skip computing LiquidationEnd health if Init health > 0, because
        // LiquidationEnd health >= Init health.
        self.fixed_mut()
            .maybe_recover_from_being_liquidated(pre_init_health);
        if self.fixed().being_liquidated() {
            let liq_end_health = health_cache.health(HealthType::LiquidationEnd);
            self.fixed_mut()
                .maybe_recover_from_being_liquidated(liq_end_health);
        }
        require!(
            !self.fixed().being_liquidated(),
            MangoError::BeingLiquidated
        );
        Ok(())
    }

    pub fn check_health_post(
        &mut self,
        health_cache: &HealthCache,
        pre_init_health: I80F48,
    ) -> Result<I80F48> {
        let post_init_health = health_cache.health(HealthType::Init);
        msg!("post_init_health: {}", post_init_health);
        self.check_health_post_checks(pre_init_health, post_init_health)?;
        Ok(post_init_health)
    }

    pub fn check_health_post_checks(
        &mut self,
        pre_init_health: I80F48,
        post_init_health: I80F48,
    ) -> Result<()> {
        // Accounts that have negative init health may only take actions that don't further
        // decrease their health.
        // To avoid issues with rounding, we allow accounts to decrease their health by up to
        // $1e-6. This is safe because the grace amount is way less than the cost of a transaction.
        // And worst case, users can only use this to gradually drive their own account into
        // liquidation.
        // There is an exception for accounts with health between $0 and -$0.001 (-1000 native),
        // because we don't want to allow empty accounts or accounts with extremely tiny deposits
        // to immediately drive themselves into bankruptcy. (accounts with large deposits can also
        // be in this health range, but it's really unlikely)
        let health_does_not_decrease = if post_init_health < -1000 {
            post_init_health.ceil() >= pre_init_health.ceil()
        } else {
            post_init_health >= pre_init_health
        };

        require!(
            post_init_health >= 0 || health_does_not_decrease,
            MangoError::HealthMustBePositiveOrIncrease
        );
        Ok(())
    }

    pub fn check_liquidatable(&mut self, health_cache: &HealthCache) -> Result<CheckLiquidatable> {
        // Once maint_health falls below 0, we want to start liquidating,
        // we want to allow liquidation to continue until init_health is positive,
        // to prevent constant oscillation between the two states
        if self.being_liquidated() {
            let liq_end_health = health_cache.health(HealthType::LiquidationEnd);
            if self
                .fixed_mut()
                .maybe_recover_from_being_liquidated(liq_end_health)
            {
                msg!("Liqee init_health above zero");
                return Ok(CheckLiquidatable::BecameNotLiquidatable);
            }
        } else {
            let maint_health = health_cache.health(HealthType::Maint);
            if maint_health >= I80F48::ZERO {
                msg!("Liqee is not liquidatable");
                return Ok(CheckLiquidatable::NotLiquidatable);
            }
            self.fixed_mut().set_being_liquidated(true);
        }
        return Ok(CheckLiquidatable::Liquidatable);
    }

    fn write_borsh_vec_length(&mut self, offset: usize, count: u8) {
        let dst: &mut [u8] = &mut self.dynamic_mut()[offset - BORSH_VEC_SIZE_BYTES..offset];
        dst.copy_from_slice(&BorshVecLength::from(count).to_le_bytes());
    }

    // writes length of tokens vec at appropriate offset so that borsh can infer the vector length
    // length used is that present in the header
    fn write_token_length(&mut self) {
        let offset = self.header().token_offset(0);
        let count = self.header().token_count;
        self.write_borsh_vec_length(offset, count)
    }

    fn write_serum3_length(&mut self) {
        let offset = self.header().serum3_offset(0);
        let count = self.header().serum3_count;
        self.write_borsh_vec_length(offset, count)
    }

    fn write_perp_length(&mut self) {
        let offset = self.header().perp_offset(0);
        let count = self.header().perp_count;
        self.write_borsh_vec_length(offset, count)
    }

    fn write_perp_oo_length(&mut self) {
        let offset = self.header().perp_oo_offset(0);
        let count = self.header().perp_oo_count;
        self.write_borsh_vec_length(offset, count)
    }

    fn write_token_conditional_swap_length(&mut self) {
        let offset = self.header().token_conditional_swap_offset(0);
        let count = self.header().token_conditional_swap_count;
        self.write_borsh_vec_length(offset, count)
    }

    pub fn expand_dynamic_content(
        &mut self,
        new_token_count: u8,
        new_serum3_count: u8,
        new_perp_count: u8,
        new_perp_oo_count: u8,
        new_token_conditional_swap_count: u8,
    ) -> Result<()> {
        let new_header = MangoAccountDynamicHeader {
            token_count: new_token_count,
            serum3_count: new_serum3_count,
            perp_count: new_perp_count,
            perp_oo_count: new_perp_oo_count,
            token_conditional_swap_count: new_token_conditional_swap_count,
        };
        let old_header = self.header().clone();

        new_header.check_resize_from(&old_header)?;

        let dynamic = self.dynamic_mut();

        // expand dynamic components by first moving existing positions, and then setting new ones to defaults


        // TODO: If we want to allow shrinking
        // - Determine if new start pos is to the left or to the right,
        //   and then either shift r-to-l or reverse.
        // - "Defrag" by always copying only active positions over
        // - zero memory with inactive positions to ensure there's
        //   no leftover bytes in between active positions

        // Possibly we could make i teasier for us by starting with the
        // defrag step, then all useful data would be contiguous

        // Attempt at defrag
        let mut next_free = 0;
        for (i, pos) in self.all_token_positions().enumerate() {
            if pos.is_active() && i != next_free {
                memmove(pos to bytes[next_free]);
                next_free += 1;
            }
        }
        // Do the same for the other positions

        // Moving the data is difficult: cases:
        //
        // TTTT....PP....SSS...
        // TTTTPPSSS (copy PP first then SSS)
        //
        // TTTTPPSSS
        // TTTT....PP....SSS...  (copy SSS first, then PP
        //
        // Do all moves to the left l-to-r and all to the right r-to-l?

        // Algorithm:
        // - if it's copy to the left, do it, go next
        // - if it's copy to the right, skip on first pass, go next
        // on the second pass to copies to the right in reverse
        // zero the empty spots in a third pass

        // TTTT....PP....SSS...
        // TTTT.....PP..SSS...
    )

        // token conditional swaps
        if old_header.token_conditional_swap_count() > 0 {
            unsafe {
                sol_memmove(
                    &mut dynamic[new_header.token_conditional_swap_offset(0)],
                    &mut dynamic[old_header.token_conditional_swap_offset(0)],
                    size_of::<TokenConditionalSwap>() * old_header.token_conditional_swap_count(),
                );
            }
        }
        for i in old_header.token_conditional_swap_count..new_token_conditional_swap_count {
            *get_helper_mut(dynamic, new_header.token_conditional_swap_offset(i.into())) =
                TokenConditionalSwap::default();
        }

        // perp oo
        if old_header.perp_oo_count() > 0 {
            unsafe {
                sol_memmove(
                    &mut dynamic[new_header.perp_oo_offset(0)],
                    &mut dynamic[old_header.perp_oo_offset(0)],
                    size_of::<PerpOpenOrder>() * old_header.perp_oo_count(),
                );
            }
        }
        for i in old_header.perp_oo_count..new_perp_oo_count {
            *get_helper_mut(dynamic, new_header.perp_oo_offset(i.into())) =
                PerpOpenOrder::default();
        }

        // perp positions
        if old_header.perp_count() > 0 {
            unsafe {
                sol_memmove(
                    &mut dynamic[new_header.perp_offset(0)],
                    &mut dynamic[old_header.perp_offset(0)],
                    size_of::<PerpPosition>() * old_header.perp_count(),
                );
            }
        }
        for i in old_header.perp_count..new_perp_count {
            *get_helper_mut(dynamic, new_header.perp_offset(i.into())) = PerpPosition::default();
        }

        // serum3 positions
        if old_header.serum3_count() > 0 {
            unsafe {
                sol_memmove(
                    &mut dynamic[new_header.serum3_offset(0)],
                    &mut dynamic[old_header.serum3_offset(0)],
                    size_of::<Serum3Orders>() * old_header.serum3_count(),
                );
            }
        }
        for i in old_header.serum3_count..new_serum3_count {
            *get_helper_mut(dynamic, new_header.serum3_offset(i.into())) = Serum3Orders::default();
        }

        // token positions
        if old_header.token_count() > 0 {
            unsafe {
                sol_memmove(
                    &mut dynamic[new_header.token_offset(0)],
                    &mut dynamic[old_header.token_offset(0)],
                    size_of::<TokenPosition>() * old_header.token_count(),
                );
            }
        }
        for i in old_header.token_count..new_token_count {
            *get_helper_mut(dynamic, new_header.token_offset(i.into())) = TokenPosition::default();
        }

        // update the already-parsed header
        *self.header_mut() = new_header;

        // write new lengths to the dynamic data (uses header)
        self.write_token_length();
        self.write_serum3_length();
        self.write_perp_length();
        self.write_perp_oo_length();
        self.write_token_conditional_swap_length();

        Ok(())
    }
}

/// Trait to allow a AccountLoader<MangoAccountFixed> to create an accessor for the full account.
pub trait MangoAccountLoader<'a> {
    fn load_full(self) -> Result<MangoAccountLoadedRefCell<'a>>;
    fn load_full_mut(self) -> Result<MangoAccountLoadedRefCellMut<'a>>;
    fn load_full_init(self) -> Result<MangoAccountLoadedRefCellMut<'a>>;
}

impl<'a, 'info: 'a> MangoAccountLoader<'a> for &'a AccountLoader<'info, MangoAccountFixed> {
    fn load_full(self) -> Result<MangoAccountLoadedRefCell<'a>> {
        // Error checking
        self.load()?;

        let data = self.as_ref().try_borrow_data()?;
        let header =
            MangoAccountDynamicHeader::from_bytes(&data[8 + size_of::<MangoAccountFixed>()..])?;
        let (_, data) = Ref::map_split(data, |d| d.split_at(8));
        let (fixed_bytes, dynamic) =
            Ref::map_split(data, |d| d.split_at(size_of::<MangoAccountFixed>()));
        Ok(MangoAccountLoadedRefCell {
            header,
            fixed: Ref::map(fixed_bytes, |b| bytemuck::from_bytes(b)),
            dynamic,
        })
    }

    fn load_full_mut(self) -> Result<MangoAccountLoadedRefCellMut<'a>> {
        // Error checking
        self.load_mut()?;

        let data = self.as_ref().try_borrow_mut_data()?;
        let header =
            MangoAccountDynamicHeader::from_bytes(&data[8 + size_of::<MangoAccountFixed>()..])?;
        let (_, data) = RefMut::map_split(data, |d| d.split_at_mut(8));
        let (fixed_bytes, dynamic) =
            RefMut::map_split(data, |d| d.split_at_mut(size_of::<MangoAccountFixed>()));
        Ok(MangoAccountLoadedRefCellMut {
            header,
            fixed: RefMut::map(fixed_bytes, |b| bytemuck::from_bytes_mut(b)),
            dynamic,
        })
    }

    fn load_full_init(self) -> Result<MangoAccountLoadedRefCellMut<'a>> {
        // Error checking
        self.load_init()?;

        {
            let mut data = self.as_ref().try_borrow_mut_data()?;

            let disc_bytes: &mut [u8] = &mut data[0..8];
            disc_bytes.copy_from_slice(bytemuck::bytes_of(&(MangoAccount::discriminator())));

            MangoAccountDynamicHeader::initialize(&mut data[8 + size_of::<MangoAccountFixed>()..])?;
        }

        self.load_full_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_account() -> MangoAccountValue {
        let account = MangoAccount::default_for_tests();
        let mut bytes = AnchorSerialize::try_to_vec(&account).unwrap();

        // The MangoAccount struct is missing some dynamic fields, add space for them
        let tcs_length = 2;
        let expected_space = MangoAccount::space(
            account.tokens.len() as u8,
            account.serum3.len() as u8,
            account.perps.len() as u8,
            account.perp_open_orders.len() as u8,
            tcs_length,
        );
        bytes.extend(vec![0u8; expected_space - bytes.len()]);

        // Set the length of these dynamic parts
        let (fixed, dynamic) = bytes.split_at_mut(size_of::<MangoAccountFixed>());
        let mut header = MangoAccountDynamicHeader::from_bytes(dynamic).unwrap();
        header.token_conditional_swap_count = tcs_length;
        let mut account = MangoAccountRefMut {
            header: &mut header,
            fixed: bytemuck::from_bytes_mut(fixed),
            dynamic,
        };
        account.write_token_conditional_swap_length();

        MangoAccountValue::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn test_serialization_match() {
        let mut account = MangoAccount::default_for_tests();
        account.group = Pubkey::new_unique();
        account.owner = Pubkey::new_unique();
        account.name = crate::util::fill_from_str("abcdef").unwrap();
        account.delegate = Pubkey::new_unique();
        account.account_num = 1;
        account.being_liquidated = 2;
        account.in_health_region = 3;
        account.bump = 4;
        account.net_deposits = 5;
        account.health_region_begin_init_health = 7;
        account.buyback_fees_accrued_current = 10;
        account.buyback_fees_accrued_previous = 11;
        account.buyback_fees_expiry_timestamp = 12;
        account.tokens.resize(8, TokenPosition::default());
        account.tokens[0].token_index = 8;
        account.serum3.resize(8, Serum3Orders::default());
        account.perps.resize(4, PerpPosition::default());
        account.perps[0].market_index = 9;
        account.perp_open_orders.resize(8, PerpOpenOrder::default());
        account.next_token_conditional_swap_id = 13;

        let account_bytes_without_tcs = AnchorSerialize::try_to_vec(&account).unwrap();
        let account_bytes_with_tcs = {
            let mut b = account_bytes_without_tcs.clone();
            // tcs adds 4 bytes of padding and 4 bytes of Vec size
            b.extend([0u8; 8]);
            b
        };
        assert_eq!(
            8 + account_bytes_with_tcs.len(),
            MangoAccount::space(8, 8, 4, 8, 0)
        );

        let account2 = MangoAccountValue::from_bytes(&account_bytes_without_tcs).unwrap();
        assert_eq!(account.group, account2.fixed.group);
        assert_eq!(account.owner, account2.fixed.owner);
        assert_eq!(account.name, account2.fixed.name);
        assert_eq!(account.delegate, account2.fixed.delegate);
        assert_eq!(account.account_num, account2.fixed.account_num);
        assert_eq!(account.being_liquidated, account2.fixed.being_liquidated);
        assert_eq!(account.in_health_region, account2.fixed.in_health_region);
        assert_eq!(account.bump, account2.fixed.bump);
        assert_eq!(account.net_deposits, account2.fixed.net_deposits);
        assert_eq!(
            account.perp_spot_transfers,
            account2.fixed.perp_spot_transfers
        );
        assert_eq!(
            account.health_region_begin_init_health,
            account2.fixed.health_region_begin_init_health
        );
        assert_eq!(
            account.buyback_fees_accrued_current,
            account2.fixed.buyback_fees_accrued_current
        );
        assert_eq!(
            account.buyback_fees_accrued_previous,
            account2.fixed.buyback_fees_accrued_previous
        );
        assert_eq!(
            account.buyback_fees_expiry_timestamp,
            account2.fixed.buyback_fees_expiry_timestamp
        );
        assert_eq!(
            account.next_token_conditional_swap_id,
            account2.fixed.next_token_conditional_swap_id
        );
        assert_eq!(
            account.tokens[0].token_index,
            account2
                .token_position_by_raw_index_unchecked(0)
                .token_index
        );
        assert_eq!(
            account.serum3[0].open_orders,
            account2.serum3_orders_by_raw_index_unchecked(0).open_orders
        );
        assert_eq!(
            account.perps[0].market_index,
            account2
                .perp_position_by_raw_index_unchecked(0)
                .market_index
        );
        assert_eq!(account2.all_token_conditional_swaps().count(), 0);

        let account3 = MangoAccountValue::from_bytes(&account_bytes_with_tcs).unwrap();
        assert_eq!(account3.all_token_conditional_swaps().count(), 0);
    }

    #[test]
    fn test_token_positions() {
        let mut account = make_test_account();
        assert!(account.token_position(1).is_err());
        assert!(account.token_position_and_raw_index(2).is_err());
        assert!(account.token_position_mut(3).is_err());
        assert_eq!(
            account.token_position_by_raw_index_unchecked(0).token_index,
            TokenIndex::MAX
        );

        {
            let (pos, raw, active) = account.ensure_token_position(1).unwrap();
            assert_eq!(raw, 0);
            assert_eq!(active, 0);
            assert_eq!(pos.token_index, 1);
        }
        {
            let (pos, raw, active) = account.ensure_token_position(7).unwrap();
            assert_eq!(raw, 1);
            assert_eq!(active, 1);
            assert_eq!(pos.token_index, 7);
        }
        {
            let (pos, raw, active) = account.ensure_token_position(42).unwrap();
            assert_eq!(raw, 2);
            assert_eq!(active, 2);
            assert_eq!(pos.token_index, 42);
        }

        {
            account.deactivate_token_position(1);

            let (pos, raw, active) = account.ensure_token_position(42).unwrap();
            assert_eq!(raw, 2);
            assert_eq!(active, 1);
            assert_eq!(pos.token_index, 42);

            let (pos, raw, active) = account.ensure_token_position(8).unwrap();
            assert_eq!(raw, 1);
            assert_eq!(active, 1);
            assert_eq!(pos.token_index, 8);
        }

        assert_eq!(account.active_token_positions().count(), 3);
        account.deactivate_token_position(0);
        assert_eq!(
            account.token_position_by_raw_index_unchecked(0).token_index,
            TokenIndex::MAX
        );
        assert!(account.token_position(1).is_err());
        assert!(account.token_position_mut(1).is_err());
        assert!(account.token_position(8).is_ok());
        assert!(account.token_position(42).is_ok());
        assert_eq!(account.token_position_and_raw_index(42).unwrap().1, 2);
        assert_eq!(account.active_token_positions().count(), 2);

        {
            let (pos, raw) = account.token_position_mut(42).unwrap();
            assert_eq!(pos.token_index, 42);
            assert_eq!(raw, 2);
        }
        {
            let (pos, raw) = account.token_position_mut(8).unwrap();
            assert_eq!(pos.token_index, 8);
            assert_eq!(raw, 1);
        }
    }

    #[test]
    fn test_serum3_orders() {
        let mut account = make_test_account();
        assert!(account.serum3_orders(1).is_err());
        assert!(account.serum3_orders_mut(3).is_err());
        assert_eq!(
            account.serum3_orders_by_raw_index_unchecked(0).market_index,
            Serum3MarketIndex::MAX
        );

        assert_eq!(account.create_serum3_orders(1).unwrap().market_index, 1);
        assert_eq!(account.create_serum3_orders(7).unwrap().market_index, 7);
        assert_eq!(account.create_serum3_orders(42).unwrap().market_index, 42);
        assert!(account.create_serum3_orders(7).is_err());
        assert_eq!(account.active_serum3_orders().count(), 3);

        assert!(account.deactivate_serum3_orders(7).is_ok());
        assert_eq!(
            account.serum3_orders_by_raw_index_unchecked(1).market_index,
            Serum3MarketIndex::MAX
        );
        assert!(account.create_serum3_orders(8).is_ok());
        assert_eq!(
            account.serum3_orders_by_raw_index_unchecked(1).market_index,
            8
        );

        assert_eq!(account.active_serum3_orders().count(), 3);
        assert!(account.deactivate_serum3_orders(1).is_ok());
        assert!(account.serum3_orders(1).is_err());
        assert!(account.serum3_orders_mut(1).is_err());
        assert!(account.serum3_orders(8).is_ok());
        assert!(account.serum3_orders(42).is_ok());
        assert_eq!(account.active_serum3_orders().count(), 2);

        assert_eq!(account.serum3_orders_mut(42).unwrap().market_index, 42);
        assert_eq!(account.serum3_orders_mut(8).unwrap().market_index, 8);
        assert!(account.serum3_orders_mut(7).is_err());
    }

    #[test]
    fn test_perp_positions() {
        let mut account = make_test_account();
        assert!(account.perp_position(1).is_err());
        assert!(account.perp_position_mut(3).is_err());
        assert_eq!(
            account.perp_position_by_raw_index_unchecked(0).market_index,
            PerpMarketIndex::MAX
        );

        {
            let (pos, raw) = account.ensure_perp_position(1, 0).unwrap();
            assert_eq!(raw, 0);
            assert_eq!(pos.market_index, 1);
            assert_eq!(account.token_position_mut(0).unwrap().0.in_use_count, 1);
        }
        {
            let (pos, raw) = account.ensure_perp_position(7, 0).unwrap();
            assert_eq!(raw, 1);
            assert_eq!(pos.market_index, 7);
            assert_eq!(account.token_position_mut(0).unwrap().0.in_use_count, 2);
        }
        {
            let (pos, raw) = account.ensure_perp_position(42, 0).unwrap();
            assert_eq!(raw, 2);
            assert_eq!(pos.market_index, 42);
            assert_eq!(account.token_position_mut(0).unwrap().0.in_use_count, 3);
        }

        {
            let pos_res = account.perp_position_mut(1);
            assert!(pos_res.is_ok());
            assert_eq!(pos_res.unwrap().market_index, 1)
        }

        {
            let pos_res = account.perp_position_mut(99);
            assert!(pos_res.is_err());
        }

        {
            assert!(account.deactivate_perp_position(7, 0).is_ok());

            let (pos, raw) = account.ensure_perp_position(42, 0).unwrap();
            assert_eq!(raw, 2);
            assert_eq!(pos.market_index, 42);
            assert_eq!(account.token_position_mut(0).unwrap().0.in_use_count, 2);

            let (pos, raw) = account.ensure_perp_position(8, 0).unwrap();
            assert_eq!(raw, 1);
            assert_eq!(pos.market_index, 8);
            assert_eq!(account.token_position_mut(0).unwrap().0.in_use_count, 3);
        }

        assert_eq!(account.active_perp_positions().count(), 3);
        assert!(account.deactivate_perp_position(1, 0).is_ok());
        assert_eq!(
            account.perp_position_by_raw_index_unchecked(0).market_index,
            PerpMarketIndex::MAX
        );
        assert!(account.perp_position(1).is_err());
        assert!(account.perp_position_mut(1).is_err());
        assert!(account.perp_position(8).is_ok());
        assert!(account.perp_position(42).is_ok());
        assert_eq!(account.active_perp_positions().count(), 2);
    }

    #[test]
    fn test_buyback_fees() {
        let mut account = make_test_account();
        let fixed = account.fixed_mut();
        assert_eq!(fixed.buyback_fees_accrued(), 0);
        fixed.expire_buyback_fees(1000, 10);
        assert_eq!(fixed.buyback_fees_accrued(), 0);
        assert_eq!(fixed.buyback_fees_expiry_timestamp, 1010);

        fixed.accrue_buyback_fees(10);
        fixed.accrue_buyback_fees(5);
        assert_eq!(fixed.buyback_fees_accrued(), 15);
        fixed.reduce_buyback_fees_accrued(2);
        assert_eq!(fixed.buyback_fees_accrued(), 13);

        fixed.expire_buyback_fees(1009, 10);
        assert_eq!(fixed.buyback_fees_expiry_timestamp, 1010);
        assert_eq!(fixed.buyback_fees_accrued(), 13);
        assert_eq!(fixed.buyback_fees_accrued_current, 13);

        fixed.expire_buyback_fees(1010, 10);
        assert_eq!(fixed.buyback_fees_expiry_timestamp, 1020);
        assert_eq!(fixed.buyback_fees_accrued(), 13);
        assert_eq!(fixed.buyback_fees_accrued_previous, 13);
        assert_eq!(fixed.buyback_fees_accrued_current, 0);

        fixed.accrue_buyback_fees(5);
        assert_eq!(fixed.buyback_fees_accrued(), 18);

        fixed.reduce_buyback_fees_accrued(15);
        assert_eq!(fixed.buyback_fees_accrued(), 3);
        assert_eq!(fixed.buyback_fees_accrued_previous, 0);
        assert_eq!(fixed.buyback_fees_accrued_current, 3);

        fixed.expire_buyback_fees(1021, 10);
        fixed.accrue_buyback_fees(1);
        assert_eq!(fixed.buyback_fees_expiry_timestamp, 1030);
        assert_eq!(fixed.buyback_fees_accrued_previous, 3);
        assert_eq!(fixed.buyback_fees_accrued_current, 1);

        fixed.expire_buyback_fees(1051, 10);
        assert_eq!(fixed.buyback_fees_expiry_timestamp, 1060);
        assert_eq!(fixed.buyback_fees_accrued_previous, 0);
        assert_eq!(fixed.buyback_fees_accrued_current, 0);

        fixed.accrue_buyback_fees(7);
        fixed.expire_buyback_fees(1060, 10);
        fixed.accrue_buyback_fees(5);
        assert_eq!(fixed.buyback_fees_expiry_timestamp, 1070);
        assert_eq!(fixed.buyback_fees_accrued(), 12);

        fixed.reduce_buyback_fees_accrued(12);
        assert_eq!(fixed.buyback_fees_accrued(), 0);
    }

    #[test]
    fn test_token_conditional_swap() {
        let mut account = make_test_account();
        assert_eq!(account.all_token_conditional_swaps().count(), 2);
        assert_eq!(account.active_token_conditional_swaps().count(), 0);
        assert_eq!(account.token_conditional_swap_free_index().unwrap(), 0);

        let tcs = account.free_token_conditional_swap_mut().unwrap();
        tcs.id = 123;
        tcs.has_data = 1;
        assert_eq!(account.all_token_conditional_swaps().count(), 2);
        assert_eq!(account.active_token_conditional_swaps().count(), 1);
        assert_eq!(account.token_conditional_swap_free_index().unwrap(), 1);

        let tcs = account.free_token_conditional_swap_mut().unwrap();
        tcs.id = 234;
        tcs.has_data = 1;
        assert_eq!(account.all_token_conditional_swaps().count(), 2);
        assert_eq!(account.active_token_conditional_swaps().count(), 2);

        let (index, tcs) = account.token_conditional_swap_by_id(123).unwrap();
        assert_eq!(index, 0);
        assert_eq!(tcs.id, 123);
        let tcs = account.token_conditional_swap_by_index(0).unwrap();
        assert_eq!(tcs.id, 123);

        let (index, tcs) = account.token_conditional_swap_by_id(234).unwrap();
        assert_eq!(index, 1);
        assert_eq!(tcs.id, 234);
        let tcs = account.token_conditional_swap_by_index(1).unwrap();
        assert_eq!(tcs.id, 234);

        assert!(account.free_token_conditional_swap_mut().is_err());
        assert!(account.token_conditional_swap_free_index().is_err());

        let tcs = account.token_conditional_swap_mut_by_index(0).unwrap();
        tcs.has_data = 0;
        assert_eq!(account.all_token_conditional_swaps().count(), 2);
        assert_eq!(account.active_token_conditional_swaps().count(), 1);
        assert_eq!(
            account.active_token_conditional_swaps().next().unwrap().id,
            234
        );
        assert!(account.token_conditional_swap_by_id(123).is_err());

        assert_eq!(account.token_conditional_swap_free_index().unwrap(), 0);
        let tcs = account.free_token_conditional_swap_mut().unwrap();
        assert_eq!(tcs.id, 123); // old data
    }
}
