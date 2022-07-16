use std::{
    cell::{Ref, RefMut},
    mem::{self, size_of},
};

use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;

use crate::mango_account_loader::{
    get_helper, get_helper_mut, GetAccessor, GetAccessorMut, Header,
};

use super::TokenPosition;

#[account(zero_copy)]
pub struct EMTestAccount {
    pub owner: Pubkey,
}

impl EMTestAccount {}

#[derive(Clone, Copy)]
pub struct EMTestAccountHeader {
    // pub header_version: u8,
    pub header_size: u8,
    pub token_count: u8,
    // pub token: TokenPosition,
}

// const_assert_eq!(size_of::<EMTestAccountHeader>(), 32);
// const_assert_eq!(size_of::<EMTestAccountHeader>() % 8, 0);

impl EMTestAccountHeader {
    fn token_offset(&self, raw_index: usize) -> usize {
        1 + self.header_size as usize + 6 + raw_index * mem::size_of::<TokenPosition>()
    }
    fn token_get_raw<'a>(&self, data: &'a [u8], raw_index: usize) -> &'a TokenPosition {
        get_helper(data, self.token_offset(raw_index))
    }
    fn token_get_raw_mut<'a>(&self, data: &'a mut [u8], raw_index: usize) -> &'a mut TokenPosition {
        get_helper_mut(data, self.token_offset(raw_index))
    }
    fn token_iter<'a>(&'a self, data: &'a [u8]) -> impl Iterator<Item = &'a TokenPosition> + '_ {
        (0..self.token_count as usize).map(|i| self.token_get_raw(data, i))
    }
}

unsafe impl bytemuck::Pod for EMTestAccountHeader {}
unsafe impl bytemuck::Zeroable for EMTestAccountHeader {}

pub struct EMTestAccountDynamicAccessor<'a> {
    header: &'a EMTestAccountHeader,
    data: Ref<'a, [u8]>,
}
impl<'a> EMTestAccountDynamicAccessor<'a> {
    fn token_get_raw(&self, raw_index: usize) -> &TokenPosition {
        self.header.token_get_raw(&self.data, raw_index)
    }
}

pub struct EMTestAccountDynamicAccessorMut<'a> {
    header: &'a EMTestAccountHeader,
    data: RefMut<'a, [u8]>,
}
impl<'a> EMTestAccountDynamicAccessorMut<'a> {
    // it's sad, but need to re-implement the non-mut interface here
    pub fn token_get_raw(&self, raw_index: usize) -> &TokenPosition {
        self.header.token_get_raw(&self.data, raw_index)
    }

    pub fn token_get_raw_mut(&mut self, raw_index: usize) -> &mut TokenPosition {
        self.header.token_get_raw_mut(&mut self.data, raw_index)
    }
    pub fn token_iter(&self) -> impl Iterator<Item = &TokenPosition> + '_ {
        self.header.token_iter(&self.data)
    }
}

impl Header for EMTestAccountHeader {
    fn try_new_header(data: &[u8]) -> Result<Self> {
        let header_version = data[0];
        msg!("header_version {:?}", header_version);
        match header_version {
            0 => Ok(Self {
                header_size: 1,
                token_count: data[1],
            }),
            _ => Err(ErrorCode::AccountDiscriminatorNotFound.into()),
        }
    }
}

impl<'a> GetAccessor<'a> for EMTestAccountHeader {
    type Accessor = EMTestAccountDynamicAccessor<'a>;
    fn new_accessor(header: &'a Self, data: Ref<'a, [u8]>) -> Self::Accessor {
        Self::Accessor { header, data }
    }
}

impl<'a> GetAccessorMut<'a> for EMTestAccountHeader {
    type AccessorMut = EMTestAccountDynamicAccessorMut<'a>;
    fn new_accessor_mut(header: &'a Self, data: RefMut<'a, [u8]>) -> Self::AccessorMut {
        Self::AccessorMut { header, data }
    }
}
