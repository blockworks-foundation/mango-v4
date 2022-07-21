use std::cell::Ref;
use std::cell::RefMut;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use arrayref::array_ref;
use bytemuck::Zeroable;
use solana_program::program_memory::sol_memmove;

use super::{PerpPositions, Serum3Orders, TokenPosition};

type BorshVecLength = u32;
const BORSH_VEC_PADDING_BYTES: usize = 4;
const BORSH_VEC_SIZE_BYTES: usize = 4;

// Mango Account
// This struct definition is only for clients e.g. typescript, so that they can easily use out of the box
// deserialization and not have to do custom deserialization
// On chain, we would prefer zero-copying to optimize for compute
#[account]
#[derive(Default)]
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
}

impl MangoAccount2 {
    pub fn space(token_count: u8, serum3_count: u8, perp_count: u8) -> usize {
        8 + size_of::<MangoAccount2Fixed>()
            + Self::dynamic_size(token_count, serum3_count, perp_count)
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

    pub fn dynamic_size(token_count: u8, serum3_count: u8, perp_count: u8) -> usize {
        Self::dynamic_perp_vec_offset(token_count, serum3_count)
            + (BORSH_VEC_SIZE_BYTES + size_of::<PerpPositions>() * usize::from(perp_count))
    }
}

#[test]
fn test_dynamic_offsets() {
    let mut account = MangoAccount2::default();
    account.tokens.resize(3, TokenPosition::zeroed());
    account.serum3.resize(5, Serum3Orders::default());
    account.perps.resize(7, PerpPositions::default());
    assert_eq!(
        8 + AnchorSerialize::try_to_vec(&account).unwrap().len(),
        MangoAccount2::space(3, 5, 7)
    );
}

// Mango Account fixed part for easy zero copy deserialization
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct MangoAccount2Fixed {
    pub owner: Pubkey,
}

// Header is created by scanning and parsing dynamic portion of the account
// Header stores useful information e.g. offsets to easily seek into dynamic content
pub trait Header: Sized {
    // build header by scanning and parsing dynamic portion of the account
    fn try_new_header(data: &[u8]) -> Result<Self>;

    // initialize a header on a new account, if necessary
    fn initialize(data: &mut [u8]) -> Result<()>;
}
pub trait GetAccessor<'a, FixedPart> {
    type Accessor;
    fn new_accessor(fixed: Ref<'a, FixedPart>, header: Self, data: Ref<'a, [u8]>)
        -> Self::Accessor;
}

pub trait GetAccessorMut<'a, FixedPart> {
    type AccessorMut;
    fn new_accessor_mut(
        fixed: RefMut<'a, FixedPart>,
        header: Self,
        dynamic: RefMut<'a, [u8]>,
    ) -> Self::AccessorMut;
}

#[derive(Clone)]
pub struct MangoAccount2DynamicHeader {
    pub token_count: u8,
    pub serum3_count: u8,
    pub perp_count: u8,
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
        MangoAccount2::dynamic_token_vec_offset()
            + BORSH_VEC_SIZE_BYTES
            + raw_index * size_of::<TokenPosition>()
    }

    // offset into dynamic data where 1st Serum3Orders would be found
    fn serum3_offset(&self, raw_index: usize) -> usize {
        MangoAccount2::dynamic_serum3_vec_offset(self.token_count)
            + BORSH_VEC_SIZE_BYTES
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
        MangoAccount2::dynamic_perp_vec_offset(self.token_count, self.serum3_count)
            + BORSH_VEC_SIZE_BYTES
            + raw_index * size_of::<PerpPositions>()
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
}

pub struct MangoAccount2Accessor<
    Fixed: Deref<Target = MangoAccount2Fixed>,
    Dynamic: Deref<Target = [u8]>,
> {
    pub header: MangoAccount2DynamicHeader,
    pub fixed: Fixed,
    pub dynamic: Dynamic,
}

type MangoAccountAcc<'a> = MangoAccount2Accessor<&'a MangoAccount2Fixed, &'a [u8]>;
type MangoAccountAccMut<'a> = MangoAccount2Accessor<&'a mut MangoAccount2Fixed, &'a mut [u8]>;

pub fn test_fun(a: MangoAccountAcc) -> u16 {
    a.token_raw(1).token_index
}

impl<Fixed: Deref<Target = MangoAccount2Fixed>, Dynamic: Deref<Target = [u8]>>
    MangoAccount2Accessor<Fixed, Dynamic>
{
    // get TokenPosition at raw_index
    pub fn token_raw(&self, raw_index: usize) -> &TokenPosition {
        get_helper(&self.dynamic, self.header.token_offset(raw_index))
    }

    // get iter over all TokenPositions (including inactive)
    pub fn token_iter_raw(&self) -> impl Iterator<Item = &TokenPosition> + '_ {
        (0..self.header.token_count()).map(|i| self.token_raw(i))
    }

    // get Serum3Orders at raw_index
    pub fn serum3_raw(&self, raw_index: usize) -> &Serum3Orders {
        get_helper(&self.dynamic, self.header.serum3_offset(raw_index))
    }

    // get PerpPosition at raw_index
    pub fn perp_raw(&self, raw_index: usize) -> &PerpPositions {
        get_helper(&self.dynamic, self.header.perp_offset(raw_index))
    }

    pub fn borrow<'a>(&'a self) -> MangoAccount2Accessor<&'a MangoAccount2Fixed, &'a [u8]> {
        MangoAccount2Accessor {
            fixed: &self.fixed,
            header: self.header.clone(),
            dynamic: &self.dynamic,
        }
    }
}

impl<Fixed: DerefMut<Target = MangoAccount2Fixed>, Dynamic: DerefMut<Target = [u8]>>
    MangoAccount2Accessor<Fixed, Dynamic>
{
    // get mut TokenPosition at raw_index
    pub fn token_raw_mut(&mut self, raw_index: usize) -> &mut TokenPosition {
        get_helper_mut(&mut self.dynamic, self.header.token_offset(raw_index))
    }

    // get mut Serum3Orders at raw_index
    pub fn serum3_raw_mut(&mut self, raw_index: usize) -> &mut Serum3Orders {
        get_helper_mut(&mut self.dynamic, self.header.serum3_offset(raw_index))
    }

    // get mut PerpPosition at raw_index
    pub fn perp_raw_mut(&mut self, raw_index: usize) -> &mut PerpPositions {
        get_helper_mut(&mut self.dynamic, self.header.perp_offset(raw_index))
    }

    // writes length of tokens vec at appropriate offset so that borsh can infer the vector length
    // length used is that present in the header
    fn write_token_length(&mut self) {
        let tokens_offset = self.header.token_offset(0);
        // msg!(
        //     "writing tokens length at {}",
        //     tokens_offset - size_of::<BorshVecLength>()
        // );
        let dst: &mut [u8] = &mut self.dynamic[tokens_offset - BORSH_VEC_SIZE_BYTES..tokens_offset];
        dst.copy_from_slice(&BorshVecLength::from(self.header.token_count).to_le_bytes());
    }

    fn write_serum3_length(&mut self) {
        let serum3_offset = self.header.serum3_offset(0);
        // msg!(
        //     "writing serum3 length at {}",
        //     serum3_offset - size_of::<BorshVecLength>()
        // );
        let dst: &mut [u8] = &mut self.dynamic[serum3_offset - BORSH_VEC_SIZE_BYTES..serum3_offset];
        dst.copy_from_slice(&BorshVecLength::from(self.header.serum3_count).to_le_bytes());
    }

    fn write_perp_length(&mut self) {
        let perp_offset = self.header.perp_offset(0);
        // msg!(
        //     "writing perp length at {}",
        //     perp_offset - size_of::<BorshVecLength>()
        // );
        let dst: &mut [u8] = &mut self.dynamic[perp_offset - BORSH_VEC_SIZE_BYTES..perp_offset];
        dst.copy_from_slice(&BorshVecLength::from(self.header.perp_count).to_le_bytes());
    }

    pub fn expand_dynamic_content(
        &mut self,
        new_token_count: u8,
        new_serum3_count: u8,
        new_perp_count: u8,
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

        // expand dynamic components by first moving existing positions, and then setting new ones to defaults

        // perp positions
        unsafe {
            sol_memmove(
                &mut self.dynamic[new_header.perp_offset(0)],
                &mut self.dynamic[self.header.perp_offset(0)],
                size_of::<PerpPositions>() * self.header.perp_count(),
            );
        }
        for i in self.header.perp_count..new_perp_count {
            *get_helper_mut(&mut self.dynamic, new_header.perp_offset(i.into())) =
                PerpPositions::default();
        }

        // serum3 positions
        unsafe {
            sol_memmove(
                &mut self.dynamic[new_header.serum3_offset(0)],
                &mut self.dynamic[self.header.serum3_offset(0)],
                size_of::<Serum3Orders>() * self.header.serum3_count(),
            );
        }
        for i in self.header.serum3_count..new_serum3_count {
            *get_helper_mut(&mut self.dynamic, new_header.serum3_offset(i.into())) =
                Serum3Orders::default();
        }

        // token positions
        unsafe {
            sol_memmove(
                &mut self.dynamic[new_header.token_offset(0)],
                &mut self.dynamic[self.header.token_offset(0)],
                size_of::<TokenPosition>() * self.header.token_count(),
            );
        }
        for i in self.header.token_count..new_token_count {
            *get_helper_mut(&mut self.dynamic, new_header.token_offset(i.into())) =
                TokenPosition::default();
        }

        // update header
        self.header.token_count = new_token_count;
        self.header.serum3_count = new_serum3_count;
        self.header.perp_count = new_perp_count;

        // write new lengths (uses header)
        self.write_token_length();
        self.write_serum3_length();
        self.write_perp_length();

        Ok(())
    }
}

impl Header for MangoAccount2DynamicHeader {
    fn try_new_header(data: &[u8]) -> Result<Self> {
        let token_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
            data,
            MangoAccount2::dynamic_token_vec_offset(),
            BORSH_VEC_SIZE_BYTES
        ]))
        .unwrap();
        // msg!(
        //     "reading tokens length at {}",
        //     8 - size_of::<BorshVecLength>()
        // );

        let serum3_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
            data,
            MangoAccount2::dynamic_serum3_vec_offset(token_count),
            BORSH_VEC_SIZE_BYTES
        ]))
        .unwrap();
        // msg!(
        //     "reading serum3 length at {}",
        //     8 + size_of::<TokenPosition>() * token_count + 8 - size_of::<BorshVecLength>()
        // );

        let perp_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
            data,
            MangoAccount2::dynamic_perp_vec_offset(token_count, serum3_count),
            BORSH_VEC_SIZE_BYTES
        ]))
        .unwrap();
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

    fn initialize(_data: &mut [u8]) -> Result<()> {
        Ok(())
    }
}

impl<'a> GetAccessor<'a, MangoAccount2Fixed> for MangoAccount2DynamicHeader {
    type Accessor = MangoAccount2Accessor<Ref<'a, MangoAccount2Fixed>, Ref<'a, [u8]>>;
    fn new_accessor(
        fixed: Ref<'a, MangoAccount2Fixed>,
        header: Self,
        dynamic: Ref<'a, [u8]>,
    ) -> Self::Accessor {
        Self::Accessor {
            fixed,
            header,
            dynamic,
        }
    }
}

impl<'a> GetAccessorMut<'a, MangoAccount2Fixed> for MangoAccount2DynamicHeader {
    type AccessorMut = MangoAccount2Accessor<RefMut<'a, MangoAccount2Fixed>, RefMut<'a, [u8]>>;
    fn new_accessor_mut(
        fixed: RefMut<'a, MangoAccount2Fixed>,
        header: Self,
        dynamic: RefMut<'a, [u8]>,
    ) -> Self::AccessorMut {
        Self::AccessorMut {
            fixed,
            header,
            dynamic,
        }
    }
}

#[derive(Clone)]
pub struct MangoAccountLoader<
    'info,
    FixedPart: bytemuck::Pod,
    HeaderPart: Header,
    ClientAccount: Owner + Discriminator,
> {
    acc_info: AccountInfo<'info>,
    phantom1: PhantomData<&'info FixedPart>,
    phantom2: PhantomData<&'info HeaderPart>,
    phantom3: PhantomData<&'info ClientAccount>,
}

impl<'info, FixedPart: bytemuck::Pod, HeaderPart: Header, ClientAccount: Owner + Discriminator>
    MangoAccountLoader<'info, FixedPart, HeaderPart, ClientAccount>
{
    pub fn new(
        acc_info: AccountInfo<'info>,
    ) -> Result<MangoAccountLoader<'info, FixedPart, HeaderPart, ClientAccount>> {
        if acc_info.owner != &ClientAccount::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*acc_info.owner, ClientAccount::owner())));
        }
        Ok(Self {
            acc_info,
            phantom1: PhantomData,
            phantom2: PhantomData,
            phantom3: PhantomData,
        })
    }

    /// Returns a Ref to the account data structure for reading.
    pub fn load<'a>(&'a self) -> Result<HeaderPart::Accessor>
    where
        HeaderPart: GetAccessor<'a, FixedPart>,
    {
        let data = self.acc_info.try_borrow_data()?;
        if data.len() < ClientAccount::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &ClientAccount::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }
        drop(data);

        let data = self.acc_info.try_borrow_data()?;
        let (fixed, dynamic) = Ref::map_split(data, |data| {
            let (fixed_slice, dynamic_slice) = data.split_at(8 + size_of::<FixedPart>());
            let (_disc, fixed_slice) = fixed_slice.split_at(8);
            (
                bytemuck::from_bytes::<FixedPart>(fixed_slice),
                dynamic_slice,
            )
        });

        Ok(HeaderPart::new_accessor(
            fixed,
            HeaderPart::try_new_header(&dynamic)?,
            dynamic,
        ))
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    pub fn load_mut<'a>(&'a self) -> Result<HeaderPart::AccessorMut>
    where
        HeaderPart: GetAccessorMut<'a, FixedPart>,
    {
        if !self.acc_info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data = self.acc_info.try_borrow_mut_data()?;
        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &ClientAccount::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        let (fixed, dynamic) = RefMut::map_split(data, |data| {
            let (fixed_slice, dynamic_slice) = data.split_at_mut(8 + size_of::<FixedPart>());
            let (_disc, fixed_slice) = fixed_slice.split_at_mut(8);
            (
                bytemuck::from_bytes_mut::<FixedPart>(fixed_slice),
                dynamic_slice,
            )
        });

        Ok(HeaderPart::new_accessor_mut(
            fixed,
            HeaderPart::try_new_header(&dynamic)?,
            dynamic,
        ))
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    /// Should only be called once, when the account is being initialized.
    pub fn load_init<'a>(&'a self) -> Result<HeaderPart::AccessorMut>
    where
        HeaderPart: GetAccessorMut<'a, FixedPart>,
    {
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
        disc_bytes.copy_from_slice(bytemuck::bytes_of(&(ClientAccount::discriminator())));

        let (fixed, mut dynamic) = RefMut::map_split(data, |data| {
            let (fixed_slice, dynamic_slice) = data.split_at_mut(8 + size_of::<FixedPart>());
            let (_disc, fixed_slice) = fixed_slice.split_at_mut(8);
            (
                bytemuck::from_bytes_mut::<FixedPart>(fixed_slice),
                dynamic_slice,
            )
        });

        HeaderPart::initialize(&mut dynamic)?;

        Ok(HeaderPart::new_accessor_mut(
            fixed,
            HeaderPart::try_new_header(&dynamic)?,
            dynamic,
        ))
    }
}
