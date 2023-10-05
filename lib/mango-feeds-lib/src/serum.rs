use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct SerumEventQueueHeader {
    pub _account_flags: u64, // Initialized, EventQueue
    pub head: u64,
    pub count: u64,
    pub seq_num: u64,
}
unsafe impl Zeroable for SerumEventQueueHeader {}
unsafe impl Pod for SerumEventQueueHeader {}
