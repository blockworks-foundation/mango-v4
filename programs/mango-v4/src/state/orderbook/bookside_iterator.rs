use super::*;

pub struct BookSideIterItem<'a> {
    pub handle: BookSideOrderHandle,
    pub node: &'a LeafNode,
    pub price_lots: i64,
    pub is_valid: bool,
}

/// Iterates the fixed and oracle_pegged OrderTrees simultaneously, allowing users to
/// walk the orderbook without caring about where an order came from.
///
/// This will skip over orders that are not currently matchable, but might be valid
/// in the future.
///
/// This may return invalid orders (tif expired, peg_limit exceeded; see is_valid) which
/// users are supposed to remove from the orderbook if they can.
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
            if node.peg_limit != -1 && side.is_price_better(price, node.peg_limit) {
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
    let price_data = fixed_price_data(price_lots).unwrap();
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
                        price_lots: fixed_price_lots(d_node.price_data()),
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
                    price_lots: fixed_price_lots(node.price_data()),
                    is_valid: node.is_not_expired(self.now_ts),
                })
            }
            (None, None) => None,
        }
    }
}
