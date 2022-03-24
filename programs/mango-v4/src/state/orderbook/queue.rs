use crate::error::MangoError;
use anchor_lang::prelude::*;
use mango_macro::Pod;

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
