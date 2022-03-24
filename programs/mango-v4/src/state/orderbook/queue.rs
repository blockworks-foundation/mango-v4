use std::cell::RefMut;
use std::mem::size_of;

use crate::error::MangoError;
use crate::state::PerpMarket;
use anchor_lang::prelude::*;
use bytemuck::{cast_slice_mut, from_bytes_mut};
use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::rent::Rent;

use mango_macro::Pod;

use super::metadata::MetaData;

pub const MAX_NUM_EVENTS: usize = 512;

#[inline]
pub fn remove_slop_mut<T: bytemuck::Pod>(bytes: &mut [u8]) -> &mut [T] {
    let slop = bytes.len() % size_of::<T>();
    let new_len = bytes.len() - slop;
    cast_slice_mut(&mut bytes[..new_len])
}

pub fn strip_header_mut<'a, H: bytemuck::Pod, D: bytemuck::Pod>(
    account: &'a AccountInfo,
) -> std::result::Result<(RefMut<'a, H>, RefMut<'a, [D]>), Error> {
    Ok(RefMut::map_split(account.try_borrow_mut_data()?, |data| {
        let (header_bytes, inner_bytes) = data.split_at_mut(size_of::<H>());
        (from_bytes_mut(header_bytes), remove_slop_mut(inner_bytes))
    }))
}

pub trait QueueHeader: bytemuck::Pod {
    type Item: bytemuck::Pod + Copy;

    fn head(&self) -> usize;
    fn set_head(&mut self, value: usize);
    fn count(&self) -> usize;
    fn set_count(&mut self, value: usize);

    fn incr_event_id(&mut self);
    fn decr_event_id(&mut self, n: usize);
}

pub struct Queue<'a, H: QueueHeader> {
    pub header: RefMut<'a, H>,
    pub buf: RefMut<'a, [H::Item]>,
}

impl<'a, H: QueueHeader> Queue<'a, H> {
    pub fn new(header: RefMut<'a, H>, buf: RefMut<'a, [H::Item]>) -> Self {
        Self { header, buf }
    }

    pub fn load_mut(account: &'a AccountInfo) -> Result<Self> {
        let (header, buf) = strip_header_mut::<H, H::Item>(account)?;
        Ok(Self { header, buf })
    }

    pub fn len(&self) -> usize {
        self.header.count()
    }

    pub fn full(&self) -> bool {
        self.header.count() == self.buf.len()
    }

    pub fn empty(&self) -> bool {
        self.header.count() == 0
    }

    pub fn push_back(&mut self, value: H::Item) -> std::result::Result<(), H::Item> {
        if self.full() {
            return Err(value);
        }
        let slot = (self.header.head() + self.header.count()) % self.buf.len();
        self.buf[slot] = value;

        let count = self.header.count();
        self.header.set_count(count + 1);

        self.header.incr_event_id();
        Ok(())
    }

    pub fn peek_front(&self) -> Option<&H::Item> {
        if self.empty() {
            return None;
        }
        Some(&self.buf[self.header.head()])
    }

    pub fn peek_front_mut(&mut self) -> Option<&mut H::Item> {
        if self.empty() {
            return None;
        }
        Some(&mut self.buf[self.header.head()])
    }

    pub fn pop_front(&mut self) -> std::result::Result<H::Item, ()> {
        if self.empty() {
            return Err(());
        }
        let value = self.buf[self.header.head()];

        let count = self.header.count();
        self.header.set_count(count - 1);

        let head = self.header.head();
        self.header.set_head((head + 1) % self.buf.len());

        Ok(value)
    }

    pub fn revert_pushes(&mut self, desired_len: usize) -> Result<()> {
        require!(desired_len <= self.header.count(), MangoError::SomeError);
        let len_diff = self.header.count() - desired_len;
        self.header.set_count(desired_len);
        self.header.decr_event_id(len_diff);
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &H::Item> {
        QueueIterator {
            queue: self,
            index: 0,
        }
    }
}

struct QueueIterator<'a, 'b, H: QueueHeader> {
    queue: &'b Queue<'a, H>,
    index: usize,
}

impl<'a, 'b, H: QueueHeader> Iterator for QueueIterator<'a, 'b, H> {
    type Item = &'b H::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.queue.len() {
            None
        } else {
            let item =
                &self.queue.buf[(self.queue.header.head() + self.index) % self.queue.buf.len()];
            self.index += 1;
            Some(item)
        }
    }
}

#[account(zero_copy)]
pub struct EventQueueHeader {
    pub meta_data: MetaData,
    head: usize,
    count: usize,
    pub seq_num: usize,
}
// unsafe impl TriviallyTransmutable for EventQueueHeader {}

impl QueueHeader for EventQueueHeader {
    type Item = AnyEvent;

    fn head(&self) -> usize {
        self.head
    }
    fn set_head(&mut self, value: usize) {
        self.head = value;
    }
    fn count(&self) -> usize {
        self.count
    }
    fn set_count(&mut self, value: usize) {
        self.count = value;
    }
    fn incr_event_id(&mut self) {
        self.seq_num += 1;
    }
    fn decr_event_id(&mut self, n: usize) {
        self.seq_num -= n;
    }
}

pub type EventQueue<'a> = Queue<'a, EventQueueHeader>;

impl<'a> EventQueue<'a> {
    pub fn load_mut_checked(
        account: &'a AccountInfo,
        program_id: &Pubkey,
        perp_market: &PerpMarket,
    ) -> Result<Self> {
        require!(account.owner == program_id, MangoError::SomeError);
        require!(
            &perp_market.event_queue == account.key,
            MangoError::SomeError
        );
        Self::load_mut(account)
    }

    pub fn load_and_init(
        account: &'a AccountInfo,
        program_id: &Pubkey,
        rent: &Rent,
    ) -> Result<Self> {
        // NOTE: check this first so we can borrow account later
        require!(
            rent.is_exempt(account.lamports(), account.data_len()),
            MangoError::SomeError
        );

        let state = Self::load_mut(account)?;
        require!(account.owner == program_id, MangoError::SomeError);

        // require!(
        //     !state.header.meta_data.is_initialized,
        //     MangoError::SomeError
        // );
        // state.header.meta_data = MetaData::new(DataType::EventQueue, 0, true);

        Ok(state)
    }
}

const EVENT_SIZE: usize = 200;
#[derive(Copy, Clone, Debug, Pod)]
#[repr(C)]
pub struct AnyEvent {
    pub event_type: u8,
    pub padding: [u8; EVENT_SIZE - 1],
}
