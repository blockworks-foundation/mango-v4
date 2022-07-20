use std::cell::Ref;
use std::cell::RefMut;
use std::marker::PhantomData;
use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use anchor_lang::ZeroCopy;
use arrayref::array_ref;
use bytemuck::Zeroable;

use super::{PerpPositions, Serum3Orders, TokenPosition};

type BorshVecLength = u32;

// Mango Account
// This struct definition is only for clients e.g. typescript, so that they can easily use out of the box
// deserialization and not have to do custom deserialization
// On chain, we would prefer zero-copying to optimize for compute
#[account]
pub struct MangoAccount2 {
    // fixed
    // note: keep MangoAccount2Fixed in sync with changes here
    pub owner: Pubkey,
    // TODO: port remaining fixed fields from MangoAccount

    // dynamic
    // note: padding is required for TokenPosition, etc. to be aligned
    pub padding1: u32,
    pub tokens: Vec<TokenPosition>,
    pub padding2: u32,
    pub serum3: Vec<Serum3Orders>,
    pub padding3: u32,
    pub perps: Vec<PerpPositions>,
    // placeholders for future features, adding them here for backwards compatibility
    pub padding4: u32,
    pub feature4: Vec<u8>,
    pub padding5: u32,
    pub feature5: Vec<u8>,
    pub padding6: u32,
    pub feature6: Vec<u8>,
    pub padding7: u32,
    pub feature7: Vec<u8>,
}

impl MangoAccount2 {
    pub fn space(token_count: usize, serum3_count: usize, perp_count: usize) -> usize {
        8 + size_of::<MangoAccount2Fixed>() // owner
            + 8 // padding + length of vec
            + size_of::<TokenPosition>() * token_count
            + 8
            + size_of::<Serum3Orders>() * serum3_count
            + 8
            + size_of::<PerpPositions>() * perp_count
            + 8 // feature 4
            + 8 // feature 5
            + 8 // feature 6
            + 8 // feature 7
    }
}

// Mango Account fixed part for easy zero copy deserialization
#[account(zero_copy)]
pub struct MangoAccount2Fixed {
    pub owner: Pubkey,
}

// Header is created by scanning and parsing dynamic portion of the account
// Header stores useful information e.g. offsets to easily seek into dynamic content
pub trait Header: Sized {
    // build header by scanning and parsing dynamic portion of the account
    fn try_new_header(data: &[u8]) -> Result<Self>;

    // build header from desired dynamic portion size
    fn new_header(token_count: usize, serum3_count: usize, perp_count: usize) -> Self;
}
pub trait GetAccessor<'a> {
    type Accessor;
    fn new_accessor(header: Self, data: Ref<'a, [u8]>) -> Self::Accessor;
}

pub trait GetAccessorMut<'a> {
    type AccessorMut;
    fn new_accessor_mut(header: Self, data: RefMut<'a, [u8]>) -> Self::AccessorMut;
}

pub struct SplitAccount<T, U> {
    pub fixed: T,
    pub dynamic: U,
}

pub struct MangoAccount2DynamicHeader {
    pub token_count: usize,
    pub serum3_count: usize,
    pub perp_count: usize,
}

fn get_helper<T: bytemuck::Pod>(data: &[u8], index: usize) -> &T {
    bytemuck::from_bytes(&data[index..index + size_of::<T>()])
}

fn get_helper_mut<T: bytemuck::Pod>(data: &mut [u8], index: usize) -> &mut T {
    bytemuck::from_bytes_mut(&mut data[index..index + size_of::<T>()])
}

impl MangoAccount2DynamicHeader {
    // offset into dynamic data where 1st TokenPosition would be found
    fn token_offset(&self, raw_index: usize) -> usize {
        8 + raw_index * size_of::<TokenPosition>()
    }

    // offset into dynamic data where 1st Serum3Orders would be found
    fn serum3_offset(&self, raw_index: usize) -> usize {
        self.token_offset(0)
            + size_of::<TokenPosition>() * self.token_count
            + 8
            + raw_index * size_of::<Serum3Orders>()
    }

    // offset into dynamic data where 1st PerpPositions would be found
    fn perp_offset(&self, raw_index: usize) -> usize {
        // msg!(
        //     "perp_offset self.serum3_offset(0) {}",
        //     self.serum3_offset(0)
        // );
        // msg!(
        //     "perp_offset size_of::<Serum3Orders>() * self.serum3_count {}",
        //     size_of::<Serum3Orders>() * self.serum3_count
        // );
        self.serum3_offset(0)
            + size_of::<Serum3Orders>() * self.serum3_count
            + 8
            + raw_index * size_of::<PerpPositions>()
    }

    // get TokenPosition at raw_index
    fn token_get_raw<'a>(&self, data: &'a [u8], raw_index: usize) -> &'a TokenPosition {
        get_helper(data, self.token_offset(raw_index))
    }

    // get mut TokenPosition at raw_index
    fn token_get_raw_mut<'a>(&self, data: &'a mut [u8], raw_index: usize) -> &'a mut TokenPosition {
        get_helper_mut(data, self.token_offset(raw_index))
    }

    // get iter over all TokenPositions
    fn token_iter<'a>(&'a self, data: &'a [u8]) -> impl Iterator<Item = &'a TokenPosition> + '_ {
        (0..self.token_count as usize).map(|i| self.token_get_raw(data, i))
    }

    // get Serum3Orders at raw_index
    fn serum3_get_raw<'a>(&self, data: &'a [u8], raw_index: usize) -> &'a Serum3Orders {
        get_helper(data, self.serum3_offset(raw_index))
    }

    // get mut Serum3Orders at raw_index
    fn serum3_get_raw_mut<'a>(&self, data: &'a mut [u8], raw_index: usize) -> &'a mut Serum3Orders {
        get_helper_mut(data, self.serum3_offset(raw_index))
    }

    // get iter Serum3Orders all TokenPositions
    fn serum3_iter<'a>(&'a self, data: &'a [u8]) -> impl Iterator<Item = &'a Serum3Orders> + '_ {
        (0..self.serum3_count as usize).map(|i| self.serum3_get_raw(data, i))
    }

    // get PerpPosition at raw_index
    fn perp_get_raw<'a>(&self, data: &'a [u8], raw_index: usize) -> &'a PerpPositions {
        get_helper(data, self.perp_offset(raw_index))
    }

    // get mut PerpPosition at raw_index
    fn perp_get_raw_mut<'a>(&self, data: &'a mut [u8], raw_index: usize) -> &'a mut PerpPositions {
        get_helper_mut(data, self.perp_offset(raw_index))
    }

    // get iter PerpPosition all PerpPositions
    fn perp_iter<'a>(&'a self, data: &'a [u8]) -> impl Iterator<Item = &'a PerpPositions> + '_ {
        (0..self.perp_count as usize).map(|i| self.perp_get_raw(data, i))
    }
}

pub struct MangoAccount2DynamicAccessor<'a> {
    pub header: MangoAccount2DynamicHeader,
    data: Ref<'a, [u8]>,
}

impl<'a> MangoAccount2DynamicAccessor<'a> {
    pub fn token_get_raw(&self, raw_index: usize) -> &TokenPosition {
        self.header.token_get_raw(&self.data, raw_index)
    }

    pub fn token_iter(&self) -> impl Iterator<Item = &TokenPosition> + '_ {
        self.header.token_iter(&self.data)
    }

    pub fn serum3_get_raw(&self, raw_index: usize) -> &Serum3Orders {
        self.header.serum3_get_raw(&self.data, raw_index)
    }

    pub fn perp_get_raw(&self, raw_index: usize) -> &PerpPositions {
        self.header.perp_get_raw(&self.data, raw_index)
    }
}

pub struct MangoAccount2DynamicAccessorMut<'a> {
    pub header: MangoAccount2DynamicHeader,
    data: RefMut<'a, [u8]>,
}

impl<'a> MangoAccount2DynamicAccessorMut<'a> {
    pub fn token_get_raw(&self, raw_index: usize) -> &TokenPosition {
        self.header.token_get_raw(&self.data, raw_index)
    }

    pub fn token_get_raw_mut(&mut self, raw_index: usize) -> &mut TokenPosition {
        self.header.token_get_raw_mut(&mut self.data, raw_index)
    }

    pub fn token_iter(&self) -> impl Iterator<Item = &TokenPosition> + '_ {
        self.header.token_iter(&self.data)
    }

    pub fn serum3_get_raw_mut(&mut self, raw_index: usize) -> &mut Serum3Orders {
        self.header.serum3_get_raw_mut(&mut self.data, raw_index)
    }

    pub fn perp_get_raw_mut(&mut self, raw_index: usize) -> &mut PerpPositions {
        self.header.perp_get_raw_mut(&mut self.data, raw_index)
    }

    // writes length of tokens vec at appropriate offset so that borsh can infer the vector length
    // length used is that present in the header
    pub fn write_tokens_length(&mut self) {
        let tokens_offset = self.header.token_offset(0);
        // msg!(
        //     "writing tokens length at {}",
        //     tokens_offset - size_of::<BorshVecLength>()
        // );
        let dst: &mut [u8] =
            &mut self.data[tokens_offset - size_of::<BorshVecLength>()..tokens_offset];
        dst.copy_from_slice(&(self.header.token_count as u32).to_le_bytes());
    }

    pub fn write_serum3_length(&mut self) {
        let serum3_offset = self.header.serum3_offset(0);
        // msg!(
        //     "writing serum3 length at {}",
        //     serum3_offset - size_of::<BorshVecLength>()
        // );
        let dst: &mut [u8] =
            &mut self.data[serum3_offset - size_of::<BorshVecLength>()..serum3_offset];
        dst.copy_from_slice(&(self.header.serum3_count as u32).to_le_bytes());
    }

    pub fn write_perp_length(&mut self) {
        let perp_offset = self.header.perp_offset(0);
        // msg!(
        //     "writing perp length at {}",
        //     perp_offset - size_of::<BorshVecLength>()
        // );
        let dst: &mut [u8] = &mut self.data[perp_offset - size_of::<BorshVecLength>()..perp_offset];
        dst.copy_from_slice(&(self.header.perp_count as u32).to_le_bytes());
    }

    pub fn expand_dynamic_content(
        &mut self,
        new_token_count: usize,
        new_serum3_count: usize,
        new_perp_count: usize,
    ) -> Result<()> {
        require_gt!(new_token_count, self.header.token_count);
        require_gt!(new_serum3_count, self.header.serum3_count);
        require_gt!(new_perp_count, self.header.perp_count);

        // create a temp copy to compute new starting offsets
        let new_header = MangoAccount2DynamicHeader {
            token_count: new_token_count,
            serum3_count: new_serum3_count,
            perp_count: new_perp_count,
        };

        // slide dynamic components further later, starting at the one closest to the end i.e. perp positions

        // move perp positions
        for i in (0..new_header.perp_count).rev() {
            let dest_offset = new_header.perp_offset(i);
            let source_copy = if i < self.header.perp_count {
                // create a clone since we are modifying self.data mutably later
                *self.header.perp_get_raw(&self.data, i)
            } else {
                // new unset positions
                PerpPositions::zeroed()
            };
            let dest_bytes: &mut [u8] =
                &mut self.data[dest_offset..dest_offset + size_of::<PerpPositions>()];
            dest_bytes.copy_from_slice(bytemuck::bytes_of(&source_copy));
        }

        // move serum3 orders
        for i in (0..new_header.serum3_count).rev() {
            let dest_offset = new_header.serum3_offset(i);
            let source_copy = if i < self.header.serum3_count {
                *self.header.serum3_get_raw(&self.data, i)
            } else {
                Serum3Orders::zeroed()
            };
            // msg!(
            //     "pos {:?} serum market index {:?}",
            //     i,
            //     source_copy.market_index
            // );
            let dest_bytes: &mut [u8] =
                &mut self.data[dest_offset..dest_offset + size_of::<Serum3Orders>()];
            dest_bytes.copy_from_slice(bytemuck::bytes_of(&source_copy));
        }

        // move token positions
        for i in (0..new_header.token_count).rev() {
            let dest_offset = new_header.token_offset(i);
            let source_copy = if i < self.header.token_count {
                *self.header.token_get_raw(&self.data, i)
            } else {
                TokenPosition::zeroed()
            };
            let dest_bytes: &mut [u8] =
                &mut self.data[dest_offset..dest_offset + size_of::<TokenPosition>()];
            dest_bytes.copy_from_slice(bytemuck::bytes_of(&source_copy));
        }

        // update header
        self.header.token_count = new_token_count;
        self.header.serum3_count = new_serum3_count;
        self.header.perp_count = new_perp_count;

        // write new lengths (uses header)
        self.write_tokens_length();
        self.write_serum3_length();
        self.write_perp_length();

        Ok(())
    }
}

impl Header for MangoAccount2DynamicHeader {
    fn try_new_header(data: &[u8]) -> Result<Self> {
        let token_count = u32::from_le_bytes(*array_ref![
            data,
            8 - size_of::<BorshVecLength>(),
            size_of::<BorshVecLength>()
        ]) as usize;
        // msg!(
        //     "reading tokens length at {}",
        //     8 - size_of::<BorshVecLength>()
        // );

        let serum3_count = u32::from_le_bytes(*array_ref![
            data,
            8 + size_of::<TokenPosition>() * token_count + 8 - size_of::<BorshVecLength>(),
            size_of::<BorshVecLength>()
        ]) as usize;
        // msg!(
        //     "reading serum3 length at {}",
        //     8 + size_of::<TokenPosition>() * token_count + 8 - size_of::<BorshVecLength>()
        // );

        let perp_count = u32::from_le_bytes(*array_ref![
            data,
            8 + size_of::<TokenPosition>() * token_count
                + 8
                + size_of::<Serum3Orders>() * serum3_count
                + 8
                - size_of::<BorshVecLength>(),
            size_of::<BorshVecLength>()
        ]) as usize;
        // msg!(
        //     "reading perp length at {}",
        //     8 + size_of::<TokenPosition>() * token_count
        //         + 8
        //         + size_of::<Serum3Orders>() * serum3_count
        //         + 8
        //         - size_of::<BorshVecLength>()
        // );

        // msg!(
        //     "scanned & parsed {:?} {:?} {:?}",
        //     token_count,
        //     serum3_count,
        //     perp_count
        // );

        Ok(Self {
            token_count,
            serum3_count,
            perp_count,
        })
    }

    fn new_header(token_count: usize, serum3_count: usize, perp_count: usize) -> Self {
        Self {
            token_count,
            serum3_count,
            perp_count,
        }
    }
}

impl<'a> GetAccessor<'a> for MangoAccount2DynamicHeader {
    type Accessor = MangoAccount2DynamicAccessor<'a>;
    fn new_accessor(header: Self, data: Ref<'a, [u8]>) -> Self::Accessor {
        MangoAccount2DynamicAccessor { header, data }
    }
}

impl<'a> GetAccessorMut<'a> for MangoAccount2DynamicHeader {
    type AccessorMut = MangoAccount2DynamicAccessorMut<'a>;
    fn new_accessor_mut(header: Self, data: RefMut<'a, [u8]>) -> Self::AccessorMut {
        Self::AccessorMut { header, data }
    }
}

#[derive(Clone)]
pub struct MangoAccountLoader<'info, T: ZeroCopy + Owner, U: Header, V: Owner + Discriminator> {
    acc_info: AccountInfo<'info>,
    phantom_t: PhantomData<&'info T>,
    phantom_u: PhantomData<&'info U>,
    phantom_v: PhantomData<&'info V>,
}

impl<'info, T: ZeroCopy + Owner, U: Header, V: Owner + Discriminator>
    MangoAccountLoader<'info, T, U, V>
{
    pub fn new(acc_info: AccountInfo<'info>) -> MangoAccountLoader<'info, T, U, V> {
        Self {
            acc_info,
            phantom_t: PhantomData,
            phantom_u: PhantomData,
            phantom_v: PhantomData,
        }
    }

    /// Returns a Ref to the account data structure for reading.
    pub fn load<'a>(&'a self) -> Result<SplitAccount<Ref<T>, U::Accessor>>
    where
        U: GetAccessor<'a>,
    {
        let data = self.acc_info.try_borrow_data()?;
        if data.len() < V::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &V::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }
        drop(data);

        let data = self.acc_info.try_borrow_data()?;
        let (fixed, dynamic) = Ref::map_split(data, |data| {
            let (fixed_slice, dynamic_slice) = data.split_at(8 + size_of::<T>());
            let (_disc, fixed_slice) = fixed_slice.split_at(8);
            (bytemuck::from_bytes::<T>(fixed_slice), dynamic_slice)
        });

        Ok(SplitAccount {
            fixed,
            dynamic: U::new_accessor(U::try_new_header(&dynamic)?, dynamic),
        })
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    pub fn load_mut<'a>(&'a self) -> Result<SplitAccount<RefMut<T>, U::AccessorMut>>
    where
        U: GetAccessorMut<'a>,
    {
        if !self.acc_info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data = self.acc_info.try_borrow_mut_data()?;
        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &V::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        let (fixed, dynamic) = RefMut::map_split(data, |data| {
            let (fixed_slice, dynamic_slice) = data.split_at_mut(8 + size_of::<T>());
            let (_disc, fixed_slice) = fixed_slice.split_at_mut(8);
            (bytemuck::from_bytes_mut::<T>(fixed_slice), dynamic_slice)
        });

        Ok(SplitAccount {
            fixed,
            dynamic: U::new_accessor_mut(U::try_new_header(&dynamic)?, dynamic),
        })
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    /// Should only be called once, when the account is being initialized.
    pub fn load_init<'a>(
        &'a self,
        token_count: usize,
        serum3_count: usize,
        perp_count: usize,
    ) -> Result<SplitAccount<RefMut<T>, U::AccessorMut>>
    where
        U: GetAccessorMut<'a>,
    {
        let mut data = self.acc_info.try_borrow_mut_data()?;
        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        let discriminator = u64::from_le_bytes(disc_bytes);
        if discriminator != 0 {
            return Err(ErrorCode::AccountDiscriminatorAlreadySet.into());
        }

        let disc_bytes: &mut [u8] = &mut data[0..8];
        disc_bytes.copy_from_slice(bytemuck::bytes_of(&(V::discriminator())));

        let (fixed, dynamic) = RefMut::map_split(data, |data| {
            let (fixed_slice, dynamic_slice) = data.split_at_mut(8 + size_of::<T>());
            let (_disc, fixed_slice) = fixed_slice.split_at_mut(8);
            (bytemuck::from_bytes_mut::<T>(fixed_slice), dynamic_slice)
        });

        Ok(SplitAccount {
            fixed,
            dynamic: U::new_accessor_mut(
                U::new_header(token_count, serum3_count, perp_count),
                dynamic,
            ),
        })
    }
}
