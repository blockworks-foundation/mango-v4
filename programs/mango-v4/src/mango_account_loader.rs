//! Type facilitating on demand zero copy deserialization.

use anchor_lang::error::{Error, ErrorCode};
use anchor_lang::{
    require_eq, require_gt, Accounts, AccountsClose, AccountsExit, Key, Owner, Result,
    ToAccountInfo, ToAccountInfos, ToAccountMetas, ZeroCopy,
};
use arrayref::array_ref;
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::msg;
use solana_program::pubkey::Pubkey;
use std::cell::{Ref, RefMut};
use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::marker::PhantomData;
use std::mem::{self, size_of};
use std::ops::DerefMut;

pub trait Header: Sized {
    // Parses a header struct from account bytes. The bytes that come after the
    // fixed-layout part will be passed in.
    fn try_new_header(data: &[u8]) -> Result<Self>;
}

pub trait GetAccessor<'a> {
    type Accessor;
    // Create a non-mut accessor from a header and some bytes.
    // Note that the same array is passed in as for try_new_header() -- you need to
    // skip the header bytes yourself.
    fn new_accessor(header: &'a Self, data: Ref<'a, [u8]>) -> Self::Accessor;
}

pub trait GetAccessorMut<'a> {
    type AccessorMut;
    // same as above, but for mut accessors
    fn new_accessor_mut(header: &'a Self, data: RefMut<'a, [u8]>) -> Self::AccessorMut;
}

#[derive(Copy, Clone)]
struct ExampleTokenPosition {
    a: u16,
    b: u8,
}

unsafe impl bytemuck::Zeroable for ExampleTokenPosition {}
unsafe impl bytemuck::Pod for ExampleTokenPosition {}

// This header struct gets created when MangoAccountLoader is created from an AccountInfo,
// Store things here that would be too expensive or inconvenient to recompute on every load().
struct ExampleDynamicHeader {
    header_size: u8,
    token_count: u8,
    serum3_count: u8,
}

pub fn get_helper<T: bytemuck::Pod>(data: &[u8], index: usize) -> &T {
    bytemuck::from_bytes(&data[index..index + mem::size_of::<T>()])
}
pub fn get_helper_mut<T: bytemuck::Pod>(data: &mut [u8], index: usize) -> &mut T {
    msg!("62");
    msg!("data {}", data.len());
    msg!("size of T {}", size_of::<T>());
    msg!("index {}", index);
    bytemuck::from_bytes_mut(&mut data[index..index + mem::size_of::<T>()])
}

// Since we need to implement two accessors: one for non-mut and one for mut accesses
// (and the mut one needs to reimplement all non-mut functions a second time...), it's
// convenient to put most functions here.
impl ExampleDynamicHeader {
    fn token_offset(&self, raw_index: usize) -> usize {
        self.header_size as usize + raw_index * mem::size_of::<ExampleTokenPosition>()
    }
    fn token_get_raw<'a>(&self, data: &'a [u8], raw_index: usize) -> &'a ExampleTokenPosition {
        get_helper(data, self.token_offset(raw_index))
    }
    fn token_get_raw_mut<'a>(
        &self,
        data: &'a mut [u8],
        raw_index: usize,
    ) -> &'a mut ExampleTokenPosition {
        get_helper_mut(data, self.token_offset(raw_index))
    }
    fn token_iter<'a>(
        &'a self,
        data: &'a [u8],
    ) -> impl Iterator<Item = &'a ExampleTokenPosition> + '_ {
        (0..self.token_count as usize).map(|i| self.token_get_raw(data, i))
    }
}

// non-mut accessor
struct ExampleDynamicAccessor<'a> {
    header: &'a ExampleDynamicHeader,
    data: Ref<'a, [u8]>,
}
impl<'a> ExampleDynamicAccessor<'a> {
    fn token_get_raw(&self, raw_index: usize) -> &ExampleTokenPosition {
        self.header.token_get_raw(&self.data, raw_index)
    }
}

// mut accessor
struct ExampleDynamicAccessorMut<'a> {
    header: &'a ExampleDynamicHeader,
    data: RefMut<'a, [u8]>,
}
impl<'a> ExampleDynamicAccessorMut<'a> {
    // it's sad, but need to re-implement the non-mut interface here
    fn token_get_raw(&self, raw_index: usize) -> &ExampleTokenPosition {
        self.header.token_get_raw(&self.data, raw_index)
    }

    fn token_get_raw_mut(&mut self, raw_index: usize) -> &mut ExampleTokenPosition {
        self.header.token_get_raw_mut(&mut self.data, raw_index)
    }
    fn token_iter(&self) -> impl Iterator<Item = &ExampleTokenPosition> + '_ {
        self.header.token_iter(&self.data)
    }
}

impl Header for ExampleDynamicHeader {
    fn try_new_header(data: &[u8]) -> Result<Self> {
        let header_version = data[0];
        msg!("header_version {:?}", header_version);
        match header_version {
            0 => Ok(Self {
                header_size: 2,
                token_count: data[1],
                serum3_count: data[2],
            }),
            _ => Err(ErrorCode::AccountDiscriminatorNotFound.into()),
        }
    }
}

impl<'a> GetAccessor<'a> for ExampleDynamicHeader {
    type Accessor = ExampleDynamicAccessor<'a>;
    fn new_accessor(header: &'a Self, data: Ref<'a, [u8]>) -> Self::Accessor {
        Self::Accessor { header, data }
    }
}

impl<'a> GetAccessorMut<'a> for ExampleDynamicHeader {
    type AccessorMut = ExampleDynamicAccessorMut<'a>;
    fn new_accessor_mut(header: &'a Self, data: RefMut<'a, [u8]>) -> Self::AccessorMut {
        Self::AccessorMut { header, data }
    }
}

fn testfn() {
    let mut data_buf = [5u8, 7, 10, 13, 45, 1, 12];
    let mut data = std::cell::RefCell::new(&mut data_buf[..]);
    let header = ExampleDynamicHeader {
        header_size: 2,
        token_count: 3,
        serum3_count: 0,
    };
    let r = data.borrow_mut();
    let (mut l, r) = RefMut::map_split(r, |r| r.split_at_mut(1));
    let mut mutacc = ExampleDynamicAccessorMut {
        header: &header,
        data: r,
    };

    mutacc.token_get_raw_mut(0);
    mutacc.token_get_raw(0);

    mutacc.token_iter().map(|v| v.a).collect::<Vec<_>>();
}

pub struct SplitAccount<T, U> {
    pub fixed: T,
    pub dynamic: U,
}

#[derive(Clone)]
pub struct MangoAccountLoader<'info, T: ZeroCopy + Owner, U: Header> {
    acc_info: AccountInfo<'info>,
    header: U,
    phantom: PhantomData<&'info T>,
}

impl<'info, T: ZeroCopy + Owner + fmt::Debug, U: Header> fmt::Debug
    for MangoAccountLoader<'info, T, U>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountLoader")
            .field("acc_info", &self.acc_info)
            .field("phantom", &self.phantom)
            .finish()
    }
}

impl<'info, T: ZeroCopy + Owner, U: Header> MangoAccountLoader<'info, T, U> {
    fn new(acc_info: AccountInfo<'info>, header: U) -> MangoAccountLoader<'info, T, U> {
        Self {
            acc_info,
            header,
            phantom: PhantomData,
        }
    }

    /// Constructs a new `Loader` from a previously initialized account.
    #[inline(never)]
    pub fn try_from(acc_info: &AccountInfo<'info>) -> Result<MangoAccountLoader<'info, T, U>> {
        if acc_info.owner != &T::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*acc_info.owner, T::owner())));
        }
        let data: &[u8] = &acc_info.try_borrow_data()?;
        msg!("data{:?}", data);
        msg!("data.len {:?}", data.len());
        msg!("T::discriminator().len() {:?}", T::discriminator().len());
        if data.len() < T::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        msg!("T::discriminator().len() {:?}", T::discriminator().len());
        // Discriminator must match.
        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        msg!("223");
        let dynamic_bytes = &data[8 + mem::size_of::<T>()..];
        let header = U::try_new_header(dynamic_bytes)?;

        msg!("227");
        Ok(MangoAccountLoader::new(acc_info.clone(), header))
    }

    /// Constructs a new `Loader` from an uninitialized account.
    #[inline(never)]
    pub fn try_from_unchecked(
        _program_id: &Pubkey,
        acc_info: &AccountInfo<'info>,
    ) -> Result<MangoAccountLoader<'info, T, U>> {
        if acc_info.owner != &T::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*acc_info.owner, T::owner())));
        }

        let data: &[u8] = &acc_info.try_borrow_data()?;
        msg!("data{:?}", data);
        msg!("data.len {:?}", data.len());
        let dynamic_bytes = &data[8 + mem::size_of::<T>()..];
        msg!("dynamic_bytes.len {:?}", dynamic_bytes.len());
        let header = U::try_new_header(dynamic_bytes)?;

        Ok(MangoAccountLoader::new(acc_info.clone(), header))
    }

    /// Returns a Ref to the account data structure for reading.
    pub fn load<'a>(&'a self) -> Result<SplitAccount<Ref<T>, U::Accessor>>
    where
        U: GetAccessor<'a>,
    {
        let data = self.acc_info.try_borrow_data()?;
        if data.len() < T::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }

        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        let (fixed, dynamic) = Ref::map_split(data, |data| {
            let (fixed_slice, dynamic_slice) = data.split_at(8 + mem::size_of::<T>());
            (bytemuck::from_bytes(fixed_slice), dynamic_slice)
        });
        Ok(SplitAccount {
            fixed,
            dynamic: U::new_accessor(&self.header, dynamic),
        })
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    pub fn load_mut<'a>(&'a self) -> Result<SplitAccount<RefMut<T>, U::AccessorMut>>
    where
        U: GetAccessorMut<'a>,
    {
        // AccountInfo api allows you to borrow mut even if the account isn't
        // writable, so add this check for a better dev experience.
        if !self.acc_info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data = self.acc_info.try_borrow_mut_data()?;
        msg!("data.len {:?}", data.len());
        if data.len() < T::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        msg!("data.len {:?}", data.len());

        let disc_bytes = array_ref![data, 0, 8];
        msg!("{:?}", disc_bytes);
        msg!("{:?}", &T::discriminator());
        msg!("size of  T{:?}", size_of::<T>());
        if disc_bytes != &T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        let (fixed, dynamic) = RefMut::map_split(data, |data| {
            let (fixed_slice, dynamic_slice) = data.split_at_mut(8 + mem::size_of::<T>());
            msg!("fixed_slice.len {:?}", fixed_slice.len());
            msg!("dynamic_slice.len {:?}", dynamic_slice.len());
            let (_, fixed_slice) = fixed_slice.split_at_mut(8);
            msg!("fixed_slice.len {:?}", fixed_slice.len());
            (bytemuck::from_bytes_mut(fixed_slice), dynamic_slice)
        });
        msg!("311");
        Ok(SplitAccount {
            fixed,
            dynamic: U::new_accessor_mut(&self.header, dynamic),
        })
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    /// Should only be called once, when the account is being initialized.
    pub fn load_init(&self) -> Result<RefMut<T>> {
        // AccountInfo api allows you to borrow mut even if the account isn't
        // writable, so add this check for a better dev experience.
        if !self.acc_info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data = self.acc_info.try_borrow_mut_data()?;

        // The discriminator should be zero, since we're initializing.
        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        let discriminator = u64::from_le_bytes(disc_bytes);
        if discriminator != 0 {
            return Err(ErrorCode::AccountDiscriminatorAlreadySet.into());
        }

        Ok(RefMut::map(data, |data| {
            bytemuck::from_bytes_mut(&mut data.deref_mut()[8..mem::size_of::<T>() + 8])
        }))
    }
}

impl<'info, T: ZeroCopy + Owner, U: Header> Accounts<'info> for MangoAccountLoader<'info, T, U> {
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut BTreeMap<String, u8>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        let l = MangoAccountLoader::try_from(account)?;
        Ok(l)
    }
}

impl<'info, T: ZeroCopy + Owner, U: Header> AccountsExit<'info>
    for MangoAccountLoader<'info, T, U>
{
    // The account *cannot* be loaded when this is called.
    fn exit(&self, _program_id: &Pubkey) -> Result<()> {
        let mut data = self.acc_info.try_borrow_mut_data()?;
        let dst: &mut [u8] = &mut data;

        // TODO: copy_from_slice?
        //let mut writer = BpfWriter::new(dst);
        //writer.write_all(&T::discriminator()).unwrap();
        Ok(())
    }
}

/// This function is for INTERNAL USE ONLY.
/// Do NOT use this function in a program.
/// Manual closing of `AccountLoader<'info, T>` types is NOT supported.
///
/// Details: Using `close` with `AccountLoader<'info, T>` is not safe because
/// it requires the `mut` constraint but for that type the constraint
/// overwrites the "closed account" discriminator at the end of the instruction.
impl<'info, T: ZeroCopy + Owner, U: Header> AccountsClose<'info>
    for MangoAccountLoader<'info, T, U>
{
    fn close(&self, sol_destination: AccountInfo<'info>) -> Result<()> {
        // TODO
        //crate::common::close(self.to_account_info(), sol_destination)
        Ok(())
    }
}

impl<'info, T: ZeroCopy + Owner, U: Header> ToAccountMetas for MangoAccountLoader<'info, T, U> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.acc_info.is_signer);
        let meta = match self.acc_info.is_writable {
            false => AccountMeta::new_readonly(*self.acc_info.key, is_signer),
            true => AccountMeta::new(*self.acc_info.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info, T: ZeroCopy + Owner, U: Header> AsRef<AccountInfo<'info>>
    for MangoAccountLoader<'info, T, U>
{
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.acc_info
    }
}

impl<'info, T: ZeroCopy + Owner, U: Header> ToAccountInfos<'info>
    for MangoAccountLoader<'info, T, U>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.acc_info.clone()]
    }
}

impl<'info, T: ZeroCopy + Owner, U: Header> Key for MangoAccountLoader<'info, T, U> {
    fn key(&self) -> Pubkey {
        *self.acc_info.key
    }
}
