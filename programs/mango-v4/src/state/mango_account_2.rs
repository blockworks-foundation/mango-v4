use std::cell::{Ref, RefMut};
use std::fmt;

use std::marker::PhantomData;
use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use arrayref::array_ref;

use fixed::types::I80F48;
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use solana_program::program_memory::sol_memmove;

use crate::error::Contextable;
use crate::error::MangoError;
use crate::error_msg;

use super::FillEvent;
use super::LeafNode;
use super::PerpMarket;
use super::PerpMarketIndex;
use super::PerpOpenOrders;
use super::Serum3MarketIndex;
use super::Side;
use super::TokenIndex;
use super::FREE_ORDER_SLOT;
use super::{PerpPositions, Serum3Orders, TokenPosition};
use checked_math as cm;

type BorshVecLength = u32;
const BORSH_VEC_PADDING_BYTES: usize = 4;
const BORSH_VEC_SIZE_BYTES: usize = 4;

#[derive(
    Debug,
    Eq,
    PartialEq,
    Clone,
    Copy,
    TryFromPrimitive,
    IntoPrimitive,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[repr(u8)]

pub enum AccountSize {
    Small = 0,
    Large = 1,
}

impl fmt::Display for AccountSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AccountSize::Small => write!(f, "Small"),
            AccountSize::Large => write!(f, "Large"),
        }
    }
}

impl AccountSize {
    pub fn space(&self) -> (u8, u8, u8, u8) {
        match self {
            AccountSize::Small => (8, 2, 2, 2),
            AccountSize::Large => (16, 8, 8, 8),
        }
    }
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

    /// This account cannot open new positions or borrow until `init_health >= 0`
    being_liquidated: u8,

    /// This account cannot do anything except go through `resolve_bankruptcy`
    is_bankrupt: u8,

    pub account_num: u8,
    pub bump: u8,

    // pub info: [u8; INFO_LEN], // TODO: Info could be in a separate PDA?
    pub reserved: [u8; 4],

    // Cumulative (deposits - withdraws)
    // using USD prices at the time of the deposit/withdraw
    // in UI USD units
    pub net_deposits: f32,
    // Cumulative settles on perp positions
    // TODO: unimplemented
    pub net_settled: f32,

    // dynamic
    // note: padding is required for TokenPosition, etc. to be aligned
    pub padding1: u32,
    // Maps token_index -> deposit/borrow account for each token
    // that is active on this MangoAccount.
    pub tokens: Vec<TokenPosition>,
    pub padding2: u32,
    // Maps serum_market_index -> open orders for each serum market
    // that is active on this MangoAccount.
    pub serum3: Vec<Serum3Orders>,
    pub padding3: u32,
    pub perps: Vec<PerpPositions>,
    pub padding4: u32,
    pub perp_open_orders: Vec<PerpOpenOrders>,
}

impl Default for MangoAccount {
    fn default() -> Self {
        Self {
            name: Default::default(),
            group: Pubkey::default(),
            owner: Pubkey::default(),
            delegate: Pubkey::default(),
            being_liquidated: 0,
            is_bankrupt: 0,
            account_num: 0,
            bump: 0,
            reserved: Default::default(),
            net_deposits: 0.0,
            net_settled: 0.0,
            padding1: Default::default(),
            tokens: vec![TokenPosition::default(); 3],
            padding2: Default::default(),
            serum3: vec![Serum3Orders::default(); 5],
            padding3: Default::default(),
            perps: vec![PerpPositions::default(); 2],
            padding4: Default::default(),
            perp_open_orders: vec![PerpOpenOrders::default(); 2],
        }
    }
}

impl MangoAccount {
    pub fn space(account_size: AccountSize) -> usize {
        let (token_count, serum3_count, perp_count, perp_oo_count) = account_size.space();

        8 + size_of::<MangoAccountFixed>()
            + Self::dynamic_size(token_count, serum3_count, perp_count, perp_oo_count)
    }

    pub fn dynamic_token_vec_offset() -> usize {
        BORSH_VEC_PADDING_BYTES
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
            + (BORSH_VEC_SIZE_BYTES + size_of::<PerpPositions>() * usize::from(perp_count))
            + BORSH_VEC_PADDING_BYTES
    }

    pub fn dynamic_size(
        token_count: u8,
        serum3_count: u8,
        perp_count: u8,
        perp_oo_count: u8,
    ) -> usize {
        Self::dynamic_perp_oo_vec_offset(token_count, serum3_count, perp_count)
            + (BORSH_VEC_SIZE_BYTES + size_of::<PerpOpenOrders>() * usize::from(perp_oo_count))
    }
}

#[test]
fn test_dynamic_offsets() {
    let mut account = MangoAccount::default();
    account.tokens.resize(16, TokenPosition::default());
    account.serum3.resize(8, Serum3Orders::default());
    account.perps.resize(8, PerpPositions::default());
    account
        .perp_open_orders
        .resize(8, PerpOpenOrders::default());
    assert_eq!(
        8 + AnchorSerialize::try_to_vec(&account).unwrap().len(),
        MangoAccount::space(AccountSize::Large.try_into().unwrap())
    );
}

// Mango Account fixed part for easy zero copy deserialization
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct MangoAccountFixed {
    pub group: Pubkey,
    pub owner: Pubkey,
    pub name: [u8; 32],
    pub delegate: Pubkey,
    being_liquidated: u8,
    is_bankrupt: u8,
    pub account_num: u8,
    pub bump: u8,
    pub reserved: [u8; 4],
    pub net_deposits: f32,
    pub net_settled: f32,
}

impl MangoAccountFixed {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn is_owner_or_delegate(&self, ix_signer: Pubkey) -> bool {
        self.owner == ix_signer || self.delegate == ix_signer
    }

    pub fn is_bankrupt(&self) -> bool {
        self.is_bankrupt != 0
    }

    pub fn set_bankrupt(&mut self, b: bool) {
        self.is_bankrupt = if b { 1 } else { 0 };
    }

    pub fn being_liquidated(&self) -> bool {
        self.being_liquidated != 0
    }

    pub fn set_being_liquidated(&mut self, b: bool) {
        self.being_liquidated = if b { 1 } else { 0 };
    }
}

// Header is created by scanning and parsing dynamic portion of the account
// Header stores useful information e.g. offsets to easily seek into dynamic content
pub trait Header: Sized {
    // build header by scanning and parsing dynamic portion of the account
    fn from_bytes(data: &[u8]) -> Result<Self>;

    // initialize a header on a new account, if necessary
    fn initialize(data: &mut [u8]) -> Result<()>;
}

pub trait DynamicAccount: Owner + Discriminator {
    type Header: Header;
    type Fixed: bytemuck::Pod;
}

impl DynamicAccount for MangoAccount {
    type Header = MangoAccountDynamicHeader;
    type Fixed = MangoAccountFixed;
}

#[derive(Clone)]
pub struct MangoAccountDynamicHeader {
    pub token_count: u8,
    pub serum3_count: u8,
    pub perp_count: u8,
    pub perp_oo_count: u8,
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

    // offset into dynamic data where 1st PerpPositions would be found
    fn perp_offset(&self, raw_index: usize) -> usize {
        MangoAccount::dynamic_perp_vec_offset(self.token_count, self.serum3_count)
            + BORSH_VEC_SIZE_BYTES
            + raw_index * size_of::<PerpPositions>()
    }

    fn perp_oo_offset(&self, raw_index: usize) -> usize {
        MangoAccount::dynamic_perp_oo_vec_offset(
            self.token_count,
            self.serum3_count,
            self.perp_count,
        ) + BORSH_VEC_SIZE_BYTES
            + raw_index * size_of::<PerpOpenOrders>()
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
}

#[derive(Clone)]
pub struct DynamicAccessor<Header, Fixed, Dynamic> {
    pub header: Header,
    pub fixed: Fixed,
    pub dynamic: Dynamic,
}

type DynamicAccessorValue<D> =
    DynamicAccessor<<D as DynamicAccount>::Header, <D as DynamicAccount>::Fixed, Vec<u8>>;
type DynamicAccessorRef<'a, D> =
    DynamicAccessor<&'a <D as DynamicAccount>::Header, &'a <D as DynamicAccount>::Fixed, &'a [u8]>;
type DynamicAccessorRefMut<'a, D> = DynamicAccessor<
    &'a mut <D as DynamicAccount>::Header,
    &'a mut <D as DynamicAccount>::Fixed,
    &'a mut [u8],
>;

pub type MangoAccountValue = DynamicAccessorValue<MangoAccount>;
pub type MangoAccountAcc<'a> = DynamicAccessorRef<'a, MangoAccount>;
pub type MangoAccountAccWithHeader<'a> =
    DynamicAccessor<MangoAccountDynamicHeader, &'a MangoAccountFixed, &'a [u8]>;
pub type MangoAccountAccMut<'a> = DynamicAccessorRefMut<'a, MangoAccount>;

impl MangoAccountValue {
    // bytes without discriminator
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (fixed, dynamic) = bytes.split_at(size_of::<MangoAccountFixed>());
        Ok(Self {
            fixed: *bytemuck::from_bytes(&fixed),
            header: MangoAccountDynamicHeader::from_bytes(dynamic)?,
            dynamic: dynamic.to_vec(),
        })
    }
}

impl<'a> MangoAccountAccWithHeader<'a> {
    // bytes without discriminator
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        let (fixed, dynamic) = bytes.split_at(size_of::<MangoAccountFixed>());
        Ok(Self {
            fixed: bytemuck::from_bytes(&fixed),
            header: MangoAccountDynamicHeader::from_bytes(dynamic)?,
            dynamic,
        })
    }
}

pub trait DerefOrBorrow<T: ?Sized> {
    fn deref_or_borrow(&self) -> &T;
}

impl<T: ?Sized> DerefOrBorrow<T> for T {
    fn deref_or_borrow(&self) -> &T {
        self
    }
}

impl<T: ?Sized> DerefOrBorrow<T> for &T {
    fn deref_or_borrow(&self) -> &T {
        self
    }
}

impl<T: Sized> DerefOrBorrow<[T]> for Vec<T> {
    fn deref_or_borrow(&self) -> &[T] {
        &self
    }
}

impl<T: ?Sized> DerefOrBorrow<T> for &mut T {
    fn deref_or_borrow(&self) -> &T {
        self
    }
}

impl<'a, T: ?Sized> DerefOrBorrow<T> for Ref<'a, T> {
    fn deref_or_borrow(&self) -> &T {
        &self
    }
}

impl<'a, T: ?Sized> DerefOrBorrow<T> for RefMut<'a, T> {
    fn deref_or_borrow(&self) -> &T {
        &self
    }
}

pub trait DerefOrBorrowMut<T: ?Sized> {
    fn deref_or_borrow_mut(&mut self) -> &mut T;
}

impl<T: ?Sized> DerefOrBorrowMut<T> for T {
    fn deref_or_borrow_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: ?Sized> DerefOrBorrowMut<T> for &mut T {
    fn deref_or_borrow_mut(&mut self) -> &mut T {
        self
    }
}

impl<'a, T: ?Sized> DerefOrBorrowMut<T> for RefMut<'a, T> {
    fn deref_or_borrow_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: Sized> DerefOrBorrowMut<[T]> for Vec<T> {
    fn deref_or_borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

// This generic impl covers MangoAccountAcc and MangoAccountAccMut
impl<
        Header: DerefOrBorrow<MangoAccountDynamicHeader>,
        Fixed: DerefOrBorrow<MangoAccountFixed>,
        Dynamic: DerefOrBorrow<[u8]>,
    > DynamicAccessor<Header, Fixed, Dynamic>
{
    fn header(&self) -> &MangoAccountDynamicHeader {
        self.header.deref_or_borrow()
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
    pub fn token_get(&self, token_index: TokenIndex) -> Result<(&TokenPosition, usize)> {
        self.token_iter()
            .enumerate()
            .find_map(|(raw_index, p)| p.is_active_for_token(token_index).then(|| (p, raw_index)))
            .ok_or_else(|| error_msg!("position for token index {} not found", token_index))
    }

    // get TokenPosition at raw_index
    pub fn token_get_raw(&self, raw_index: usize) -> &TokenPosition {
        get_helper(self.dynamic(), self.header().token_offset(raw_index))
    }

    // get iter over all TokenPositions (including inactive)
    pub fn token_iter(&self) -> impl Iterator<Item = &TokenPosition> + '_ {
        (0..self.header().token_count()).map(|i| self.token_get_raw(i))
    }

    // get iter over all active TokenPositions
    pub fn token_iter_active(&self) -> impl Iterator<Item = &TokenPosition> + '_ {
        (0..self.header().token_count())
            .map(|i| self.token_get_raw(i))
            .filter(|token| token.is_active())
    }

    pub fn token_find(&self, token_index: TokenIndex) -> Option<&TokenPosition> {
        self.token_iter_active()
            .find(|p| p.is_active_for_token(token_index))
    }

    // get Serum3Orders at raw_index
    pub fn serum3_get_raw(&self, raw_index: usize) -> &Serum3Orders {
        get_helper(self.dynamic(), self.header().serum3_offset(raw_index))
    }

    pub fn serum3_iter(&self) -> impl Iterator<Item = &Serum3Orders> + '_ {
        (0..self.header().serum3_count()).map(|i| self.serum3_get_raw(i))
    }

    pub fn serum3_iter_active(&self) -> impl Iterator<Item = &Serum3Orders> + '_ {
        (0..self.header().serum3_count())
            .map(|i| self.serum3_get_raw(i))
            .filter(|serum3_order| serum3_order.is_active())
    }

    pub fn serum3_find(&self, market_index: Serum3MarketIndex) -> Option<&Serum3Orders> {
        self.serum3_iter_active()
            .find(|p| p.is_active_for_market(market_index))
    }

    // get PerpPosition at raw_index
    pub fn perp_get_raw(&self, raw_index: usize) -> &PerpPositions {
        get_helper(self.dynamic(), self.header().perp_offset(raw_index))
    }

    pub fn perp_iter(&self) -> impl Iterator<Item = &PerpPositions> {
        (0..self.header().perp_count()).map(|i| self.perp_get_raw(i))
    }

    pub fn perp_iter_active_accounts(&self) -> impl Iterator<Item = &PerpPositions> {
        (0..self.header().perp_count())
            .map(|i| self.perp_get_raw(i))
            .filter(|p| p.is_active())
    }

    pub fn perp_find_account(&self, market_index: PerpMarketIndex) -> Option<&PerpPositions> {
        self.perp_iter_active_accounts()
            .find(|p| p.is_active_for_market(market_index))
    }

    pub fn perp_oo_get_raw(&self, raw_index: usize) -> &PerpOpenOrders {
        get_helper(self.dynamic(), self.header().perp_oo_offset(raw_index))
    }

    pub fn perp_oo_iter(&self) -> impl Iterator<Item = &PerpOpenOrders> {
        (0..self.header().perp_oo_count()).map(|i| self.perp_oo_get_raw(i))
    }

    pub fn perp_next_order_slot(&self) -> Option<usize> {
        self.perp_oo_iter()
            .position(|&oo| oo.order_market == FREE_ORDER_SLOT)
    }

    pub fn perp_find_order_with_client_order_id(
        &self,
        market_index: PerpMarketIndex,
        client_order_id: u64,
    ) -> Option<(i128, Side)> {
        for i in 0..self.header().perp_oo_count() {
            let oo = self.perp_oo_get_raw(i);
            if oo.order_market == market_index && oo.client_order_id == client_order_id {
                return Some((oo.order_id, oo.order_side));
            }
        }
        None
    }

    pub fn perp_find_order_side(
        &self,
        market_index: PerpMarketIndex,
        order_id: i128,
    ) -> Option<Side> {
        for i in 0..self.header().perp_oo_count() {
            let oo = self.perp_oo_get_raw(i);
            if oo.order_market == market_index && oo.order_id == order_id {
                return Some(oo.order_side);
            }
        }
        None
    }

    pub fn being_liquidated(&self) -> bool {
        self.fixed().being_liquidated()
    }

    pub fn is_bankrupt(&self) -> bool {
        self.fixed().is_bankrupt()
    }

    pub fn borrow<'b>(&'b self) -> DynamicAccessorRef<'b, MangoAccount> {
        DynamicAccessor {
            header: self.header(),
            fixed: self.fixed(),
            dynamic: self.dynamic(),
        }
    }

    pub fn size(&self) -> AccountSize {
        if self.header().perp_count() > 4 {
            return AccountSize::Large;
        }
        AccountSize::Small
    }
}

impl<
        Header: DerefOrBorrowMut<MangoAccountDynamicHeader> + DerefOrBorrow<MangoAccountDynamicHeader>,
        Fixed: DerefOrBorrowMut<MangoAccountFixed> + DerefOrBorrow<MangoAccountFixed>,
        Dynamic: DerefOrBorrowMut<[u8]> + DerefOrBorrow<[u8]>,
    > DynamicAccessor<Header, Fixed, Dynamic>
{
    fn header_mut(&mut self) -> &mut MangoAccountDynamicHeader {
        self.header.deref_or_borrow_mut()
    }
    fn dynamic_mut(&mut self) -> &mut [u8] {
        self.dynamic.deref_or_borrow_mut()
    }

    pub fn borrow_mut<'b>(&'b mut self) -> DynamicAccessorRefMut<'b, MangoAccount> {
        DynamicAccessor {
            header: self.header.deref_or_borrow_mut(),
            fixed: self.fixed.deref_or_borrow_mut(),
            dynamic: self.dynamic.deref_or_borrow_mut(),
        }
    }

    /// Returns
    /// - the position
    /// - the raw index into the token positions list (for use with get_raw/deactivate)
    pub fn token_get_mut(
        &mut self,
        token_index: TokenIndex,
    ) -> Result<(&mut TokenPosition, usize)> {
        let raw_index = self
            .token_iter()
            .enumerate()
            .find_map(|(raw_index, p)| p.is_active_for_token(token_index).then(|| raw_index))
            .ok_or_else(|| error_msg!("position for token index {} not found", token_index))?;
        Ok((self.token_get_mut_raw(raw_index), raw_index))
    }

    // get mut TokenPosition at raw_index
    pub fn token_get_mut_raw(&mut self, raw_index: usize) -> &mut TokenPosition {
        let offset = self.header().token_offset(raw_index);
        get_helper_mut(self.dynamic_mut(), offset)
    }

    /// Creates or retrieves a TokenPosition for the token_index.
    /// Returns:
    /// - the position
    /// - the raw index into the token positions list (for use with get_raw)
    /// - the active index, for use with FixedOrderAccountRetriever
    pub fn token_get_mut_or_create(
        &mut self,
        token_index: TokenIndex,
    ) -> Result<(&mut TokenPosition, usize, usize)> {
        let mut active_index = 0;
        let mut match_or_free = None;
        for (raw_index, position) in self.token_iter().enumerate() {
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
            let v = self.token_get_mut_raw(raw_index);
            if !v.is_active_for_token(token_index) {
                *v = TokenPosition {
                    indexed_position: I80F48::ZERO,
                    token_index,
                    in_use_count: 0,
                    reserved: Default::default(),
                };
            }
            Ok((v, raw_index, bank_index))
        } else {
            err!(MangoError::NoFreeTokenPositionIndex)
                .context(format!("when looking for token index {}", token_index))
        }
    }

    pub fn token_deactivate(&mut self, raw_index: usize) {
        assert!(self.token_get_mut_raw(raw_index).in_use_count == 0);
        self.token_get_mut_raw(raw_index).token_index = TokenIndex::MAX;
    }

    // get mut Serum3Orders at raw_index
    pub fn serum3_get_mut_raw(&mut self, raw_index: usize) -> &mut Serum3Orders {
        let offset = self.header().serum3_offset(raw_index);
        get_helper_mut(self.dynamic_mut(), offset)
    }

    pub fn serum3_create(&mut self, market_index: Serum3MarketIndex) -> Result<&mut Serum3Orders> {
        if self.serum3_find(market_index).is_some() {
            return err!(MangoError::Serum3OpenOrdersExistAlready);
        }

        let raw_index_opt = self.serum3_iter().position(|p| !p.is_active());
        if let Some(raw_index) = raw_index_opt {
            *(self.serum3_get_mut_raw(raw_index)) = Serum3Orders {
                market_index: market_index as Serum3MarketIndex,
                ..Serum3Orders::default()
            };
            return Ok(self.serum3_get_mut_raw(raw_index));
        } else {
            return err!(MangoError::NoFreeSerum3OpenOrdersIndex);
        }
    }

    pub fn serum3_deactivate(&mut self, market_index: Serum3MarketIndex) -> Result<()> {
        let raw_index = self
            .serum3_iter()
            .position(|p| p.is_active_for_market(market_index))
            .ok_or_else(|| error_msg!("serum3 open orders index {} not found", market_index))?;
        self.serum3_get_mut_raw(raw_index).market_index = Serum3MarketIndex::MAX;
        Ok(())
    }

    pub fn serum3_find_mut(
        &mut self,
        market_index: Serum3MarketIndex,
    ) -> Option<&mut Serum3Orders> {
        let raw_index_opt = self
            .serum3_iter_active()
            .position(|p| p.is_active_for_market(market_index));
        raw_index_opt.map(|raw_index| self.serum3_get_mut_raw(raw_index))
    }

    // get mut PerpPosition at raw_index
    pub fn perp_get_mut_raw(&mut self, raw_index: usize) -> &mut PerpPositions {
        let offset = self.header().perp_offset(raw_index);
        get_helper_mut(self.dynamic_mut(), offset)
    }

    pub fn perp_oo_get_mut_raw(&mut self, raw_index: usize) -> &mut PerpOpenOrders {
        let offset = self.header().perp_oo_offset(raw_index);
        get_helper_mut(self.dynamic_mut(), offset)
    }

    pub fn perp_get_account_mut_or_create(
        &mut self,
        perp_market_index: PerpMarketIndex,
    ) -> Result<(&mut PerpPositions, usize)> {
        let mut raw_index_opt = self
            .perp_iter_active_accounts()
            .position(|p| p.is_active_for_market(perp_market_index));
        if raw_index_opt.is_none() {
            raw_index_opt = self.perp_iter().position(|p| !p.is_active());
            if let Some(raw_index) = raw_index_opt {
                *(self.perp_get_mut_raw(raw_index)) = PerpPositions {
                    market_index: perp_market_index,
                    ..Default::default()
                };
            }
        }
        if let Some(raw_index) = raw_index_opt {
            Ok((self.perp_get_mut_raw(raw_index), raw_index))
        } else {
            err!(MangoError::NoFreePerpPositionIndex)
        }
    }

    pub fn perp_deactivate_account(&mut self, raw_index: usize) {
        self.perp_get_mut_raw(raw_index).market_index = PerpMarketIndex::MAX;
    }

    pub fn perp_add_order(
        &mut self,
        perp_market_index: PerpMarketIndex,
        side: Side,
        order: &LeafNode,
    ) -> Result<()> {
        let mut perp_account = self
            .perp_get_account_mut_or_create(perp_market_index)
            .unwrap()
            .0;
        match side {
            Side::Bid => {
                perp_account.bids_base_lots = cm!(perp_account.bids_base_lots + order.quantity);
            }
            Side::Ask => {
                perp_account.asks_base_lots = cm!(perp_account.asks_base_lots + order.quantity);
            }
        };
        let slot = order.owner_slot as usize;

        let mut oo = self.perp_oo_get_mut_raw(slot);
        oo.order_market = perp_market_index;
        oo.order_side = side;
        oo.order_id = order.key;
        oo.client_order_id = order.client_order_id;
        Ok(())
    }

    pub fn perp_remove_order(&mut self, slot: usize, quantity: i64) -> Result<()> {
        {
            let oo = self.perp_oo_get_mut_raw(slot);
            require_neq!(oo.order_market, FREE_ORDER_SLOT);
            let order_side = oo.order_side;
            let perp_market_index = oo.order_market;
            let perp_account = self
                .perp_get_account_mut_or_create(perp_market_index)
                .unwrap()
                .0;

            // accounting
            match order_side {
                Side::Bid => {
                    perp_account.bids_base_lots = cm!(perp_account.bids_base_lots - quantity);
                }
                Side::Ask => {
                    perp_account.asks_base_lots = cm!(perp_account.asks_base_lots - quantity);
                }
            }
        }

        // release space
        let oo = self.perp_oo_get_mut_raw(slot);
        oo.order_market = FREE_ORDER_SLOT;
        oo.order_side = Side::Bid;
        oo.order_id = 0i128;
        oo.client_order_id = 0u64;
        Ok(())
    }

    pub fn perp_execute_maker(
        &mut self,
        perp_market_index: PerpMarketIndex,
        perp_market: &mut PerpMarket,
        fill: &FillEvent,
    ) -> Result<()> {
        let pa = self
            .perp_get_account_mut_or_create(perp_market_index)
            .unwrap()
            .0;
        pa.settle_funding(perp_market);

        let side = fill.taker_side.invert_side();
        let (base_change, quote_change) = fill.base_quote_change(side);
        pa.change_base_position(perp_market, base_change);
        let quote = I80F48::from_num(
            perp_market
                .quote_lot_size
                .checked_mul(quote_change)
                .unwrap(),
        );
        let fees = quote.abs() * fill.maker_fee;
        if !fill.market_fees_applied {
            perp_market.fees_accrued += fees;
        }
        pa.quote_position_native = pa.quote_position_native.checked_add(quote - fees).unwrap();

        if fill.maker_out {
            self.perp_remove_order(fill.maker_slot as usize, base_change.abs())
        } else {
            match side {
                Side::Bid => {
                    pa.bids_base_lots = cm!(pa.bids_base_lots - base_change.abs());
                }
                Side::Ask => {
                    pa.asks_base_lots = cm!(pa.asks_base_lots - base_change.abs());
                }
            }
            Ok(())
        }
    }

    pub fn perp_execute_taker(
        &mut self,
        perp_market_index: PerpMarketIndex,
        perp_market: &mut PerpMarket,
        fill: &FillEvent,
    ) -> Result<()> {
        let pa = self
            .perp_get_account_mut_or_create(perp_market_index)
            .unwrap()
            .0;
        pa.settle_funding(perp_market);

        let (base_change, quote_change) = fill.base_quote_change(fill.taker_side);
        pa.remove_taker_trade(base_change, quote_change);
        pa.change_base_position(perp_market, base_change);
        let quote = I80F48::from_num(perp_market.quote_lot_size * quote_change);

        // fees are assessed at time of trade; no need to assess fees here

        pa.quote_position_native += quote;
        Ok(())
    }

    // writes length of tokens vec at appropriate offset so that borsh can infer the vector length
    // length used is that present in the header
    fn write_token_length(&mut self) {
        let tokens_offset = self.header().token_offset(0);
        // msg!(
        //     "writing tokens length at {}",
        //     tokens_offset - size_of::<BorshVecLength>()
        // );
        let count = self.header().token_count;
        let dst: &mut [u8] =
            &mut self.dynamic_mut()[tokens_offset - BORSH_VEC_SIZE_BYTES..tokens_offset];
        dst.copy_from_slice(&BorshVecLength::from(count).to_le_bytes());
    }

    fn write_serum3_length(&mut self) {
        let serum3_offset = self.header().serum3_offset(0);
        // msg!(
        //     "writing serum3 length at {}",
        //     serum3_offset - size_of::<BorshVecLength>()
        // );
        let count = self.header().serum3_count;
        let dst: &mut [u8] =
            &mut self.dynamic_mut()[serum3_offset - BORSH_VEC_SIZE_BYTES..serum3_offset];
        dst.copy_from_slice(&BorshVecLength::from(count).to_le_bytes());
    }

    fn write_perp_length(&mut self) {
        let perp_offset = self.header().perp_offset(0);
        // msg!(
        //     "writing perp length at {}",
        //     perp_offset - size_of::<BorshVecLength>()
        // );
        let count = self.header().perp_count;
        let dst: &mut [u8] =
            &mut self.dynamic_mut()[perp_offset - BORSH_VEC_SIZE_BYTES..perp_offset];
        dst.copy_from_slice(&BorshVecLength::from(count).to_le_bytes());
    }

    fn write_perp_oo_length(&mut self) {
        let perp_oo_offset = self.header().perp_oo_offset(0);
        // msg!(
        //     "writing perp length at {}",
        //     perp_offset - size_of::<BorshVecLength>()
        // );
        let count = self.header().perp_oo_count;
        let dst: &mut [u8] =
            &mut self.dynamic_mut()[perp_oo_offset - BORSH_VEC_SIZE_BYTES..perp_oo_offset];
        dst.copy_from_slice(&BorshVecLength::from(count).to_le_bytes());
    }

    pub fn expand_dynamic_content(&mut self, account_size: AccountSize) -> Result<()> {
        let (new_token_count, new_serum3_count, new_perp_count, new_perp_oo_count) =
            account_size.space();

        require_gt!(new_token_count, self.header().token_count);
        require_gt!(new_serum3_count, self.header().serum3_count);
        require_gt!(new_perp_count, self.header().perp_count);
        require_gt!(new_perp_oo_count, self.header().perp_oo_count);

        // create a temp copy to compute new starting offsets
        let new_header = MangoAccountDynamicHeader {
            token_count: new_token_count,
            serum3_count: new_serum3_count,
            perp_count: new_perp_count,
            perp_oo_count: new_perp_oo_count,
        };
        let old_header = self.header().clone();
        let dynamic = self.dynamic_mut();

        // expand dynamic components by first moving existing positions, and then setting new ones to defaults

        // perp oo
        unsafe {
            sol_memmove(
                &mut dynamic[new_header.perp_oo_offset(0)],
                &mut dynamic[old_header.perp_oo_offset(0)],
                size_of::<PerpOpenOrders>() * old_header.perp_oo_count(),
            );
        }
        for i in old_header.perp_oo_count..new_perp_oo_count {
            *get_helper_mut(dynamic, new_header.perp_oo_offset(i.into())) =
                PerpOpenOrders::default();
        }

        // perp positions
        unsafe {
            sol_memmove(
                &mut dynamic[new_header.perp_offset(0)],
                &mut dynamic[old_header.perp_offset(0)],
                size_of::<PerpPositions>() * old_header.perp_count(),
            );
        }
        for i in old_header.perp_count..new_perp_count {
            *get_helper_mut(dynamic, new_header.perp_offset(i.into())) = PerpPositions::default();
        }

        // serum3 positions
        unsafe {
            sol_memmove(
                &mut dynamic[new_header.serum3_offset(0)],
                &mut dynamic[old_header.serum3_offset(0)],
                size_of::<Serum3Orders>() * old_header.serum3_count(),
            );
        }
        for i in old_header.serum3_count..new_serum3_count {
            *get_helper_mut(dynamic, new_header.serum3_offset(i.into())) = Serum3Orders::default();
        }

        // token positions
        unsafe {
            sol_memmove(
                &mut dynamic[new_header.token_offset(0)],
                &mut dynamic[old_header.token_offset(0)],
                size_of::<TokenPosition>() * old_header.token_count(),
            );
        }
        for i in old_header.token_count..new_token_count {
            *get_helper_mut(dynamic, new_header.token_offset(i.into())) = TokenPosition::default();
        }

        // update header
        let header_mut = self.header_mut();
        header_mut.token_count = new_token_count;
        header_mut.serum3_count = new_serum3_count;
        header_mut.perp_count = new_perp_count;
        header_mut.perp_oo_count = new_perp_oo_count;

        // write new lengths (uses header)
        self.write_token_length();
        self.write_serum3_length();
        self.write_perp_length();
        self.write_perp_oo_length();

        Ok(())
    }
}

impl Header for MangoAccountDynamicHeader {
    fn from_bytes(data: &[u8]) -> Result<Self> {
        let token_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
            data,
            MangoAccount::dynamic_token_vec_offset(),
            BORSH_VEC_SIZE_BYTES
        ]))
        .unwrap();

        let serum3_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
            data,
            MangoAccount::dynamic_serum3_vec_offset(token_count),
            BORSH_VEC_SIZE_BYTES
        ]))
        .unwrap();

        let perp_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
            data,
            MangoAccount::dynamic_perp_vec_offset(token_count, serum3_count),
            BORSH_VEC_SIZE_BYTES
        ]))
        .unwrap();

        let perp_oo_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
            data,
            MangoAccount::dynamic_perp_oo_vec_offset(token_count, serum3_count, perp_count),
            BORSH_VEC_SIZE_BYTES
        ]))
        .unwrap();

        Ok(Self {
            token_count,
            serum3_count,
            perp_count,
            perp_oo_count,
        })
    }

    fn initialize(_data: &mut [u8]) -> Result<()> {
        Ok(())
    }
}

pub struct AccountLoaderDynamic<'info, D: DynamicAccount> {
    acc_info: AccountInfo<'info>,
    phantom1: PhantomData<&'info D>,
}

impl<'info, D: DynamicAccount> AccountLoaderDynamic<'info, D> {
    pub fn try_from(acc_info: &AccountInfo<'info>) -> Result<Self> {
        if acc_info.owner != &D::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*acc_info.owner, D::owner())));
        }

        let data = acc_info.try_borrow_data()?;
        if data.len() < D::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &D::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(Self {
            acc_info: acc_info.clone(),
            phantom1: PhantomData,
        })
    }

    pub fn try_from_unchecked(acc_info: &AccountInfo<'info>) -> Result<Self> {
        if acc_info.owner != &D::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*acc_info.owner, D::owner())));
        }
        Ok(Self {
            acc_info: acc_info.clone(),
            phantom1: PhantomData,
        })
    }

    /// Returns a Ref to the account data structure for reading.
    pub fn load_fixed<'a>(&'a self) -> Result<Ref<'a, D::Fixed>> {
        let data = self.acc_info.try_borrow_data()?;
        let fixed = Ref::map(data, |d| {
            bytemuck::from_bytes(&d[8..8 + size_of::<D::Fixed>()])
        });
        Ok(fixed)
    }

    /// Returns a Ref to the account data structure for reading.
    pub fn load<'a>(
        &'a self,
    ) -> Result<DynamicAccessor<D::Header, Ref<'a, D::Fixed>, Ref<'a, [u8]>>> {
        let data = self.acc_info.try_borrow_data()?;
        let header = D::Header::from_bytes(&data[8 + size_of::<D::Fixed>()..])?;
        let (_, data) = Ref::map_split(data, |d| d.split_at(8));
        let (fixed_bytes, dynamic) = Ref::map_split(data, |d| d.split_at(size_of::<D::Fixed>()));
        Ok(DynamicAccessor {
            header,
            fixed: Ref::map(fixed_bytes, |b| bytemuck::from_bytes(b)),
            dynamic,
        })
    }

    pub fn load_init<'a>(
        &'a self,
    ) -> Result<DynamicAccessor<D::Header, RefMut<'a, D::Fixed>, RefMut<'a, [u8]>>> {
        if !self.acc_info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let mut data = self.acc_info.try_borrow_mut_data()?;
        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        let discriminator = u64::from_le_bytes(disc_bytes);
        if discriminator != 0 {
            return Err(ErrorCode::AccountDiscriminatorAlreadySet.into());
        }

        let disc_bytes: &mut [u8] = &mut data[0..8];
        disc_bytes.copy_from_slice(bytemuck::bytes_of(&(D::discriminator())));

        D::Header::initialize(&mut data[8 + size_of::<D::Fixed>()..])?;

        drop(data);

        self.load_mut()
    }

    /// Returns a Ref to the account data structure for reading.
    pub fn load_mut<'a>(
        &'a self,
    ) -> Result<DynamicAccessor<D::Header, RefMut<'a, D::Fixed>, RefMut<'a, [u8]>>> {
        if !self.acc_info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data = self.acc_info.try_borrow_mut_data()?;
        let header = D::Header::from_bytes(&data[8 + size_of::<D::Fixed>()..])?;
        let (_, data) = RefMut::map_split(data, |d| d.split_at_mut(8));
        let (fixed_bytes, dynamic) =
            RefMut::map_split(data, |d| d.split_at_mut(size_of::<D::Fixed>()));
        Ok(DynamicAccessor {
            header,
            fixed: RefMut::map(fixed_bytes, |b| bytemuck::from_bytes_mut(b)),
            dynamic,
        })
    }
}

impl<'info, D: DynamicAccount> anchor_lang::Accounts<'info> for AccountLoaderDynamic<'info, D> {
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut std::collections::BTreeMap<String, u8>,
        _reallocs: &mut std::collections::BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        let l = AccountLoaderDynamic::try_from(account)?;
        Ok(l)
    }
}

impl<'info, D: DynamicAccount> anchor_lang::AccountsExit<'info> for AccountLoaderDynamic<'info, D> {
    fn exit(&self, _program_id: &Pubkey) -> Result<()> {
        // Normally anchor writes the discriminator again here, but I don't see why
        let data = self.acc_info.try_borrow_data()?;
        if data.len() < D::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &D::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }
        Ok(())
    }
}

impl<'info, D: DynamicAccount> anchor_lang::AccountsClose<'info>
    for AccountLoaderDynamic<'info, D>
{
    fn close(&self, sol_destination: AccountInfo<'info>) -> Result<()> {
        close(self.to_account_info(), sol_destination)
    }
}

impl<'info, D: DynamicAccount> anchor_lang::ToAccountMetas for AccountLoaderDynamic<'info, D> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.acc_info.is_signer);
        let meta = match self.acc_info.is_writable {
            false => AccountMeta::new_readonly(*self.acc_info.key, is_signer),
            true => AccountMeta::new(*self.acc_info.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info, D: DynamicAccount> AsRef<AccountInfo<'info>> for AccountLoaderDynamic<'info, D> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.acc_info
    }
}

impl<'info, D: DynamicAccount> anchor_lang::ToAccountInfos<'info>
    for AccountLoaderDynamic<'info, D>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.acc_info.clone()]
    }
}

impl<'info, D: DynamicAccount> anchor_lang::Key for AccountLoaderDynamic<'info, D> {
    fn key(&self) -> Pubkey {
        *self.acc_info.key
    }
}

// https://github.com/coral-xyz/anchor/blob/master/lang/src/common.rs#L8
fn close<'info>(info: AccountInfo<'info>, sol_destination: AccountInfo<'info>) -> Result<()> {
    // Transfer tokens from the account to the sol_destination.
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() =
        dest_starting_lamports.checked_add(info.lamports()).unwrap();
    **info.lamports.borrow_mut() = 0;
    // Mark the account discriminator as closed.
    let mut data = info.try_borrow_mut_data()?;
    let dst: &mut [u8] = &mut data;
    dst[0..8].copy_from_slice(&[255, 255, 255, 255, 255, 255, 255, 255]);

    Ok(())
}
