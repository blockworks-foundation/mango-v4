use std::cell::{Ref, RefMut};

use std::marker::PhantomData;
use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use arrayref::array_ref;

// Header is created by scanning and parsing dynamic portion of the account
// Header stores useful information e.g. offsets to easily seek into dynamic content
pub trait DynamicHeader: Sized {
    // build header by scanning and parsing dynamic portion of the account
    fn from_bytes(data: &[u8]) -> Result<Self>;

    // initialize a header on a new account, if necessary
    fn initialize(data: &mut [u8]) -> Result<()>;
}

pub trait DynamicAccountType: Owner + Discriminator {
    type Header: DynamicHeader;
    type Fixed: bytemuck::Pod;
}

#[derive(Clone)]
pub struct DynamicAccount<Header, Fixed, Dynamic> {
    pub header: Header,
    pub fixed: Fixed,
    pub dynamic: Dynamic,
}

pub type DynamicAccountValue<D> =
    DynamicAccount<<D as DynamicAccountType>::Header, <D as DynamicAccountType>::Fixed, Vec<u8>>;
pub type DynamicAccountRef<'a, D> = DynamicAccount<
    &'a <D as DynamicAccountType>::Header,
    &'a <D as DynamicAccountType>::Fixed,
    &'a [u8],
>;
pub type DynamicAccountRefMut<'a, D> = DynamicAccount<
    &'a mut <D as DynamicAccountType>::Header,
    &'a mut <D as DynamicAccountType>::Fixed,
    &'a mut [u8],
>;

// Want to generalize over:
// - T (which is Borrow<T>)
// - &T (which is Borrow<T> and Deref<Target=T>)
// - Ref<T> (which is Deref<T>)
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
        self
    }
}

impl<T: ?Sized> DerefOrBorrow<T> for &mut T {
    fn deref_or_borrow(&self) -> &T {
        self
    }
}

impl<'a, T: ?Sized> DerefOrBorrow<T> for Ref<'a, T> {
    fn deref_or_borrow(&self) -> &T {
        self
    }
}

impl<'a, T: ?Sized> DerefOrBorrow<T> for RefMut<'a, T> {
    fn deref_or_borrow(&self) -> &T {
        self
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

pub struct AccountLoaderDynamic<'info, D: DynamicAccountType> {
    /// CHECK: is checked below
    acc_info: AccountInfo<'info>,
    phantom1: PhantomData<&'info D>,
}

impl<'info, D: DynamicAccountType> AccountLoaderDynamic<'info, D> {
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
    pub fn load_fixed(&self) -> Result<Ref<D::Fixed>> {
        let data = self.acc_info.try_borrow_data()?;
        let fixed = Ref::map(data, |d| {
            bytemuck::from_bytes(&d[8..8 + size_of::<D::Fixed>()])
        });
        Ok(fixed)
    }

    #[allow(clippy::type_complexity)]
    /// Returns a Ref to the account data structure for reading.
    pub fn load(&self) -> Result<DynamicAccount<D::Header, Ref<D::Fixed>, Ref<[u8]>>> {
        let data = self.acc_info.try_borrow_data()?;
        let header = D::Header::from_bytes(&data[8 + size_of::<D::Fixed>()..])?;
        let (_, data) = Ref::map_split(data, |d| d.split_at(8));
        let (fixed_bytes, dynamic) = Ref::map_split(data, |d| d.split_at(size_of::<D::Fixed>()));
        Ok(DynamicAccount {
            header,
            fixed: Ref::map(fixed_bytes, |b| bytemuck::from_bytes(b)),
            dynamic,
        })
    }

    #[allow(clippy::type_complexity)]
    pub fn load_init(&self) -> Result<DynamicAccount<D::Header, RefMut<D::Fixed>, RefMut<[u8]>>> {
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
    #[allow(clippy::type_complexity)]
    pub fn load_mut(&self) -> Result<DynamicAccount<D::Header, RefMut<D::Fixed>, RefMut<[u8]>>> {
        if !self.acc_info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data = self.acc_info.try_borrow_mut_data()?;
        let header = D::Header::from_bytes(&data[8 + size_of::<D::Fixed>()..])?;
        let (_, data) = RefMut::map_split(data, |d| d.split_at_mut(8));
        let (fixed_bytes, dynamic) =
            RefMut::map_split(data, |d| d.split_at_mut(size_of::<D::Fixed>()));
        Ok(DynamicAccount {
            header,
            fixed: RefMut::map(fixed_bytes, |b| bytemuck::from_bytes_mut(b)),
            dynamic,
        })
    }
}

impl<'info, D: DynamicAccountType> anchor_lang::Accounts<'info> for AccountLoaderDynamic<'info, D> {
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

impl<'info, D: DynamicAccountType> anchor_lang::AccountsExit<'info>
    for AccountLoaderDynamic<'info, D>
{
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

impl<'info, D: DynamicAccountType> anchor_lang::AccountsClose<'info>
    for AccountLoaderDynamic<'info, D>
{
    fn close(&self, sol_destination: AccountInfo<'info>) -> Result<()> {
        close(self.to_account_info(), sol_destination)
    }
}

impl<'info, D: DynamicAccountType> anchor_lang::ToAccountMetas for AccountLoaderDynamic<'info, D> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.acc_info.is_signer);
        let meta = match self.acc_info.is_writable {
            false => AccountMeta::new_readonly(*self.acc_info.key, is_signer),
            true => AccountMeta::new(*self.acc_info.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info, D: DynamicAccountType> AsRef<AccountInfo<'info>> for AccountLoaderDynamic<'info, D> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.acc_info
    }
}

impl<'info, D: DynamicAccountType> anchor_lang::ToAccountInfos<'info>
    for AccountLoaderDynamic<'info, D>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.acc_info.clone()]
    }
}

impl<'info, D: DynamicAccountType> anchor_lang::Key for AccountLoaderDynamic<'info, D> {
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
