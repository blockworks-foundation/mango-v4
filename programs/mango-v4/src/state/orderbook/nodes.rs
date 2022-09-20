use std::mem::size_of;

use anchor_lang::prelude::*;
use bytemuck::{cast_mut, cast_ref};
use mango_macro::Pod;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use static_assertions::const_assert_eq;

use super::order_type::OrderType;

pub type NodeHandle = u32;
const NODE_SIZE: usize = 96;

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum NodeTag {
    Uninitialized = 0,
    InnerNode = 1,
    LeafNode = 2,
    FreeNode = 3,
    LastFreeNode = 4,
}

/// InnerNodes and LeafNodes compose the binary tree of orders.
///
/// Each InnerNode has exactly two children, which are either InnerNodes themselves,
/// or LeafNodes. The children share the top `prefix_len` bits of `key`. The left
/// child has a 0 in the next bit, and the right a 1.
#[derive(Copy, Clone, Pod, AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct InnerNode {
    pub tag: u32,
    /// number of highest `key` bits that all children share
    /// e.g. if it's 2, the two highest bits of `key` will be the same on all children
    pub prefix_len: u32,

    /// only the top `prefix_len` bits of `key` are relevant
    pub key: i128,

    /// indexes into `BookSide::nodes`
    pub children: [NodeHandle; 2],

    /// The earliest expiry timestamp for the left and right subtrees.
    ///
    /// Needed to be able to find and remove expired orders without having to
    /// iterate through the whole bookside.
    pub child_earliest_expiry: [u64; 2],

    pub reserved: [u8; 48],
}
const_assert_eq!(size_of::<InnerNode>() % 8, 0);
const_assert_eq!(size_of::<InnerNode>(), NODE_SIZE);

impl InnerNode {
    pub fn new(prefix_len: u32, key: i128) -> Self {
        Self {
            tag: NodeTag::InnerNode.into(),
            prefix_len,
            key,
            children: [0; 2],
            child_earliest_expiry: [u64::MAX; 2],
            reserved: [0; NODE_SIZE - 48],
        }
    }

    /// Returns the handle of the child that may contain the search key
    /// and 0 or 1 depending on which child it was.
    pub(crate) fn walk_down(&self, search_key: i128) -> (NodeHandle, bool) {
        let crit_bit_mask = 1i128 << (127 - self.prefix_len);
        let crit_bit = (search_key & crit_bit_mask) != 0;
        (self.children[crit_bit as usize], crit_bit)
    }

    /// The lowest timestamp at which one of the contained LeafNodes expires.
    #[inline(always)]
    pub fn earliest_expiry(&self) -> u64 {
        std::cmp::min(self.child_earliest_expiry[0], self.child_earliest_expiry[1])
    }
}

/// LeafNodes represent an order in the binary tree
#[derive(Debug, Copy, Clone, PartialEq, Eq, Pod, AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct LeafNode {
    pub tag: u32,
    pub owner_slot: u8,
    pub order_type: OrderType, // this was added for TradingView move order

    pub padding: [u8; 1],

    /// Time in seconds after `timestamp` at which the order expires.
    /// A value of 0 means no expiry.
    pub time_in_force: u8,

    /// The binary tree key
    pub key: i128,

    pub owner: Pubkey,
    pub quantity: i64,
    pub client_order_id: u64,

    // The time the order was placed
    pub timestamp: u64,

    pub reserved: [u8; 16],
}
const_assert_eq!(size_of::<LeafNode>() % 8, 0);
const_assert_eq!(size_of::<LeafNode>(), NODE_SIZE);

#[inline(always)]
fn key_to_price(key: i128) -> i64 {
    (key >> 64) as i64
}

impl LeafNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        owner_slot: u8,
        key: i128,
        owner: Pubkey,
        quantity: i64,
        client_order_id: u64,
        timestamp: u64,
        order_type: OrderType,
        time_in_force: u8,
    ) -> Self {
        Self {
            tag: NodeTag::LeafNode.into(),
            owner_slot,
            order_type,
            padding: [0],
            time_in_force,
            key,
            owner,
            quantity,
            client_order_id,
            timestamp,
            reserved: [0; 16],
        }
    }

    #[inline(always)]
    pub fn price(&self) -> i64 {
        key_to_price(self.key)
    }

    /// Time at which this order will expire, u64::MAX if never
    #[inline(always)]
    pub fn expiry(&self) -> u64 {
        if self.time_in_force == 0 {
            u64::MAX
        } else {
            self.timestamp + self.time_in_force as u64
        }
    }

    #[inline(always)]
    pub fn is_valid(&self, now_ts: u64) -> bool {
        self.time_in_force == 0 || now_ts < self.timestamp + self.time_in_force as u64
    }
}

#[derive(Copy, Clone, Pod)]
#[repr(C)]
pub struct FreeNode {
    pub(crate) tag: u32,
    pub(crate) next: NodeHandle,
    pub(crate) reserved: [u8; NODE_SIZE - 8],
}

#[zero_copy]
#[derive(Pod)]
pub struct AnyNode {
    pub tag: u32,
    pub data: [u8; 92], // note: anchor can't parse the struct for IDL when it includes non numbers, NODE_SIZE == 96, 92 == 96 - 4
}

const_assert_eq!(size_of::<AnyNode>(), NODE_SIZE);
const_assert_eq!(size_of::<AnyNode>(), size_of::<InnerNode>());
const_assert_eq!(size_of::<AnyNode>(), size_of::<LeafNode>());
const_assert_eq!(size_of::<AnyNode>(), size_of::<FreeNode>());

pub(crate) enum NodeRef<'a> {
    Inner(&'a InnerNode),
    Leaf(&'a LeafNode),
}

pub(crate) enum NodeRefMut<'a> {
    Inner(&'a mut InnerNode),
    Leaf(&'a mut LeafNode),
}

impl AnyNode {
    pub fn key(&self) -> Option<i128> {
        match self.case()? {
            NodeRef::Inner(inner) => Some(inner.key),
            NodeRef::Leaf(leaf) => Some(leaf.key),
        }
    }

    pub(crate) fn children(&self) -> Option<[NodeHandle; 2]> {
        match self.case().unwrap() {
            NodeRef::Inner(&InnerNode { children, .. }) => Some(children),
            NodeRef::Leaf(_) => None,
        }
    }

    pub(crate) fn case(&self) -> Option<NodeRef> {
        match NodeTag::try_from(self.tag) {
            Ok(NodeTag::InnerNode) => Some(NodeRef::Inner(cast_ref(self))),
            Ok(NodeTag::LeafNode) => Some(NodeRef::Leaf(cast_ref(self))),
            _ => None,
        }
    }

    fn case_mut(&mut self) -> Option<NodeRefMut> {
        match NodeTag::try_from(self.tag) {
            Ok(NodeTag::InnerNode) => Some(NodeRefMut::Inner(cast_mut(self))),
            Ok(NodeTag::LeafNode) => Some(NodeRefMut::Leaf(cast_mut(self))),
            _ => None,
        }
    }

    #[inline]
    pub fn as_leaf(&self) -> Option<&LeafNode> {
        match self.case() {
            Some(NodeRef::Leaf(leaf_ref)) => Some(leaf_ref),
            _ => None,
        }
    }

    #[inline]
    pub fn as_leaf_mut(&mut self) -> Option<&mut LeafNode> {
        match self.case_mut() {
            Some(NodeRefMut::Leaf(leaf_ref)) => Some(leaf_ref),
            _ => None,
        }
    }

    #[inline]
    pub fn as_inner(&self) -> Option<&InnerNode> {
        match self.case() {
            Some(NodeRef::Inner(inner_ref)) => Some(inner_ref),
            _ => None,
        }
    }

    #[inline]
    pub fn as_inner_mut(&mut self) -> Option<&mut InnerNode> {
        match self.case_mut() {
            Some(NodeRefMut::Inner(inner_ref)) => Some(inner_ref),
            _ => None,
        }
    }

    #[inline]
    pub fn earliest_expiry(&self) -> u64 {
        match self.case().unwrap() {
            NodeRef::Inner(inner) => inner.earliest_expiry(),
            NodeRef::Leaf(leaf) => leaf.expiry(),
        }
    }
}

impl AsRef<AnyNode> for InnerNode {
    fn as_ref(&self) -> &AnyNode {
        cast_ref(self)
    }
}

impl AsRef<AnyNode> for LeafNode {
    #[inline]
    fn as_ref(&self) -> &AnyNode {
        cast_ref(self)
    }
}
