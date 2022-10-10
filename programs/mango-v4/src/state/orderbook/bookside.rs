use anchor_lang::prelude::*;
use bytemuck::{cast, cast_mut, cast_ref};

use num_enum::{IntoPrimitive, TryFromPrimitive};
use static_assertions::const_assert_eq;

use crate::state::orderbook::bookside_iterator::*;

use crate::error::MangoError;
use crate::state::orderbook::nodes::{
    AnyNode, FreeNode, InnerNode, LeafNode, NodeHandle, NodeRef, NodeTag,
};

pub const MAX_BOOK_NODES: usize = 1024;

#[derive(
    Eq,
    PartialEq,
    Copy,
    Clone,
    TryFromPrimitive,
    IntoPrimitive,
    Debug,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[repr(u8)]
pub enum OrderTreeType {
    Bids,
    Asks,
}

/// A binary tree on AnyNode::key()
///
/// The key encodes the price in the top 64 bits.
#[account(zero_copy)]
pub struct OrderTree {
    // pub meta_data: MetaData,
    // todo: do we want this type at this level?
    pub order_tree_type: OrderTreeType,
    pub padding: [u8; 3],
    pub bump_index: u32,
    pub free_list_len: u32,
    pub free_list_head: NodeHandle,
    pub root_node: NodeHandle,
    pub leaf_count: u32,
    pub nodes: [AnyNode; MAX_BOOK_NODES],
    pub reserved: [u8; 256],
}
const_assert_eq!(
    std::mem::size_of::<OrderTree>(),
    1 + 3 + 4 * 2 + 4 + 4 + 4 + 96 * 1024 + 256 // 98584
);
const_assert_eq!(std::mem::size_of::<OrderTree>() % 8, 0);

impl OrderTree {
    /// Iterate over all entries, including invalid orders
    ///
    /// smallest to highest for asks
    /// highest to smallest for bids
    pub fn iter(&self) -> OrderTreeIter {
        OrderTreeIter::new(self)
    }

    pub fn node_mut(&mut self, key: NodeHandle) -> Option<&mut AnyNode> {
        let node = &mut self.nodes[key as usize];
        let tag = NodeTag::try_from(node.tag);
        match tag {
            Ok(NodeTag::InnerNode) | Ok(NodeTag::LeafNode) => Some(node),
            _ => None,
        }
    }
    pub fn node(&self, key: NodeHandle) -> Option<&AnyNode> {
        let node = &self.nodes[key as usize];
        let tag = NodeTag::try_from(node.tag);
        match tag {
            Ok(NodeTag::InnerNode) | Ok(NodeTag::LeafNode) => Some(node),
            _ => None,
        }
    }

    pub fn remove_min(&mut self) -> Option<LeafNode> {
        self.remove_by_key(self.min_leaf()?.key)
    }

    pub fn remove_max(&mut self) -> Option<LeafNode> {
        self.remove_by_key(self.max_leaf()?.key)
    }

    pub fn remove_worst(&mut self) -> Option<LeafNode> {
        match self.order_tree_type {
            OrderTreeType::Bids => self.remove_min(),
            OrderTreeType::Asks => self.remove_max(),
        }
    }

    /// Remove the order with the lowest expiry timestamp, if that's < now_ts.
    pub fn remove_one_expired(&mut self, now_ts: u64) -> Option<LeafNode> {
        let (expired_h, expires_at) = self.find_earliest_expiry()?;
        if expires_at < now_ts {
            self.remove_by_key(self.node(expired_h)?.key()?)
        } else {
            None
        }
    }

    pub fn root(&self) -> Option<NodeHandle> {
        if self.leaf_count == 0 {
            None
        } else {
            Some(self.root_node)
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn to_price_quantity_vec(&self, reverse: bool) -> Vec<(i64, i64)> {
        let mut pqs = vec![];
        let mut current: NodeHandle = match self.root() {
            None => return pqs,
            Some(node_handle) => node_handle,
        };

        let left = reverse as usize;
        let right = !reverse as usize;
        let mut stack = vec![];
        loop {
            let root_contents = self.node(current).unwrap(); // should never fail unless book is already fucked
            match root_contents.case().unwrap() {
                NodeRef::Inner(inner) => {
                    stack.push(inner);
                    current = inner.children[left];
                }
                NodeRef::Leaf(leaf) => {
                    // if you hit leaf then pop stack and go right
                    // all inner nodes on stack have already been visited to the left
                    pqs.push((leaf.price(), leaf.quantity));
                    match stack.pop() {
                        None => return pqs,
                        Some(inner) => {
                            current = inner.children[right];
                        }
                    }
                }
            }
        }
    }

    pub fn min_leaf(&self) -> Option<&LeafNode> {
        self.leaf_min_max(false)
    }

    pub fn max_leaf(&self) -> Option<&LeafNode> {
        self.leaf_min_max(true)
    }
    fn leaf_min_max(&self, find_max: bool) -> Option<&LeafNode> {
        let mut root: NodeHandle = self.root()?;

        let i = if find_max { 1 } else { 0 };
        loop {
            let root_contents = self.node(root)?;
            match root_contents.case()? {
                NodeRef::Inner(inner) => {
                    root = inner.children[i];
                }
                NodeRef::Leaf(leaf) => {
                    return Some(leaf);
                }
            }
        }
    }

    pub fn remove_by_key(&mut self, search_key: u128) -> Option<LeafNode> {
        // path of InnerNode handles that lead to the removed leaf
        let mut stack: Vec<(NodeHandle, bool)> = vec![];

        // special case potentially removing the root
        let mut parent_h = self.root()?;
        let (mut child_h, mut crit_bit) = match self.node(parent_h).unwrap().case().unwrap() {
            NodeRef::Leaf(&leaf) if leaf.key == search_key => {
                assert_eq!(self.leaf_count, 1);
                self.root_node = 0;
                self.leaf_count = 0;
                let _old_root = self.remove(parent_h).unwrap();
                return Some(leaf);
            }
            NodeRef::Leaf(_) => return None,
            NodeRef::Inner(inner) => inner.walk_down(search_key),
        };
        stack.push((parent_h, crit_bit));

        // walk down the tree until finding the key
        loop {
            match self.node(child_h).unwrap().case().unwrap() {
                NodeRef::Inner(inner) => {
                    parent_h = child_h;
                    let (new_child_h, new_crit_bit) = inner.walk_down(search_key);
                    child_h = new_child_h;
                    crit_bit = new_crit_bit;
                    stack.push((parent_h, crit_bit));
                }
                NodeRef::Leaf(leaf) => {
                    if leaf.key != search_key {
                        return None;
                    }
                    break;
                }
            }
        }

        // replace parent with its remaining child node
        // free child_h, replace *parent_h with *other_child_h, free other_child_h
        let other_child_h = self.node(parent_h).unwrap().children().unwrap()[!crit_bit as usize];
        let other_child_node_contents = self.remove(other_child_h).unwrap();
        let new_expiry = other_child_node_contents.earliest_expiry();
        *self.node_mut(parent_h).unwrap() = other_child_node_contents;
        self.leaf_count -= 1;
        let removed_leaf: LeafNode = cast(self.remove(child_h).unwrap());

        // update child min expiry back up to the root
        let outdated_expiry = removed_leaf.expiry();
        stack.pop(); // the final parent has been replaced by the remaining leaf
        self.update_parent_earliest_expiry(&stack, outdated_expiry, new_expiry);

        Some(removed_leaf)
    }

    pub fn remove(&mut self, key: NodeHandle) -> Option<AnyNode> {
        let val = *self.node(key)?;

        self.nodes[key as usize] = cast(FreeNode {
            tag: if self.free_list_len == 0 {
                NodeTag::LastFreeNode.into()
            } else {
                NodeTag::FreeNode.into()
            },
            next: self.free_list_head,
            reserved: [0; 88],
        });

        self.free_list_len += 1;
        self.free_list_head = key;
        Some(val)
    }

    pub fn insert(&mut self, val: &AnyNode) -> Result<NodeHandle> {
        match NodeTag::try_from(val.tag) {
            Ok(NodeTag::InnerNode) | Ok(NodeTag::LeafNode) => (),
            _ => unreachable!(),
        };

        if self.free_list_len == 0 {
            require!(
                (self.bump_index as usize) < self.nodes.len() && self.bump_index < u32::MAX,
                MangoError::SomeError // todo
            );

            self.nodes[self.bump_index as usize] = *val;
            let key = self.bump_index;
            self.bump_index += 1;
            return Ok(key);
        }

        let key = self.free_list_head;
        let node = &mut self.nodes[key as usize];

        // TODO OPT possibly unnecessary require here - remove if we need compute
        match NodeTag::try_from(node.tag) {
            Ok(NodeTag::FreeNode) => assert!(self.free_list_len > 1),
            Ok(NodeTag::LastFreeNode) => assert_eq!(self.free_list_len, 1),
            _ => unreachable!(),
        };

        // TODO - test borrow requireer
        self.free_list_head = cast_ref::<AnyNode, FreeNode>(node).next;
        self.free_list_len -= 1;
        *node = *val;
        Ok(key)
    }
    pub fn insert_leaf(&mut self, new_leaf: &LeafNode) -> Result<(NodeHandle, Option<LeafNode>)> {
        // path of InnerNode handles that lead to the new leaf
        let mut stack: Vec<(NodeHandle, bool)> = vec![];

        // deal with inserts into an empty tree
        let mut root: NodeHandle = match self.root() {
            Some(h) => h,
            None => {
                // create a new root if none exists
                let handle = self.insert(new_leaf.as_ref())?;
                self.root_node = handle;
                self.leaf_count = 1;
                return Ok((handle, None));
            }
        };

        // walk down the tree until we find the insert location
        loop {
            // require if the new node will be a child of the root
            let root_contents = *self.node(root).unwrap();
            let root_key = root_contents.key().unwrap();
            if root_key == new_leaf.key {
                // This should never happen because key should never match
                if let Some(NodeRef::Leaf(&old_root_as_leaf)) = root_contents.case() {
                    // clobber the existing leaf
                    *self.node_mut(root).unwrap() = *new_leaf.as_ref();
                    self.update_parent_earliest_expiry(
                        &stack,
                        old_root_as_leaf.expiry(),
                        new_leaf.expiry(),
                    );
                    return Ok((root, Some(old_root_as_leaf)));
                }
                // InnerNodes have a random child's key, so matching can happen and is fine
            }
            let shared_prefix_len: u32 = (root_key ^ new_leaf.key).leading_zeros();
            match root_contents.case() {
                None => unreachable!(),
                Some(NodeRef::Inner(inner)) => {
                    let keep_old_root = shared_prefix_len >= inner.prefix_len;
                    if keep_old_root {
                        let (child, crit_bit) = inner.walk_down(new_leaf.key);
                        stack.push((root, crit_bit));
                        root = child;
                        continue;
                    };
                }
                _ => (),
            };
            // implies root is a Leaf or Inner where shared_prefix_len < prefix_len
            // we'll replace root with a new InnerNode that has new_leaf and root as children

            // change the root in place to represent the LCA of [new_leaf] and [root]
            let crit_bit_mask: u128 = 1u128 << (127 - shared_prefix_len);
            let new_leaf_crit_bit = (crit_bit_mask & new_leaf.key) != 0;
            let old_root_crit_bit = !new_leaf_crit_bit;

            let new_leaf_handle = self.insert(new_leaf.as_ref())?;
            let moved_root_handle = match self.insert(&root_contents) {
                Ok(h) => h,
                Err(e) => {
                    self.remove(new_leaf_handle).unwrap();
                    return Err(e);
                }
            };

            let new_root: &mut InnerNode = cast_mut(self.node_mut(root).unwrap());
            *new_root = InnerNode::new(shared_prefix_len, new_leaf.key);

            new_root.children[new_leaf_crit_bit as usize] = new_leaf_handle;
            new_root.children[old_root_crit_bit as usize] = moved_root_handle;

            let new_leaf_expiry = new_leaf.expiry();
            let old_root_expiry = root_contents.earliest_expiry();
            new_root.child_earliest_expiry[new_leaf_crit_bit as usize] = new_leaf_expiry;
            new_root.child_earliest_expiry[old_root_crit_bit as usize] = old_root_expiry;

            // walk up the stack and fix up the new min if needed
            if new_leaf_expiry < old_root_expiry {
                self.update_parent_earliest_expiry(&stack, old_root_expiry, new_leaf_expiry);
            }

            self.leaf_count += 1;
            return Ok((new_leaf_handle, None));
        }
    }

    pub fn is_full(&self) -> bool {
        self.free_list_len <= 1 && (self.bump_index as usize) >= self.nodes.len() - 1
    }

    /// When a node changes, the parents' child_earliest_expiry may need to be updated.
    ///
    /// This function walks up the `stack` of parents and applies the change where the
    /// previous child's `outdated_expiry` is replaced by `new_expiry`.
    pub fn update_parent_earliest_expiry(
        &mut self,
        stack: &[(NodeHandle, bool)],
        mut outdated_expiry: u64,
        mut new_expiry: u64,
    ) {
        // Walk from the top of the stack to the root of the tree.
        // Since the stack grows by appending, we need to iterate the slice in reverse order.
        for (parent_h, crit_bit) in stack.iter().rev() {
            let parent = self.node_mut(*parent_h).unwrap().as_inner_mut().unwrap();
            if parent.child_earliest_expiry[*crit_bit as usize] != outdated_expiry {
                break;
            }
            outdated_expiry = parent.earliest_expiry();
            parent.child_earliest_expiry[*crit_bit as usize] = new_expiry;
            new_expiry = parent.earliest_expiry();
        }
    }

    /// Returns the handle of the node with the lowest expiry timestamp, and this timestamp
    pub fn find_earliest_expiry(&self) -> Option<(NodeHandle, u64)> {
        let mut current: NodeHandle = match self.root() {
            Some(h) => h,
            None => return None,
        };

        loop {
            let contents = *self.node(current).unwrap();
            match contents.case() {
                None => unreachable!(),
                Some(NodeRef::Inner(inner)) => {
                    current = inner.children[(inner.child_earliest_expiry[0]
                        > inner.child_earliest_expiry[1])
                        as usize];
                }
                _ => {
                    return Some((current, contents.earliest_expiry()));
                }
            };
        }
    }
}

#[derive(
    Eq,
    PartialEq,
    Copy,
    Clone,
    TryFromPrimitive,
    IntoPrimitive,
    Debug,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[repr(u8)]
pub enum BookSideOrderTree {
    Fixed,
    OraclePegged,
}

/// Reference to a node in a book side component
pub struct BookSideOrderHandle {
    pub node: NodeHandle,
    pub order_tree: BookSideOrderTree,
}

#[derive(Clone, Copy)]
pub struct BookSideRef<'a> {
    pub fixed: &'a OrderTree,
    pub oracle_pegged: &'a OrderTree,
}

pub struct BookSideRefMut<'a> {
    pub fixed: &'a mut OrderTree,
    pub oracle_pegged: &'a mut OrderTree,
}

impl<'a> BookSideRef<'a> {
    /// Iterate over all entries in the book filtering out invalid orders
    ///
    /// smallest to highest for asks
    /// highest to smallest for bids
    pub fn iter_valid(
        &self,
        now_ts: u64,
        oracle_price_lots: i64,
    ) -> impl Iterator<Item = BookSideIterItem> {
        BookSideIter::new(*self, now_ts, oracle_price_lots).filter(|it| it.is_valid)
    }

    /// Iterate over all entries, including invalid orders
    pub fn iter_all_including_invalid(&self, now_ts: u64, oracle_price_lots: i64) -> BookSideIter {
        BookSideIter::new(*self, now_ts, oracle_price_lots)
    }

    pub fn orders(&self, component: BookSideOrderTree) -> &OrderTree {
        match component {
            BookSideOrderTree::Fixed => self.fixed,
            BookSideOrderTree::OraclePegged => self.oracle_pegged,
        }
    }

    pub fn node(&self, key: BookSideOrderHandle) -> Option<&AnyNode> {
        self.orders(key.order_tree).node(key.node)
    }

    pub fn is_full(&self, component: BookSideOrderTree) -> bool {
        self.orders(component).is_full()
    }
}

impl<'a> BookSideRefMut<'a> {
    pub fn non_mut(&self) -> BookSideRef {
        BookSideRef {
            fixed: self.fixed,
            oracle_pegged: self.oracle_pegged,
        }
    }

    pub fn orders_mut(&mut self, component: BookSideOrderTree) -> &mut OrderTree {
        match component {
            BookSideOrderTree::Fixed => self.fixed,
            BookSideOrderTree::OraclePegged => self.oracle_pegged,
        }
    }

    pub fn node_mut(&mut self, key: BookSideOrderHandle) -> Option<&mut AnyNode> {
        self.orders_mut(key.order_tree).node_mut(key.node)
    }

    pub fn remove_worst(&mut self, component: BookSideOrderTree) -> Option<LeafNode> {
        self.orders_mut(component).remove_worst()
    }

    /// Remove the order with the lowest expiry timestamp, if that's < now_ts.
    pub fn remove_one_expired(
        &mut self,
        component: BookSideOrderTree,
        now_ts: u64,
    ) -> Option<LeafNode> {
        self.orders_mut(component).remove_one_expired(now_ts)
    }

    pub fn remove_by_key(
        &mut self,
        component: BookSideOrderTree,
        search_key: u128,
    ) -> Option<LeafNode> {
        self.orders_mut(component).remove_by_key(search_key)
    }

    pub fn remove(&mut self, key: BookSideOrderHandle) -> Option<AnyNode> {
        self.orders_mut(key.order_tree).remove(key.node)
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use bytemuck::Zeroable;

    fn new_order_tree(order_tree_type: OrderTreeType) -> OrderTree {
        OrderTree {
            order_tree_type,
            padding: [0u8; 3],
            bump_index: 0,
            free_list_len: 0,
            free_list_head: 0,
            root_node: 0,
            leaf_count: 0,
            nodes: [AnyNode::zeroed(); MAX_BOOK_NODES],
            reserved: [0; 256],
        }
    }

    fn verify_order_tree(order_tree: &OrderTree) {
        verify_order_tree_invariant(order_tree);
        verify_order_tree_iteration(order_tree);
        verify_order_tree_expiry(order_tree);
    }

    // check that BookSide binary tree key invariant holds
    fn verify_order_tree_invariant(order_tree: &OrderTree) {
        let r = match order_tree.root() {
            Some(h) => h,
            None => return,
        };

        fn recursive_check(order_tree: &OrderTree, h: NodeHandle) {
            match order_tree.node(h).unwrap().case().unwrap() {
                NodeRef::Inner(&inner) => {
                    let left = order_tree.node(inner.children[0]).unwrap().key().unwrap();
                    let right = order_tree.node(inner.children[1]).unwrap().key().unwrap();

                    // the left and right keys share the InnerNode's prefix
                    assert!((inner.key ^ left).leading_zeros() >= inner.prefix_len);
                    assert!((inner.key ^ right).leading_zeros() >= inner.prefix_len);

                    // the left and right node key have the critbit unset and set respectively
                    let crit_bit_mask: u128 = 1u128 << (127 - inner.prefix_len);
                    assert!(left & crit_bit_mask == 0);
                    assert!(right & crit_bit_mask != 0);

                    recursive_check(order_tree, inner.children[0]);
                    recursive_check(order_tree, inner.children[1]);
                }
                _ => {}
            }
        }
        recursive_check(order_tree, r);
    }

    // check that iteration of order tree has the right order and misses no leaves
    fn verify_order_tree_iteration(order_tree: &OrderTree) {
        let mut total = 0;
        let ascending = order_tree.order_tree_type == OrderTreeType::Asks;
        let mut last_key = if ascending { 0 } else { u128::MAX };
        for (_, node) in order_tree.iter() {
            let key = node.key;
            if ascending {
                assert!(key >= last_key);
            } else {
                assert!(key <= last_key);
            }
            last_key = key;
            total += 1;
        }
        assert_eq!(order_tree.leaf_count, total);
    }

    // check that BookSide::child_expiry invariant holds
    fn verify_order_tree_expiry(order_tree: &OrderTree) {
        let r = match order_tree.root() {
            Some(h) => h,
            None => return,
        };

        fn recursive_check(order_tree: &OrderTree, h: NodeHandle) {
            match order_tree.node(h).unwrap().case().unwrap() {
                NodeRef::Inner(&inner) => {
                    let left = order_tree
                        .node(inner.children[0])
                        .unwrap()
                        .earliest_expiry();
                    let right = order_tree
                        .node(inner.children[1])
                        .unwrap()
                        .earliest_expiry();

                    // child_expiry must hold the expiry of the children
                    assert_eq!(inner.child_earliest_expiry[0], left);
                    assert_eq!(inner.child_earliest_expiry[1], right);

                    recursive_check(order_tree, inner.children[0]);
                    recursive_check(order_tree, inner.children[1]);
                }
                _ => {}
            }
        }
        recursive_check(order_tree, r);
    }

    #[test]
    fn order_tree_expiry_manual() {
        let mut bids = new_order_tree(OrderTreeType::Bids);
        let new_expiring_leaf = |key: u128, expiry: u64| {
            LeafNode::new(
                0,
                key,
                Pubkey::default(),
                0,
                0,
                expiry - 1,
                OrderType::Limit,
                1,
                0,
            )
        };

        assert!(bids.find_earliest_expiry().is_none());

        bids.insert_leaf(&new_expiring_leaf(0, 5000)).unwrap();
        assert_eq!(bids.find_earliest_expiry().unwrap(), (bids.root_node, 5000));
        verify_order_tree(&bids);

        let (new4000_h, _) = bids.insert_leaf(&new_expiring_leaf(1, 4000)).unwrap();
        assert_eq!(bids.find_earliest_expiry().unwrap(), (new4000_h, 4000));
        verify_order_tree(&bids);

        let (_new4500_h, _) = bids.insert_leaf(&new_expiring_leaf(2, 4500)).unwrap();
        assert_eq!(bids.find_earliest_expiry().unwrap(), (new4000_h, 4000));
        verify_order_tree(&bids);

        let (new3500_h, _) = bids.insert_leaf(&new_expiring_leaf(3, 3500)).unwrap();
        assert_eq!(bids.find_earliest_expiry().unwrap(), (new3500_h, 3500));
        verify_order_tree(&bids);
        // the first two levels of the tree are innernodes, with 0;1 on one side and 2;3 on the other
        assert_eq!(
            bids.node_mut(bids.root_node)
                .unwrap()
                .as_inner_mut()
                .unwrap()
                .child_earliest_expiry,
            [4000, 3500]
        );

        bids.remove_by_key(3).unwrap();
        verify_order_tree(&bids);
        assert_eq!(
            bids.node_mut(bids.root_node)
                .unwrap()
                .as_inner_mut()
                .unwrap()
                .child_earliest_expiry,
            [4000, 4500]
        );
        assert_eq!(bids.find_earliest_expiry().unwrap().1, 4000);

        bids.remove_by_key(0).unwrap();
        verify_order_tree(&bids);
        assert_eq!(
            bids.node_mut(bids.root_node)
                .unwrap()
                .as_inner_mut()
                .unwrap()
                .child_earliest_expiry,
            [4000, 4500]
        );
        assert_eq!(bids.find_earliest_expiry().unwrap().1, 4000);

        bids.remove_by_key(1).unwrap();
        verify_order_tree(&bids);
        assert_eq!(bids.find_earliest_expiry().unwrap().1, 4500);

        bids.remove_by_key(2).unwrap();
        verify_order_tree(&bids);
        assert!(bids.find_earliest_expiry().is_none());
    }

    #[test]
    fn order_tree_expiry_random() {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let mut bids = new_order_tree(OrderTreeType::Bids);
        let new_expiring_leaf = |key: u128, expiry: u64| {
            LeafNode::new(
                0,
                key,
                Pubkey::default(),
                0,
                0,
                expiry - 1,
                OrderType::Limit,
                1,
                0,
            )
        };

        // add 200 random leaves
        let mut keys = vec![];
        for _ in 0..200 {
            let key: u128 = rng.gen_range(0..10000); // overlap in key bits
            if keys.contains(&key) {
                continue;
            }
            let expiry = rng.gen_range(1..200); // give good chance of duplicate expiry times
            keys.push(key);
            bids.insert_leaf(&new_expiring_leaf(key, expiry)).unwrap();
            verify_order_tree(&bids);
        }

        // remove 50 at random
        for _ in 0..50 {
            if keys.len() == 0 {
                break;
            }
            let k = keys[rng.gen_range(0..keys.len())];
            bids.remove_by_key(k).unwrap();
            keys.retain(|v| *v != k);
            verify_order_tree(&bids);
        }
    }

    fn bookside2_iteration_random_helper(side: Side) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let order_tree_type = match side {
            Side::Bid => OrderTreeType::Bids,
            Side::Ask => OrderTreeType::Asks,
        };

        let mut direct = new_order_tree(order_tree_type);
        let mut oracle_pegged = new_order_tree(order_tree_type);
        let new_leaf =
            |key: u128| LeafNode::new(0, key, Pubkey::default(), 0, 0, 1, OrderType::Limit, 0, 0);

        // add 100 leaves to each BookSide, mostly random
        let mut keys = vec![];

        // ensure at least one oracle pegged order visible even at oracle price 1
        let key = new_node_key(side, oracle_pegged_price_data(20), 0);
        keys.push(key);
        oracle_pegged.insert_leaf(&new_leaf(key)).unwrap();

        while oracle_pegged.leaf_count < 100 {
            let price_data: u64 = oracle_pegged_price_data(rng.gen_range(-20..20));
            let seq_num: u64 = rng.gen_range(0..1000);
            let key = new_node_key(side, price_data, seq_num);
            if keys.contains(&key) {
                continue;
            }
            keys.push(key);
            oracle_pegged.insert_leaf(&new_leaf(key)).unwrap();
        }

        while direct.leaf_count < 100 {
            let price_data: u64 = rng.gen_range(1..50);
            let seq_num: u64 = rng.gen_range(0..1000);
            let key = new_node_key(side, price_data, seq_num);
            if keys.contains(&key) {
                continue;
            }
            keys.push(key);
            direct.insert_leaf(&new_leaf(key)).unwrap();
        }

        let bookside = BookSideRef {
            fixed: &direct,
            oracle_pegged: &oracle_pegged,
        };

        // verify iteration order for different oracle prices
        for oracle_price_lots in 1..40 {
            println!("oracle {oracle_price_lots}");
            let mut total = 0;
            let ascending = order_tree_type == OrderTreeType::Asks;
            let mut last_price = if ascending { 0 } else { i64::MAX };
            for order in bookside.iter_all_including_invalid(0, oracle_price_lots) {
                let price = order.price_lots;
                println!("{} {:?} {price}", order.node.key, order.handle.order_tree);
                if ascending {
                    assert!(price >= last_price);
                } else {
                    assert!(price <= last_price);
                }
                last_price = price;
                total += 1;
            }
            assert!(total >= 101); // some oracle peg orders could be skipped
            if oracle_price_lots > 20 {
                assert_eq!(total, 200);
            }
        }
    }

    #[test]
    fn bookside2_iteration_random() {
        bookside2_iteration_random_helper(Side::Bid);
        bookside2_iteration_random_helper(Side::Ask);
    }
}
