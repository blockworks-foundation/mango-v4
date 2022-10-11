use anchor_lang::prelude::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use super::*;

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
            nodes: [AnyNode::zeroed(); MAX_ORDERTREE_NODES],
            reserved: [0; 256],
        }
    }

    fn bookside_iteration_random_helper(side: Side) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let order_tree_type = match side {
            Side::Bid => OrderTreeType::Bids,
            Side::Ask => OrderTreeType::Asks,
        };

        let mut fixed = new_order_tree(order_tree_type);
        let mut oracle_pegged = new_order_tree(order_tree_type);
        let new_leaf = |key: u128| {
            LeafNode::new(
                0,
                key,
                Pubkey::default(),
                0,
                0,
                1,
                PostOrderType::Limit,
                0,
                -1,
            )
        };

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

        while fixed.leaf_count < 100 {
            let price_data: u64 = rng.gen_range(1..50);
            let seq_num: u64 = rng.gen_range(0..1000);
            let key = new_node_key(side, price_data, seq_num);
            if keys.contains(&key) {
                continue;
            }
            keys.push(key);
            fixed.insert_leaf(&new_leaf(key)).unwrap();
        }

        let bookside = BookSideRef {
            fixed: &fixed,
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
    fn bookside_iteration_random() {
        bookside_iteration_random_helper(Side::Bid);
        bookside_iteration_random_helper(Side::Ask);
    }

    #[test]
    fn bookside_order_filtering() {
        let side = Side::Bid;
        let order_tree_type = OrderTreeType::Bids;

        let mut fixed = new_order_tree(order_tree_type);
        let mut oracle_pegged = new_order_tree(order_tree_type);
        let new_node = |key: u128, tif: u8, peg_limit: i64| {
            LeafNode::new(
                0,
                key,
                Pubkey::default(),
                0,
                0,
                1000,
                PostOrderType::Limit,
                tif,
                peg_limit,
            )
        };
        let mut add_fixed = |price: i64, tif: u8| {
            let key = new_node_key(side, fixed_price_data(price).unwrap(), 0);
            fixed.insert_leaf(&new_node(key, tif, -1)).unwrap();
        };
        let mut add_pegged = |price_offset: i64, tif: u8, peg_limit: i64| {
            let key = new_node_key(side, oracle_pegged_price_data(price_offset), 0);
            oracle_pegged
                .insert_leaf(&new_node(key, tif, peg_limit))
                .unwrap();
        };

        add_fixed(100, 0);
        add_fixed(120, 5);
        add_pegged(-10, 0, 100);
        add_pegged(-15, 0, -1);
        add_pegged(-20, 7, 95);

        let bookside = BookSideRef {
            fixed: &fixed,
            oracle_pegged: &oracle_pegged,
        };

        let order_prices = |now_ts: u64, oracle: i64| -> Vec<i64> {
            bookside
                .iter_valid(now_ts, oracle)
                .map(|it| it.price_lots)
                .collect()
        };

        assert_eq!(order_prices(0, 100), vec![120, 100, 90, 85, 80]);
        assert_eq!(order_prices(1004, 100), vec![120, 100, 90, 85, 80]);
        assert_eq!(order_prices(1005, 100), vec![100, 90, 85, 80]);
        assert_eq!(order_prices(1006, 100), vec![100, 90, 85, 80]);
        assert_eq!(order_prices(1007, 100), vec![100, 90, 85]);
        assert_eq!(order_prices(0, 110), vec![120, 100, 100, 95, 90]);
        assert_eq!(order_prices(0, 111), vec![120, 100, 96, 91]);
        assert_eq!(order_prices(0, 115), vec![120, 100, 100, 95]);
        assert_eq!(order_prices(0, 116), vec![120, 101, 100]);
        assert_eq!(order_prices(0, 2015), vec![2000, 120, 100]);
        assert_eq!(order_prices(1010, 2015), vec![2000, 100]);
    }
}
