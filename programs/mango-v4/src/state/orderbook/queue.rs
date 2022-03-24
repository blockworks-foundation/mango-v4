use crate::error::MangoError;
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use mango_macro::Pod;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use super::Side;

pub const MAX_NUM_EVENTS: usize = 512;

pub trait QueueHeader: bytemuck::Pod {
    type Item: bytemuck::Pod + Copy;

    fn head(&self) -> usize;
    fn set_head(&mut self, value: usize);
    fn count(&self) -> usize;
    fn set_count(&mut self, value: usize);

    fn incr_event_id(&mut self);
    fn decr_event_id(&mut self, n: usize);
}

#[account(zero_copy)]
pub struct Queue<H: QueueHeader> {
    pub header: H,
    pub buf: [H::Item; MAX_NUM_EVENTS],
}

impl<'a, H: QueueHeader> Queue<H> {
    pub fn len(&self) -> usize {
        self.header.count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

    pub fn pop_front(&mut self) -> Result<H::Item> {
        require!(!self.empty(), MangoError::SomeError);

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

struct QueueIterator<'a, H: QueueHeader> {
    queue: &'a Queue<H>,
    index: usize,
}

impl<'a, H: QueueHeader> Iterator for QueueIterator<'a, H> {
    type Item = &'a H::Item;
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
    head: usize,
    count: usize,
    pub seq_num: usize,
}

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

pub type EventQueue = Queue<EventQueueHeader>;

const EVENT_SIZE: usize = 200;
#[derive(Copy, Clone, Debug, Pod)]
#[repr(C)]
pub struct AnyEvent {
    pub event_type: u8,
    pub padding: [u8; EVENT_SIZE - 1],
}

#[derive(Copy, Clone, IntoPrimitive, TryFromPrimitive, Eq, PartialEq)]
#[repr(u8)]
pub enum EventType {
    Fill,
    Out,
    Liquidate,
}

#[derive(Copy, Clone, Debug, Pod)]
#[repr(C)]
pub struct FillEvent {
    pub event_type: u8,
    pub taker_side: Side, // side from the taker's POV
    pub maker_slot: u8,
    pub maker_out: bool, // true if maker order quantity == 0
    pub version: u8,
    pub market_fees_applied: bool,
    pub padding: [u8; 2],
    pub timestamp: u64,
    pub seq_num: usize, // note: usize same as u64

    pub maker: Pubkey,
    pub maker_order_id: i128,
    pub maker_client_order_id: u64,
    pub maker_fee: I80F48,

    // The best bid/ask at the time the maker order was placed. Used for liquidity incentives
    pub best_initial: i64,

    // Timestamp of when the maker order was placed; copied over from the LeafNode
    pub maker_timestamp: u64,

    pub taker: Pubkey,
    pub taker_order_id: i128,
    pub taker_client_order_id: u64,
    pub taker_fee: I80F48,

    pub price: i64,
    pub quantity: i64, // number of quote lots
}

impl FillEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        taker_side: Side,
        maker_slot: u8,
        maker_out: bool,
        timestamp: u64,
        seq_num: usize,
        maker: Pubkey,
        maker_order_id: i128,
        maker_client_order_id: u64,
        maker_fee: I80F48,
        best_initial: i64,
        maker_timestamp: u64,

        taker: Pubkey,
        taker_order_id: i128,
        taker_client_order_id: u64,
        taker_fee: I80F48,
        price: i64,
        quantity: i64,
        version: u8,
    ) -> FillEvent {
        Self {
            event_type: EventType::Fill as u8,
            taker_side,
            maker_slot,
            maker_out,
            version,
            market_fees_applied: true, // Since mango v3.3.5, market fees are adjusted at matching time
            padding: [0u8; 2],
            timestamp,
            seq_num,
            maker,
            maker_order_id,
            maker_client_order_id,
            maker_fee,
            best_initial,
            maker_timestamp,
            taker,
            taker_order_id,
            taker_client_order_id,
            taker_fee,
            price,
            quantity,
        }
    }

    pub fn base_quote_change(&self, side: Side) -> (i64, i64) {
        match side {
            Side::Bid => (
                self.quantity,
                -self.price.checked_mul(self.quantity).unwrap(),
            ),
            Side::Ask => (
                -self.quantity,
                self.price.checked_mul(self.quantity).unwrap(),
            ),
        }
    }
}

#[derive(Copy, Clone, Debug, Pod)]
#[repr(C)]
pub struct OutEvent {
    pub event_type: u8,
    pub side: Side,
    pub slot: u8,
    padding0: [u8; 5],
    pub timestamp: u64,
    pub seq_num: usize,
    pub owner: Pubkey,
    pub quantity: i64,
    padding1: [u8; EVENT_SIZE - 64],
}

impl OutEvent {
    pub fn new(
        side: Side,
        slot: u8,
        timestamp: u64,
        seq_num: usize,
        owner: Pubkey,
        quantity: i64,
    ) -> Self {
        Self {
            event_type: EventType::Out.into(),
            side,
            slot,
            padding0: [0; 5],
            timestamp,
            seq_num,
            owner,
            quantity,
            padding1: [0; EVENT_SIZE - 64],
        }
    }
}
