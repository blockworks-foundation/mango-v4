use crate::state::orderbook::bookside::*;
use crate::state::orderbook::nodes::*;
use crate::state::orderbook::order_type::*;

/// Iterate over orders in order (bids=descending, asks=ascending)
pub struct OrderTreeIter<'a> {
    order_tree: &'a OrderTree,
    /// InnerNodes where the right side still needs to be iterated on
    stack: Vec<&'a InnerNode>,
    /// To be returned on `next()`
    next_leaf: Option<(NodeHandle, &'a LeafNode)>,

    /// either 0, 1 to iterate low-to-high, or 1, 0 to iterate high-to-low
    left: usize,
    right: usize,
}

impl<'a> OrderTreeIter<'a> {
    pub fn new(order_tree: &'a OrderTree) -> Self {
        let (left, right) = if order_tree.order_tree_type == OrderTreeType::Bids {
            (1, 0)
        } else {
            (0, 1)
        };
        let stack = vec![];

        let mut iter = Self {
            order_tree,
            stack,
            next_leaf: None,
            left,
            right,
        };
        if order_tree.leaf_count != 0 {
            iter.next_leaf = iter.find_leftmost_leaf(order_tree.root_node);
        }
        iter
    }

    pub fn side(&self) -> Side {
        if self.left == 1 {
            Side::Bid
        } else {
            Side::Ask
        }
    }

    pub fn peek(&self) -> Option<(NodeHandle, &'a LeafNode)> {
        self.next_leaf
    }

    fn find_leftmost_leaf(&mut self, start: NodeHandle) -> Option<(NodeHandle, &'a LeafNode)> {
        let mut current = start;
        loop {
            match self.order_tree.node(current).unwrap().case().unwrap() {
                NodeRef::Inner(inner) => {
                    self.stack.push(inner);
                    current = inner.children[self.left];
                }
                NodeRef::Leaf(leaf) => {
                    return Some((current, leaf));
                }
            }
        }
    }
}

impl<'a> Iterator for OrderTreeIter<'a> {
    type Item = (NodeHandle, &'a LeafNode);

    fn next(&mut self) -> Option<Self::Item> {
        // no next leaf? done
        if self.next_leaf.is_none() {
            return None;
        }

        // start popping from stack and get the other child
        let current_leaf = self.next_leaf;
        self.next_leaf = match self.stack.pop() {
            None => None,
            Some(inner) => {
                let start = inner.children[self.right];
                // go down the left branch as much as possible until reaching a leaf
                self.find_leftmost_leaf(start)
            }
        };

        current_leaf
    }
}

pub struct BookSideIterItem<'a> {
    pub handle: BookSideOrderHandle,
    pub node: &'a LeafNode,
    pub price_lots: i64,
    pub is_valid: bool,
}

pub struct BookSideIter<'a> {
    fixed_iter: OrderTreeIter<'a>,
    oracle_pegged_iter: OrderTreeIter<'a>,
    now_ts: u64,
    oracle_price_lots: i64,
}

impl<'a> BookSideIter<'a> {
    pub fn new(book_side: BookSideRef<'a>, now_ts: u64, oracle_price_lots: i64) -> Self {
        Self {
            fixed_iter: book_side.fixed.iter(),
            oracle_pegged_iter: book_side.oracle_pegged.iter(),
            now_ts,
            oracle_price_lots,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum OrderState {
    Valid,
    Invalid,
    Skipped,
}

fn oracle_pegged_price(
    oracle_price_lots: i64,
    node: &LeafNode,
    side: Side,
) -> (OrderState, Option<i64>) {
    let price_data = node.price_data();
    let price_offset = oracle_pegged_price_offset(price_data);
    if let Some(price) = oracle_price_lots.checked_add(price_offset) {
        if price >= 1 {
            if side.is_price_better(price, node.peg_limit) {
                return (OrderState::Invalid, Some(price));
            } else {
                return (OrderState::Valid, Some(price));
            }
        }
    }
    (OrderState::Skipped, None)
}

fn key_for_price(key: u128, price_lots: i64) -> u128 {
    // We know this can never fail, because oracle pegged price will always be >= 1
    assert!(price_lots >= 1);
    let price_data = direct_price_data(price_lots).unwrap();
    let upper = (price_data as u128) << 64;
    let lower = (key as u64) as u128;
    upper | lower
}

impl<'a> Iterator for BookSideIter<'a> {
    type Item = BookSideIterItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let side = self.fixed_iter.side();

        // Skip all the oracle pegged orders that aren't representable with the current oracle
        // price. Example: iterating asks, but the best ask is at offset -100 with the oracle at 50.
        // We need to skip asks until we find the first that has a price >= 1.
        let mut o_peek = self.oracle_pegged_iter.peek();
        while let Some((_, o_node)) = o_peek {
            if oracle_pegged_price(self.oracle_price_lots, o_node, side).0 != OrderState::Skipped {
                break;
            }
            o_peek = self.oracle_pegged_iter.next()
        }

        match (self.fixed_iter.peek(), o_peek) {
            (Some((d_handle, d_node)), Some((o_handle, o_node))) => {
                let is_better = if side == Side::Bid {
                    |a, b| a > b
                } else {
                    |a, b| a < b
                };

                let (o_valid, o_price_maybe) =
                    oracle_pegged_price(self.oracle_price_lots, o_node, side);
                let o_price = o_price_maybe.unwrap(); // Skipped orders are skipped above
                if is_better(d_node.key, key_for_price(o_node.key, o_price)) {
                    self.fixed_iter.next();
                    Some(Self::Item {
                        handle: BookSideOrderHandle {
                            order_tree: BookSideOrderTree::Fixed,
                            node: d_handle,
                        },
                        node: d_node,
                        price_lots: direct_price_lots(d_node.price_data()),
                        is_valid: d_node.is_not_expired(self.now_ts),
                    })
                } else {
                    self.oracle_pegged_iter.next();
                    Some(Self::Item {
                        handle: BookSideOrderHandle {
                            order_tree: BookSideOrderTree::OraclePegged,
                            node: o_handle,
                        },
                        node: o_node,
                        price_lots: o_price,
                        is_valid: o_valid == OrderState::Valid
                            && o_node.is_not_expired(self.now_ts),
                    })
                }
            }
            (None, Some((handle, node))) => {
                self.oracle_pegged_iter.next();
                let (valid, price_maybe) = oracle_pegged_price(self.oracle_price_lots, node, side);
                let price_lots = price_maybe.unwrap(); // Skipped orders are skipped above
                Some(Self::Item {
                    handle: BookSideOrderHandle {
                        order_tree: BookSideOrderTree::OraclePegged,
                        node: handle,
                    },
                    node,
                    price_lots,
                    is_valid: valid == OrderState::Valid && node.is_not_expired(self.now_ts),
                })
            }
            (Some((handle, node)), None) => {
                self.fixed_iter.next();
                Some(Self::Item {
                    handle: BookSideOrderHandle {
                        order_tree: BookSideOrderTree::Fixed,
                        node: handle,
                    },
                    node,
                    price_lots: direct_price_lots(node.price_data()),
                    is_valid: node.is_not_expired(self.now_ts),
                })
            }
            (None, None) => None,
        }
    }
}
