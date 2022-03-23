use anchor_lang::prelude::*;
use anchor_lang::ZeroCopy;
use arrayref::array_ref;
use std::cell::RefMut;
use std::{cell::Ref, mem};

#[macro_export]
macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.zip(
            zip!($($y), +))
    )
}
#[allow(unused_imports)]
pub(crate) use zip;

#[macro_export]
macro_rules! checked_math {
    ($x: expr) => {
        checked_math::checked_math!($x).unwrap_or_else(|| panic!("math error"))
    };
}
pub(crate) use checked_math;

pub trait LoadZeroCopy {
    /// Using AccountLoader forces a AccountInfo.clone() and then binds the loaded
    /// lifetime to the AccountLoader's lifetime. This function avoids both.
    /// It checks the account owner and discriminator, then casts the data.
    fn load<T: ZeroCopy + Owner>(&self) -> Result<Ref<T>>;

    /// Same as load(), but mut
    fn load_mut<T: ZeroCopy + Owner>(&self) -> Result<RefMut<T>>;

    /// Same as load(), but doesn't check the discriminator.
    fn load_unchecked<T: ZeroCopy + Owner>(&self) -> Result<Ref<T>>;

    /// Same as load_unchecked(), but mut
    fn load_unchecked_mut<T: ZeroCopy + Owner>(&self) -> Result<RefMut<T>>;
}

impl<'info> LoadZeroCopy for AccountInfo<'info> {
    fn load_mut<T: ZeroCopy + Owner>(&self) -> Result<RefMut<T>> {
        if self.owner != &T::owner() {
            return Err(ErrorCode::AccountOwnedByWrongProgram.into());
        }

        let data = self.try_borrow_mut_data()?;

        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(RefMut::map(data, |data| {
            bytemuck::from_bytes_mut(&mut data[8..mem::size_of::<T>() + 8])
        }))
    }

    fn load<T: ZeroCopy + Owner>(&self) -> Result<Ref<T>> {
        if self.owner != &T::owner() {
            return Err(ErrorCode::AccountOwnedByWrongProgram.into());
        }

        let data = self.try_borrow_data()?;

        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(Ref::map(data, |data| {
            bytemuck::from_bytes(&data[8..mem::size_of::<T>() + 8])
        }))
    }

    fn load_unchecked_mut<T: ZeroCopy + Owner>(&self) -> Result<RefMut<T>> {
        if self.owner != &T::owner() {
            return Err(ErrorCode::AccountOwnedByWrongProgram.into());
        }

        let data = self.try_borrow_mut_data()?;

        Ok(RefMut::map(data, |data| {
            bytemuck::from_bytes_mut(&mut data[8..mem::size_of::<T>() + 8])
        }))
    }

    fn load_unchecked<T: ZeroCopy + Owner>(&self) -> Result<Ref<T>> {
        if self.owner != &T::owner() {
            return Err(ErrorCode::AccountOwnedByWrongProgram.into());
        }

        let data = self.try_borrow_data()?;

        Ok(Ref::map(data, |data| {
            bytemuck::from_bytes(&data[8..mem::size_of::<T>() + 8])
        }))
    }
}
