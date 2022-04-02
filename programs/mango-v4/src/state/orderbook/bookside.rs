use anchor_lang::prelude::*;
use bytemuck::{cast, cast_mut, cast_ref};

use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::state::orderbook::bookside_iterator::BookSideIter;

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
pub enum BookSideType {
    Bids,
    Asks,
}

/// A binary tree on AnyNode::key()
///
/// The key encodes the price in the top 64 bits.
#[account(zero_copy)]
pub struct BookSide {
    // pub meta_data: MetaData,
    // todo: do we want this type at this level?
    pub book_side_type: BookSideType,
    pub bump_index: usize,
    pub free_list_len: usize,
    pub free_list_head: NodeHandle,
    pub root_node: NodeHandle,
    pub leaf_count: usize,
    pub nodes: [AnyNode; MAX_BOOK_NODES],
}

impl BookSide {
    /// Iterate over all entries in the book filtering out invalid orders
    ///
    /// smallest to highest for asks
    /// highest to smallest for bids
    pub fn iter_valid(&self, now_ts: u64) -> BookSideIter {
        BookSideIter::new(self, now_ts)
    }

    /// Iterate over all entries, including invalid orders
    pub fn iter_all_including_invalid(&self) -> BookSideIter {
        BookSideIter::new(self, 0)
    }

    pub fn get_mut(&mut self, key: NodeHandle) -> Option<&mut AnyNode> {
        let node = &mut self.nodes[key as usize];
        let tag = NodeTag::try_from(node.tag);
        match tag {
            Ok(NodeTag::InnerNode) | Ok(NodeTag::LeafNode) => Some(node),
            _ => None,
        }
    }
    pub fn get(&self, key: NodeHandle) -> Option<&AnyNode> {
        let node = &self.nodes[key as usize];
        let tag = NodeTag::try_from(node.tag);
        match tag {
            Ok(NodeTag::InnerNode) | Ok(NodeTag::LeafNode) => Some(node),
            _ => None,
        }
    }

    pub fn remove_min(&mut self) -> Option<LeafNode> {
        self.remove_by_key(self.get(self.find_min()?)?.key()?)
    }

    pub fn remove_max(&mut self) -> Option<LeafNode> {
        self.remove_by_key(self.get(self.find_max()?)?.key()?)
    }

    pub fn remove_worst(&mut self) -> Option<LeafNode> {
        match self.book_side_type {
            BookSideType::Bids => self.remove_min(),
            BookSideType::Asks => self.remove_max(),
        }
    }

    /// Remove the order with the lowest expiry timestamp, if that's < now_ts.
    pub fn remove_one_expired(&mut self, now_ts: u64) -> Option<LeafNode> {
        let (expired_h, expires_at) = self.find_earliest_expiry()?;
        if expires_at < now_ts {
            self.remove_by_key(self.get(expired_h)?.key()?)
        } else {
            None
        }
    }

    pub fn find_max(&self) -> Option<NodeHandle> {
        self.find_min_max(true)
    }

    pub fn root(&self) -> Option<NodeHandle> {
        if self.leaf_count == 0 {
            None
        } else {
            Some(self.root_node)
        }
    }

    pub fn find_min(&self) -> Option<NodeHandle> {
        self.find_min_max(false)
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
            let root_contents = self.get(current).unwrap(); // should never fail unless book is already fucked
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

    fn find_min_max(&self, find_max: bool) -> Option<NodeHandle> {
        let mut root: NodeHandle = self.root()?;

        let i = if find_max { 1 } else { 0 };
        loop {
            let root_contents = self.get(root).unwrap();
            match root_contents.case().unwrap() {
                NodeRef::Inner(&InnerNode { children, .. }) => {
                    root = children[i];
                }
                _ => return Some(root),
            }
        }
    }

    pub fn get_min(&self) -> Option<&LeafNode> {
        self.get_min_max(false)
    }

    pub fn get_max(&self) -> Option<&LeafNode> {
        self.get_min_max(true)
    }
    pub fn get_min_max(&self, find_max: bool) -> Option<&LeafNode> {
        let mut root: NodeHandle = self.root()?;

        let i = if find_max { 1 } else { 0 };
        loop {
            let root_contents = self.get(root)?;
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

    pub fn remove_by_key(&mut self, search_key: i128) -> Option<LeafNode> {
        // path of InnerNode handles that lead to the removed leaf
        let mut stack: Vec<(NodeHandle, bool)> = vec![];

        // special case potentially removing the root
        let mut parent_h = self.root()?;
        let (mut child_h, mut crit_bit) = match self.get(parent_h).unwrap().case().unwrap() {
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
            match self.get(child_h).unwrap().case().unwrap() {
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
        let other_child_h = self.get(parent_h).unwrap().children().unwrap()[!crit_bit as usize];
        let other_child_node_contents = self.remove(other_child_h).unwrap();
        let new_expiry = other_child_node_contents.earliest_expiry();
        *self.get_mut(parent_h).unwrap() = other_child_node_contents;
        self.leaf_count -= 1;
        let removed_leaf: LeafNode = cast(self.remove(child_h).unwrap());

        // update child min expiry back up to the root
        let outdated_expiry = removed_leaf.expiry();
        stack.pop(); // the final parent has been replaced by the remaining leaf
        self.update_parent_earliest_expiry(&stack, outdated_expiry, new_expiry);

        Some(removed_leaf)
    }

    pub fn remove(&mut self, key: NodeHandle) -> Option<AnyNode> {
        let val = *self.get(key)?;

        self.nodes[key as usize] = cast(FreeNode {
            tag: if self.free_list_len == 0 {
                NodeTag::LastFreeNode.into()
            } else {
                NodeTag::FreeNode.into()
            },
            next: self.free_list_head,
            reserve: [0; 80],
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
                self.bump_index < self.nodes.len() && self.bump_index < (u32::MAX as usize),
                MangoError::SomeError // todo
            );

            self.nodes[self.bump_index] = *val;
            let key = self.bump_index as u32;
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
            let root_contents = *self.get(root).unwrap();
            let root_key = root_contents.key().unwrap();
            if root_key == new_leaf.key {
                // This should never happen because key should never match
                if let Some(NodeRef::Leaf(&old_root_as_leaf)) = root_contents.case() {
                    // clobber the existing leaf
                    *self.get_mut(root).unwrap() = *new_leaf.as_ref();
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
            let crit_bit_mask: i128 = 1i128 << (127 - shared_prefix_len);
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

            let new_root: &mut InnerNode = cast_mut(self.get_mut(root).unwrap());
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
        self.free_list_len <= 1 && self.bump_index >= self.nodes.len() - 1
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
            let parent = self.get_mut(*parent_h).unwrap().as_inner_mut().unwrap();
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
            let contents = *self.get(current).unwrap();
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

#[cfg(test)]
mod tests {
    use super::super::order_type::{OrderType, Side};
    use super::*;
    use bytemuck::Zeroable;

    fn new_bookside(book_side_type: BookSideType) -> BookSide {
        BookSide {
            book_side_type,
            bump_index: 0,
            free_list_len: 0,
            free_list_head: 0,
            root_node: 0,
            leaf_count: 0,
            nodes: [AnyNode::zeroed(); MAX_BOOK_NODES],
        }
    }

    fn verify_bookside(bookside: &BookSide) {
        verify_bookside_invariant(bookside);
        verify_bookside_iteration(bookside);
        verify_bookside_expiry(bookside);
    }

    // check that BookSide binary tree key invariant holds
    fn verify_bookside_invariant(bookside: &BookSide) {
        let r = match bookside.root() {
            Some(h) => h,
            None => return,
        };

        fn recursive_check(bookside: &BookSide, h: NodeHandle) {
            match bookside.get(h).unwrap().case().unwrap() {
                NodeRef::Inner(&inner) => {
                    let left = bookside.get(inner.children[0]).unwrap().key().unwrap();
                    let right = bookside.get(inner.children[1]).unwrap().key().unwrap();

                    // the left and right keys share the InnerNode's prefix
                    assert!((inner.key ^ left).leading_zeros() >= inner.prefix_len);
                    assert!((inner.key ^ right).leading_zeros() >= inner.prefix_len);

                    // the left and right node key have the critbit unset and set respectively
                    let crit_bit_mask: i128 = 1i128 << (127 - inner.prefix_len);
                    assert!(left & crit_bit_mask == 0);
                    assert!(right & crit_bit_mask != 0);

                    recursive_check(bookside, inner.children[0]);
                    recursive_check(bookside, inner.children[1]);
                }
                _ => {}
            }
        }
        recursive_check(bookside, r);
    }

    // check that iteration of bookside has the right order and misses no leaves
    fn verify_bookside_iteration(bookside: &BookSide) {
        let mut total = 0;
        let ascending = bookside.book_side_type == BookSideType::Asks;
        let mut last_key = if ascending { 0 } else { i128::MAX };
        for (_, node) in bookside.iter_all_including_invalid() {
            let key = node.key;
            if ascending {
                assert!(key >= last_key);
            } else {
                assert!(key <= last_key);
            }
            last_key = key;
            total += 1;
        }
        assert_eq!(bookside.leaf_count, total);
    }

    // check that BookSide::child_expiry invariant holds
    fn verify_bookside_expiry(bookside: &BookSide) {
        let r = match bookside.root() {
            Some(h) => h,
            None => return,
        };

        fn recursive_check(bookside: &BookSide, h: NodeHandle) {
            match bookside.get(h).unwrap().case().unwrap() {
                NodeRef::Inner(&inner) => {
                    let left = bookside.get(inner.children[0]).unwrap().earliest_expiry();
                    let right = bookside.get(inner.children[1]).unwrap().earliest_expiry();

                    // child_expiry must hold the expiry of the children
                    assert_eq!(inner.child_earliest_expiry[0], left);
                    assert_eq!(inner.child_earliest_expiry[1], right);

                    recursive_check(bookside, inner.children[0]);
                    recursive_check(bookside, inner.children[1]);
                }
                _ => {}
            }
        }
        recursive_check(bookside, r);
    }

    #[test]
    fn bookside_expiry_manual() {
        let mut bids = new_bookside(BookSideType::Bids);
        let new_expiring_leaf = |key: i128, expiry: u64| {
            LeafNode::new(
                0,
                key,
                Pubkey::default(),
                0,
                0,
                expiry - 1,
                OrderType::Limit,
                1,
            )
        };

        assert!(bids.find_earliest_expiry().is_none());

        bids.insert_leaf(&new_expiring_leaf(0, 5000)).unwrap();
        assert_eq!(bids.find_earliest_expiry().unwrap(), (bids.root_node, 5000));
        verify_bookside(&bids);

        let (new4000_h, _) = bids.insert_leaf(&new_expiring_leaf(1, 4000)).unwrap();
        assert_eq!(bids.find_earliest_expiry().unwrap(), (new4000_h, 4000));
        verify_bookside(&bids);

        let (_new4500_h, _) = bids.insert_leaf(&new_expiring_leaf(2, 4500)).unwrap();
        assert_eq!(bids.find_earliest_expiry().unwrap(), (new4000_h, 4000));
        verify_bookside(&bids);

        let (new3500_h, _) = bids.insert_leaf(&new_expiring_leaf(3, 3500)).unwrap();
        assert_eq!(bids.find_earliest_expiry().unwrap(), (new3500_h, 3500));
        verify_bookside(&bids);
        // the first two levels of the tree are innernodes, with 0;1 on one side and 2;3 on the other
        assert_eq!(
            bids.get_mut(bids.root_node)
                .unwrap()
                .as_inner_mut()
                .unwrap()
                .child_earliest_expiry,
            [4000, 3500]
        );

        bids.remove_by_key(3).unwrap();
        verify_bookside(&bids);
        assert_eq!(
            bids.get_mut(bids.root_node)
                .unwrap()
                .as_inner_mut()
                .unwrap()
                .child_earliest_expiry,
            [4000, 4500]
        );
        assert_eq!(bids.find_earliest_expiry().unwrap().1, 4000);

        bids.remove_by_key(0).unwrap();
        verify_bookside(&bids);
        assert_eq!(
            bids.get_mut(bids.root_node)
                .unwrap()
                .as_inner_mut()
                .unwrap()
                .child_earliest_expiry,
            [4000, 4500]
        );
        assert_eq!(bids.find_earliest_expiry().unwrap().1, 4000);

        bids.remove_by_key(1).unwrap();
        verify_bookside(&bids);
        assert_eq!(bids.find_earliest_expiry().unwrap().1, 4500);

        bids.remove_by_key(2).unwrap();
        verify_bookside(&bids);
        assert!(bids.find_earliest_expiry().is_none());
    }

    #[test]
    fn bookside_expiry_random() {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let mut bids = new_bookside(BookSideType::Bids);
        let new_expiring_leaf = |key: i128, expiry: u64| {
            LeafNode::new(
                0,
                key,
                Pubkey::default(),
                0,
                0,
                expiry - 1,
                OrderType::Limit,
                1,
            )
        };

        // add 200 random leaves
        let mut keys = vec![];
        for _ in 0..200 {
            let key: i128 = rng.gen_range(0..10000); // overlap in key bits
            if keys.contains(&key) {
                continue;
            }
            let expiry = rng.gen_range(1..200); // give good chance of duplicate expiry times
            keys.push(key);
            bids.insert_leaf(&new_expiring_leaf(key, expiry)).unwrap();
            verify_bookside(&bids);
        }

        // remove 50 at random
        for _ in 0..50 {
            if keys.len() == 0 {
                break;
            }
            let k = keys[rng.gen_range(0..keys.len())];
            bids.remove_by_key(k).unwrap();
            keys.retain(|v| *v != k);
            verify_bookside(&bids);
        }
    }

    fn bookside_contains_key(bookside: &BookSide, key: i128) -> bool {
        for (_, leaf) in bookside.iter_all_including_invalid() {
            if leaf.key == key {
                return true;
            }
        }
        false
    }

    fn bookside_contains_price(bookside: &BookSide, price: i64) -> bool {
        for (_, leaf) in bookside.iter_all_including_invalid() {
            if leaf.price() == price {
                return true;
            }
        }
        false
    }

    #[test]
    fn book_bids_full() {
        use super::super::book::Book;
        use super::super::queue::EventQueue;
        use crate::state::{MangoAccountPerps, PerpMarket};
        use fixed::types::I80F48;
        use std::cell::RefCell;

        let bids = RefCell::new(new_bookside(BookSideType::Bids));
        let asks = RefCell::new(new_bookside(BookSideType::Asks));
        let mut book = Book {
            bids: bids.borrow_mut(),
            asks: asks.borrow_mut(),
        };

        let mut event_queue = EventQueue::zeroed();

        let oracle_price = I80F48::from_num(5000.0);

        let mut perp_market = PerpMarket::zeroed();
        perp_market.quote_lot_size = 1;
        perp_market.base_lot_size = 1;
        perp_market.maint_asset_weight = I80F48::ONE;
        perp_market.maint_liab_weight = I80F48::ONE;
        perp_market.init_asset_weight = I80F48::ONE;
        perp_market.init_liab_weight = I80F48::ONE;

        let mut new_order =
            |book: &mut Book, event_queue: &mut EventQueue, side, price, now_ts| -> i128 {
                let mut account_perps = MangoAccountPerps::new();

                let quantity = 1;
                let tif = 100;

                book.new_order(
                    side,
                    &mut perp_market,
                    event_queue,
                    oracle_price,
                    &mut account_perps,
                    &Pubkey::default(),
                    price,
                    quantity,
                    i64::MAX,
                    OrderType::Limit,
                    tif,
                    0,
                    now_ts,
                    u8::MAX,
                )
                .unwrap();
                account_perps.order_id[0]
            };

        // insert bids until book side is full
        for i in 1..10 {
            new_order(
                &mut book,
                &mut event_queue,
                Side::Bid,
                1000 + i as i64,
                1000000 + i as u64,
            );
        }
        for i in 10..1000 {
            new_order(
                &mut book,
                &mut event_queue,
                Side::Bid,
                1000 + i as i64,
                1000011 as u64,
            );
            if book.bids.is_full() {
                break;
            }
        }
        assert!(book.bids.is_full());
        assert_eq!(book.bids.get_min().unwrap().price(), 1001);
        assert_eq!(
            book.bids.get_max().unwrap().price(),
            (1000 + book.bids.leaf_count) as i64
        );

        // add another bid at a higher price before expiry, replacing the lowest-price one (1001)
        new_order(&mut book, &mut event_queue, Side::Bid, 1005, 1000000 - 1);
        assert_eq!(book.bids.get_min().unwrap().price(), 1002);
        assert_eq!(event_queue.len(), 1);

        // adding another bid after expiry removes the soonest-expiring order (1005)
        new_order(&mut book, &mut event_queue, Side::Bid, 999, 2000000);
        assert_eq!(book.bids.get_min().unwrap().price(), 999);
        assert!(!bookside_contains_key(&book.bids, 1005));
        assert_eq!(event_queue.len(), 2);

        // adding an ask will wipe up to three expired bids at the top of the book
        let bids_max = book.bids.get_max().unwrap().price();
        let bids_count = book.bids.leaf_count;
        new_order(&mut book, &mut event_queue, Side::Ask, 6000, 1500000);
        assert_eq!(book.bids.leaf_count, bids_count - 5);
        assert_eq!(book.asks.leaf_count, 1);
        assert_eq!(event_queue.len(), 2 + 5);
        assert!(!bookside_contains_price(&book.bids, bids_max));
        assert!(!bookside_contains_price(&book.bids, bids_max - 1));
        assert!(!bookside_contains_price(&book.bids, bids_max - 2));
        assert!(!bookside_contains_price(&book.bids, bids_max - 3));
        assert!(!bookside_contains_price(&book.bids, bids_max - 4));
        assert!(bookside_contains_price(&book.bids, bids_max - 5));
    }
}
