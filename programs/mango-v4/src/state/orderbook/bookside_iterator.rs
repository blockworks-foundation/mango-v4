use crate::state::orderbook::bookside::*;
use crate::state::orderbook::nodes::*;

/// Iterate over orders in order (bids=descending, asks=ascending)
pub struct BookSideIter<'a> {
    book_side: &'a BookSide,
    /// InnerNodes where the right side still needs to be iterated on
    stack: Vec<&'a InnerNode>,
    /// To be returned on `next()`
    next_leaf: Option<(NodeHandle, &'a LeafNode)>,

    /// either 0, 1 to iterate low-to-high, or 1, 0 to iterate high-to-low
    left: usize,
    right: usize,

    now_ts: u64,
}

impl<'a> BookSideIter<'a> {
    pub fn new(book_side: &'a BookSide, now_ts: u64) -> Self {
        let (left, right) = if book_side.book_side_type == BookSideType::Bids {
            (1, 0)
        } else {
            (0, 1)
        };
        let stack = vec![];

        let mut iter = Self {
            book_side,
            stack,
            next_leaf: None,
            left,
            right,
            now_ts,
        };
        if book_side.leaf_count != 0 {
            iter.next_leaf = iter.find_leftmost_valid_leaf(book_side.root_node);
        }
        iter
    }

    pub fn is_bids(&self) -> bool {
        self.left == 1
    }

    pub fn peek(&self) -> Option<(NodeHandle, &'a LeafNode)> {
        self.next_leaf
    }

    fn find_leftmost_valid_leaf(
        &mut self,
        start: NodeHandle,
    ) -> Option<(NodeHandle, &'a LeafNode)> {
        let mut current = start;
        loop {
            match self.book_side.node(current).unwrap().case().unwrap() {
                NodeRef::Inner(inner) => {
                    self.stack.push(inner);
                    current = inner.children[self.left];
                }
                NodeRef::Leaf(leaf) => {
                    if leaf.is_valid(self.now_ts) {
                        return Some((current, leaf));
                    } else {
                        match self.stack.pop() {
                            None => {
                                return None;
                            }
                            Some(inner) => {
                                current = inner.children[self.right];
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<'a> Iterator for BookSideIter<'a> {
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
                // go down the left branch as much as possible until reaching a valid leaf
                self.find_leftmost_valid_leaf(start)
            }
        };

        current_leaf
    }
}

pub struct BookSide2IterItem<'a> {
    pub handle: BookSide2NodeHandle,
    pub node: &'a LeafNode,
    pub price_lots: i64,
}

pub struct BookSide2Iter<'a> {
    direct_iter: BookSideIter<'a>,
    oracle_pegged_iter: BookSideIter<'a>,
    oracle_price_lots: i64,
}

impl<'a> BookSide2Iter<'a> {
    pub fn new(book_side: &'a BookSide2, now_ts: u64, oracle_price_lots: i64) -> Self {
        Self {
            direct_iter: book_side.direct.iter_valid(now_ts),
            oracle_pegged_iter: book_side.oracle_pegged.iter_valid(now_ts),
            oracle_price_lots,
        }
    }
}

impl<'a> Iterator for BookSide2Iter<'a> {
    type Item = BookSide2IterItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.direct_iter.peek(), self.oracle_pegged_iter.peek()) {
            (Some((d_handle, d_node)), Some((o_handle, o_node))) => {
                let is_better = if self.direct_iter.is_bids() {
                    |a, b| a > b
                } else {
                    |a, b| a < b
                };

                let o_price_maybe = self.oracle_price_lots.checked_add(o_node.data());
                if o_price_maybe.is_none()
                    || is_better(d_node.key, o_node.key_with_data(o_price_maybe.unwrap()))
                {
                    self.direct_iter.next();
                    Some(Self::Item {
                        handle: BookSide2NodeHandle {
                            component: BookSide2Component::Direct,
                            node: d_handle,
                        },
                        node: d_node,
                        price_lots: d_node.data(),
                    })
                } else {
                    self.oracle_pegged_iter.next();
                    Some(Self::Item {
                        handle: BookSide2NodeHandle {
                            component: BookSide2Component::OraclePegged,
                            node: o_handle,
                        },
                        node: o_node,
                        price_lots: o_price_maybe.unwrap(),
                    })
                }
            }
            (None, Some((handle, node))) => {
                self.oracle_pegged_iter.next();
                let price_lots = match self.oracle_price_lots.checked_add(node.data()) {
                    Some(v) => v,
                    None => {
                        return None;
                    }
                };
                Some(Self::Item {
                    handle: BookSide2NodeHandle {
                        component: BookSide2Component::OraclePegged,
                        node: handle,
                    },
                    node,
                    price_lots,
                })
            }
            (Some((handle, node)), None) => {
                self.direct_iter.next();
                Some(Self::Item {
                    handle: BookSide2NodeHandle {
                        component: BookSide2Component::Direct,
                        node: handle,
                    },
                    node,
                    price_lots: node.data(),
                })
            }
            (None, None) => None,
        }
    }
}
