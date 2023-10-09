pub use book::*;
pub use bookside::*;
pub use bookside_iterator::*;
pub use nodes::*;
pub use order::*;
pub use order_type::*;
pub use ordertree::*;
pub use ordertree_iterator::*;
pub use queue::*;

mod book;
mod bookside;
mod bookside_iterator;
mod nodes;
mod order;
mod order_type;
mod ordertree;
mod ordertree_iterator;
mod queue;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Group, MangoAccount, MangoAccountValue, PerpMarket, FREE_ORDER_SLOT};
    use anchor_lang::prelude::*;
    use bytemuck::Zeroable;
    use fixed::types::I80F48;
    use solana_program::pubkey::Pubkey;
    use std::cell::RefCell;

    fn order_tree_leaf_by_key(bookside: &BookSide, key: u128) -> Option<&LeafNode> {
        for component in [BookSideOrderTree::Fixed, BookSideOrderTree::OraclePegged] {
            for (_, leaf) in bookside.nodes.iter(bookside.root(component)) {
                if leaf.key == key {
                    return Some(leaf);
                }
            }
        }
        None
    }

    fn order_tree_contains_key(bookside: &BookSide, key: u128) -> bool {
        order_tree_leaf_by_key(bookside, key).is_some()
    }

    fn order_tree_contains_price(bookside: &BookSide, price_data: u64) -> bool {
        for component in [BookSideOrderTree::Fixed, BookSideOrderTree::OraclePegged] {
            for (_, leaf) in bookside.nodes.iter(bookside.root(component)) {
                if leaf.price_data() == price_data {
                    return true;
                }
            }
        }
        false
    }

    struct OrderbookAccounts {
        bids: Box<RefCell<BookSide>>,
        asks: Box<RefCell<BookSide>>,
    }

    impl OrderbookAccounts {
        fn new() -> Self {
            let s = Self {
                bids: Box::new(RefCell::new(BookSide::zeroed())),
                asks: Box::new(RefCell::new(BookSide::zeroed())),
            };
            s.bids.borrow_mut().nodes.order_tree_type = OrderTreeType::Bids.into();
            s.asks.borrow_mut().nodes.order_tree_type = OrderTreeType::Asks.into();
            s
        }

        fn orderbook(&self) -> Orderbook {
            Orderbook {
                bids: self.bids.borrow_mut(),
                asks: self.asks.borrow_mut(),
            }
        }
    }

    fn test_setup(price: f64) -> (PerpMarket, I80F48, EventQueue, OrderbookAccounts) {
        let book = OrderbookAccounts::new();

        let event_queue = EventQueue::zeroed();

        let oracle_price = I80F48::from_num(price);

        let mut perp_market = PerpMarket::zeroed();
        perp_market.quote_lot_size = 1;
        perp_market.base_lot_size = 1;
        perp_market.maint_base_asset_weight = I80F48::ONE;
        perp_market.maint_base_liab_weight = I80F48::ONE;
        perp_market.init_base_asset_weight = I80F48::ONE;
        perp_market.init_base_liab_weight = I80F48::ONE;

        (perp_market, oracle_price, event_queue, book)
    }

    // Check what happens when one side of the book fills up
    #[test]
    fn book_bids_full() {
        let (mut perp_market, oracle_price, mut event_queue, book_accs) = test_setup(5000.0);
        let mut book = book_accs.orderbook();
        let settle_token_index = 0;

        let mut new_order = |book: &mut Orderbook,
                             event_queue: &mut EventQueue,
                             side,
                             price_lots,
                             now_ts|
         -> u128 {
            let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            let mut account = MangoAccountValue::from_bytes(&buffer).unwrap();
            account
                .ensure_perp_position(perp_market.perp_market_index, settle_token_index)
                .unwrap();

            let max_base_lots = 1;
            let time_in_force = 100;

            book.new_order(
                Order {
                    side,
                    max_base_lots,
                    max_quote_lots: i64::MAX,
                    client_order_id: 0,
                    time_in_force,
                    reduce_only: false,
                    self_trade_behavior: SelfTradeBehavior::DecrementTake,
                    params: OrderParams::Fixed {
                        price_lots,
                        order_type: PostOrderType::Limit,
                    },
                },
                &mut perp_market,
                event_queue,
                oracle_price,
                &mut account.borrow_mut(),
                &Pubkey::default(),
                now_ts,
                u8::MAX,
            )
            .unwrap();
            account.perp_order_by_raw_index_unchecked(0).id
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
        assert_eq!(
            book.bids
                .nodes
                .min_leaf(&book.bids.roots[0])
                .unwrap()
                .1
                .price_data(),
            1001
        );
        assert_eq!(
            fixed_price_lots(
                book.bids
                    .nodes
                    .max_leaf(&book.bids.roots[0])
                    .unwrap()
                    .1
                    .price_data()
            ),
            (1000 + book.bids.roots[0].leaf_count) as i64
        );

        // add another bid at a higher price before expiry, replacing the lowest-price one (1001)
        new_order(&mut book, &mut event_queue, Side::Bid, 1005, 1000000 - 1);
        assert_eq!(
            book.bids
                .nodes
                .min_leaf(&book.bids.roots[0])
                .unwrap()
                .1
                .price_data(),
            1002
        );
        assert_eq!(event_queue.len(), 1);

        // adding another bid after expiry removes the soonest-expiring order (1005)
        new_order(&mut book, &mut event_queue, Side::Bid, 999, 2000000);
        assert_eq!(
            book.bids
                .nodes
                .min_leaf(&book.bids.roots[0])
                .unwrap()
                .1
                .price_data(),
            999
        );
        assert!(!order_tree_contains_key(&book.bids, 1005));
        assert_eq!(event_queue.len(), 2);

        // adding an ask will wipe up to three expired bids at the top of the book
        let bids_max = book
            .bids
            .nodes
            .max_leaf(&book.bids.roots[0])
            .unwrap()
            .1
            .price_data();
        let bids_count = book.bids.roots[0].leaf_count;
        new_order(&mut book, &mut event_queue, Side::Ask, 6000, 1500000);
        assert_eq!(book.bids.roots[0].leaf_count, bids_count - 5);
        assert_eq!(book.asks.roots[0].leaf_count, 1);
        assert_eq!(event_queue.len(), 2 + 5);
        assert!(!order_tree_contains_price(&book.bids, bids_max));
        assert!(!order_tree_contains_price(&book.bids, bids_max - 1));
        assert!(!order_tree_contains_price(&book.bids, bids_max - 2));
        assert!(!order_tree_contains_price(&book.bids, bids_max - 3));
        assert!(!order_tree_contains_price(&book.bids, bids_max - 4));
        assert!(order_tree_contains_price(&book.bids, bids_max - 5));
    }

    #[test]
    fn book_new_order() {
        let group = Group::zeroed();
        let (mut market, oracle_price, mut event_queue, book_accs) = test_setup(1000.0);
        let mut book = book_accs.orderbook();
        let settle_token_index = 0;

        // Add lots and fees to make sure to exercise unit conversion
        market.base_lot_size = 10;
        market.quote_lot_size = 100;
        let maker_fee = I80F48::from_num(-0.001f32);
        let taker_fee = I80F48::from_num(0.01f32);
        market.maker_fee = maker_fee;
        market.taker_fee = taker_fee;

        let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut maker = MangoAccountValue::from_bytes(&buffer).unwrap();
        let mut taker = MangoAccountValue::from_bytes(&buffer).unwrap();
        maker
            .ensure_perp_position(market.perp_market_index, settle_token_index)
            .unwrap();
        taker
            .ensure_perp_position(market.perp_market_index, settle_token_index)
            .unwrap();

        let maker_pk = Pubkey::new_unique();
        let taker_pk = Pubkey::new_unique();
        let now_ts = 1000000;

        // Place a maker-bid
        let price_lots = 1000 * market.base_lot_size / market.quote_lot_size;
        let bid_quantity = 10;
        book.new_order(
            Order {
                side: Side::Bid,
                max_base_lots: bid_quantity,
                max_quote_lots: i64::MAX,
                client_order_id: 42,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::DecrementTake,
                params: OrderParams::Fixed {
                    price_lots,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut maker.borrow_mut(),
            &maker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();
        let order =
            order_tree_leaf_by_key(&book.bids, maker.perp_order_by_raw_index_unchecked(0).id)
                .unwrap();
        assert_eq!(order.client_order_id, 42);
        assert_eq!(order.quantity, bid_quantity);
        assert_eq!(
            maker.perp_order_mut_by_raw_index(0).market,
            market.perp_market_index
        );
        assert_eq!(maker.perp_order_mut_by_raw_index(1).market, FREE_ORDER_SLOT);
        assert_ne!(maker.perp_order_mut_by_raw_index(0).id, 0);
        assert_eq!(maker.perp_order_mut_by_raw_index(0).client_id, 42);
        assert_eq!(
            maker.perp_order_mut_by_raw_index(0).side_and_tree(),
            SideAndOrderTree::BidFixed
        );
        assert!(order_tree_contains_key(
            &book.bids,
            maker.perp_order_mut_by_raw_index(0).id
        ));
        assert!(order_tree_contains_price(&book.bids, price_lots as u64));
        assert_eq!(
            maker.perp_position_by_raw_index_unchecked(0).bids_base_lots,
            bid_quantity
        );
        assert_eq!(
            maker.perp_position_by_raw_index_unchecked(0).asks_base_lots,
            0
        );
        assert_eq!(
            maker
                .perp_position_by_raw_index_unchecked(0)
                .taker_base_lots,
            0
        );
        assert_eq!(
            maker
                .perp_position_by_raw_index_unchecked(0)
                .taker_quote_lots,
            0
        );
        assert_eq!(
            maker
                .perp_position_by_raw_index_unchecked(0)
                .base_position_lots(),
            0
        );
        assert_eq!(
            maker
                .perp_position_by_raw_index_unchecked(0)
                .quote_position_native()
                .to_num::<u32>(),
            0
        );
        assert_eq!(event_queue.len(), 0);

        // Take the order partially
        let match_quantity = 5;
        book.new_order(
            Order {
                side: Side::Ask,
                max_base_lots: match_quantity,
                max_quote_lots: i64::MAX,
                client_order_id: 43,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::DecrementTake,
                params: OrderParams::Fixed {
                    price_lots,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker.borrow_mut(),
            &taker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();
        // the remainder of the maker order is still on the book
        // (the maker account is unchanged: it was not even passed in)
        let order =
            order_tree_leaf_by_key(&book.bids, maker.perp_order_by_raw_index_unchecked(0).id)
                .unwrap();
        assert_eq!(fixed_price_lots(order.price_data()), price_lots);
        assert_eq!(order.quantity, bid_quantity - match_quantity);

        // fees were immediately accrued
        let match_quote = I80F48::from(match_quantity * price_lots * market.quote_lot_size);
        assert_eq!(market.fees_accrued, match_quote * (maker_fee + taker_fee));

        // the taker account is updated
        assert_eq!(
            taker.perp_order_by_raw_index_unchecked(0).market,
            FREE_ORDER_SLOT
        );
        assert_eq!(
            taker.perp_position_by_raw_index_unchecked(0).bids_base_lots,
            0
        );
        assert_eq!(
            taker.perp_position_by_raw_index_unchecked(0).asks_base_lots,
            0
        );
        assert_eq!(
            taker
                .perp_position_by_raw_index_unchecked(0)
                .taker_base_lots,
            -match_quantity
        );
        assert_eq!(
            taker
                .perp_position_by_raw_index_unchecked(0)
                .taker_quote_lots,
            match_quantity * price_lots
        );
        assert_eq!(
            taker
                .perp_position_by_raw_index_unchecked(0)
                .base_position_lots(),
            0
        );
        assert_eq!(
            taker
                .perp_position_by_raw_index_unchecked(0)
                .quote_position_native(),
            -match_quote * taker_fee
        );

        // the fill gets added to the event queue
        assert_eq!(event_queue.len(), 1);
        let event = event_queue.peek_front().unwrap();
        assert_eq!(event.event_type, EventType::Fill as u8);
        let fill: &FillEvent = bytemuck::cast_ref(event);
        assert_eq!(fill.quantity, match_quantity);
        assert_eq!(fill.price, price_lots);
        assert_eq!(fill.taker_client_order_id, 43);
        assert_eq!(fill.maker, maker_pk);
        assert_eq!(fill.taker, taker_pk);
        assert_eq!(fill.maker_fee, maker_fee.to_num::<f32>());
        assert_eq!(fill.taker_fee, taker_fee.to_num::<f32>());

        // simulate event queue processing
        maker
            .execute_perp_maker(market.perp_market_index, &mut market, fill, &group)
            .unwrap();
        taker
            .execute_perp_taker(market.perp_market_index, &mut market, fill)
            .unwrap();
        assert_eq!(market.open_interest, 2 * match_quantity);

        assert_eq!(maker.perp_order_by_raw_index_unchecked(0).market, 0);
        assert_eq!(
            maker.perp_position_by_raw_index_unchecked(0).bids_base_lots,
            bid_quantity - match_quantity
        );
        assert_eq!(
            maker.perp_position_by_raw_index_unchecked(0).asks_base_lots,
            0
        );
        assert_eq!(
            maker
                .perp_position_by_raw_index_unchecked(0)
                .taker_base_lots,
            0
        );
        assert_eq!(
            maker
                .perp_position_by_raw_index_unchecked(0)
                .taker_quote_lots,
            0
        );
        assert_eq!(
            maker
                .perp_position_by_raw_index_unchecked(0)
                .base_position_lots(),
            match_quantity
        );
        assert_eq!(
            maker
                .perp_position_by_raw_index_unchecked(0)
                .quote_position_native(),
            -match_quote - match_quote * maker_fee
        );

        assert_eq!(
            taker.perp_position_by_raw_index_unchecked(0).bids_base_lots,
            0
        );
        assert_eq!(
            taker.perp_position_by_raw_index_unchecked(0).asks_base_lots,
            0
        );
        assert_eq!(
            taker
                .perp_position_by_raw_index_unchecked(0)
                .taker_base_lots,
            0
        );
        assert_eq!(
            taker
                .perp_position_by_raw_index_unchecked(0)
                .taker_quote_lots,
            0
        );
        assert_eq!(
            taker
                .perp_position_by_raw_index_unchecked(0)
                .base_position_lots(),
            -match_quantity
        );
        assert_eq!(
            taker
                .perp_position_by_raw_index_unchecked(0)
                .quote_position_native(),
            match_quote - match_quote * taker_fee
        );
    }

    #[test]
    fn test_fee_penalty_applied_only_on_limit_order() -> Result<()> {
        // setup market
        let (mut market, oracle_price, mut event_queue, book_accs) = test_setup(1000.0);
        let mut book = book_accs.orderbook();
        let now_ts = 1000000;
        market.taker_fee = I80F48::from_num(0.01);
        market.maker_fee = I80F48::from_num(0.0);
        market.fee_penalty = 5.0;

        // setup maker account
        let maker_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut maker_account = MangoAccountValue::from_bytes(&maker_buffer).unwrap();
        let maker_pk = Pubkey::new_unique();
        maker_account.ensure_perp_position(market.perp_market_index, 0)?;

        // setup taker account
        let taker_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut taker_account = MangoAccountValue::from_bytes(&taker_buffer).unwrap();
        let taker_pk = Pubkey::new_unique();
        taker_account.ensure_perp_position(market.perp_market_index, 0)?;

        // Maker order
        book.new_order(
            Order {
                side: Side::Ask,
                max_base_lots: 2,
                max_quote_lots: i64::MAX,
                client_order_id: 42,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::default(),
                params: OrderParams::Fixed {
                    price_lots: 1000,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut maker_account.borrow_mut(),
            &maker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        // Partial taker
        book.new_order(
            Order {
                side: Side::Bid,
                max_base_lots: 1,
                max_quote_lots: i64::MAX,
                client_order_id: 43,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::DecrementTake,
                params: OrderParams::Fixed {
                    price_lots: 1000,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker_account.borrow_mut(),
            &taker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        let pos = taker_account.perp_position(market.perp_market_index)?;

        assert_eq!(
            pos.quote_position_native().round(),
            I80F48::from_num(-10),
            "Regular fees applied on limit order"
        );

        assert_eq!(
            market.fees_accrued.round(),
            I80F48::from_num(10),
            "Fees moved to market"
        );

        // Full taker
        book.new_order(
            Order {
                side: Side::Bid,
                max_base_lots: 1,
                max_quote_lots: i64::MAX,
                client_order_id: 44,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::DecrementTake,
                params: OrderParams::ImmediateOrCancel { price_lots: 1000 },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker_account.borrow_mut(),
            &taker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        let pos = taker_account.perp_position(market.perp_market_index)?;

        assert_eq!(
            pos.quote_position_native().round(),
            I80F48::from_num(-25), // -10 - 5
            "No fees, but fixed penalty applied on IOC order"
        );

        assert_eq!(
            market.fees_accrued.round(),
            I80F48::from_num(25), // 10 + 5
            "Fees moved to market"
        );

        Ok(())
    }

    // Check that there are no zero-quantity fills when max_quote_lots is not
    // enough for a single lot
    #[test]
    fn book_max_quote_lots() {
        let (mut perp_market, oracle_price, mut event_queue, book_accs) = test_setup(5000.0);
        let mut book = book_accs.orderbook();
        let settle_token_index = 0;

        let mut new_order = |book: &mut Orderbook,
                             event_queue: &mut EventQueue,
                             side,
                             price_lots,
                             max_base_lots: i64,
                             max_quote_lots: i64|
         -> u128 {
            let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            let mut account = MangoAccountValue::from_bytes(&buffer).unwrap();
            account
                .ensure_perp_position(perp_market.perp_market_index, settle_token_index)
                .unwrap();

            book.new_order(
                Order {
                    side,
                    max_base_lots,
                    max_quote_lots,
                    client_order_id: 0,
                    time_in_force: 0,
                    reduce_only: false,
                    self_trade_behavior: SelfTradeBehavior::DecrementTake,
                    params: OrderParams::Fixed {
                        price_lots,
                        order_type: PostOrderType::Limit,
                    },
                },
                &mut perp_market,
                event_queue,
                oracle_price,
                &mut account.borrow_mut(),
                &Pubkey::default(),
                0, // now_ts
                u8::MAX,
            )
            .unwrap();
            account.perp_order_by_raw_index_unchecked(0).id
        };

        // Setup
        new_order(&mut book, &mut event_queue, Side::Ask, 5000, 5, i64::MAX);
        new_order(&mut book, &mut event_queue, Side::Ask, 5001, 5, i64::MAX);
        new_order(&mut book, &mut event_queue, Side::Ask, 5002, 5, i64::MAX);

        // Try taking: the quote limit allows only one base lot to be taken.
        new_order(&mut book, &mut event_queue, Side::Bid, 5005, 30, 6000);
        // Only one fill event is generated, the matching aborts even though neither the base nor quote limit
        // is exhausted.
        assert_eq!(event_queue.len(), 1);

        // Try taking: the quote limit allows no fills
        new_order(&mut book, &mut event_queue, Side::Bid, 5005, 30, 1);
        assert_eq!(event_queue.len(), 1);
    }

    #[test]
    fn test_self_trade_decrement_take() -> Result<()> {
        // setup market
        let (mut market, oracle_price, mut event_queue, book_accs) = test_setup(1000.0);
        let mut book = book_accs.orderbook();
        let now_ts = 1000000;
        market.taker_fee = I80F48::from_num(0.01);
        market.maker_fee = I80F48::from_num(0.0);
        market.fee_penalty = 5.0;

        // setup maker account
        let maker_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut maker_account = MangoAccountValue::from_bytes(&maker_buffer).unwrap();
        let maker_pk = Pubkey::new_unique();
        maker_account.ensure_perp_position(market.perp_market_index, 0)?;

        // setup taker account
        let taker_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut taker_account = MangoAccountValue::from_bytes(&taker_buffer).unwrap();
        let taker_pk = Pubkey::new_unique();
        taker_account.ensure_perp_position(market.perp_market_index, 0)?;

        // taker limit order
        book.new_order(
            Order {
                side: Side::Ask,
                max_base_lots: 2,
                max_quote_lots: i64::MAX,
                client_order_id: 1,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::default(),
                params: OrderParams::Fixed {
                    price_lots: 1000,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker_account.borrow_mut(),
            &taker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        // maker limit order
        book.new_order(
            Order {
                side: Side::Ask,
                max_base_lots: 2,
                max_quote_lots: i64::MAX,
                client_order_id: 2,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::default(),
                params: OrderParams::Fixed {
                    price_lots: 1000,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut maker_account.borrow_mut(),
            &maker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        // taker full self-trade IOC
        book.new_order(
            Order {
                side: Side::Bid,
                max_base_lots: 1,
                max_quote_lots: i64::MAX,
                client_order_id: 3,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::DecrementTake,
                params: OrderParams::ImmediateOrCancel { price_lots: 1000 },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker_account.borrow_mut(),
            &taker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        let pos = taker_account.perp_position(market.perp_market_index)?;

        assert_eq!(
            pos.quote_position_native().round(),
            I80F48::from_num(-5),
            "Penalty applied on ioc self-trade"
        );

        assert_eq!(
            market.fees_accrued.round(),
            I80F48::from_num(5),
            "Fees moved to market"
        );

        let fill_event: FillEvent = event_queue.pop_front()?.try_into()?;
        assert_eq!(fill_event.quantity, 1);
        assert_eq!(fill_event.maker, taker_pk);
        assert_eq!(fill_event.taker, taker_pk);
        assert_eq!(fill_event.maker_fee, I80F48::ZERO);
        assert_eq!(fill_event.taker_fee, I80F48::ZERO);

        //  taker partial self trade limit
        book.new_order(
            Order {
                side: Side::Bid,
                max_base_lots: 2,
                max_quote_lots: i64::MAX,
                client_order_id: 4,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::DecrementTake,
                params: OrderParams::Fixed {
                    price_lots: 1000,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker_account.borrow_mut(),
            &taker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        let pos = taker_account.perp_position(market.perp_market_index)?;

        assert_eq!(
            pos.quote_position_native().round(),
            I80F48::from_num(-15), // -0 -10
            "No fees for self-trade but for maker match"
        );

        assert_eq!(
            market.fees_accrued.round(),
            I80F48::from_num(15), // 0 +10
            "Fees moved to market"
        );

        let fill_event: FillEvent = event_queue.pop_front()?.try_into()?;
        assert_eq!(fill_event.quantity, 1);
        assert_eq!(fill_event.maker, taker_pk);
        assert_eq!(fill_event.taker, taker_pk);
        assert_eq!(fill_event.maker_fee, I80F48::ZERO);
        assert_eq!(fill_event.taker_fee, I80F48::ZERO);

        let fill_event: FillEvent = event_queue.pop_front()?.try_into()?;
        assert_eq!(fill_event.quantity, 1);
        assert_eq!(fill_event.maker, maker_pk);
        assert_eq!(fill_event.taker, taker_pk);
        assert_eq!(fill_event.maker_fee, 0.0);
        assert_eq!(fill_event.taker_fee, 0.01);

        Ok(())
    }

    #[test]
    fn test_self_trade_cancel_provide() -> Result<()> {
        // setup market
        let (mut market, oracle_price, mut event_queue, book_accs) = test_setup(1000.0);
        let mut book = book_accs.orderbook();
        let now_ts = 1000000;
        market.taker_fee = I80F48::from_num(0.01);
        market.maker_fee = I80F48::from_num(0.0);
        market.fee_penalty = 5.0;

        // setup maker account
        let maker_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut maker_account = MangoAccountValue::from_bytes(&maker_buffer).unwrap();
        let maker_pk = Pubkey::new_unique();
        maker_account.ensure_perp_position(market.perp_market_index, 0)?;

        // setup taker account
        let taker_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut taker_account = MangoAccountValue::from_bytes(&taker_buffer).unwrap();
        let taker_pk = Pubkey::new_unique();
        taker_account.ensure_perp_position(market.perp_market_index, 0)?;

        // taker limit order
        book.new_order(
            Order {
                side: Side::Ask,
                max_base_lots: 1,
                max_quote_lots: i64::MAX,
                client_order_id: 1,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::default(),
                params: OrderParams::Fixed {
                    price_lots: 1000,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker_account.borrow_mut(),
            &taker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        // maker limit order
        book.new_order(
            Order {
                side: Side::Ask,
                max_base_lots: 2,
                max_quote_lots: i64::MAX,
                client_order_id: 2,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::default(),
                params: OrderParams::Fixed {
                    price_lots: 1000,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut maker_account.borrow_mut(),
            &maker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        // taker partial self-trade
        book.new_order(
            Order {
                side: Side::Bid,
                max_base_lots: 1,
                max_quote_lots: i64::MAX,
                client_order_id: 3,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::CancelProvide,
                params: OrderParams::Fixed {
                    price_lots: 1000,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker_account.borrow_mut(),
            &taker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        let pos = taker_account.perp_position(market.perp_market_index)?;
        assert_eq!(
            pos.quote_position_native().round(),
            I80F48::from_num(-10), // -0 -10
            "No fees for self-trade but for maker match"
        );

        assert_eq!(
            market.fees_accrued.round(),
            I80F48::from_num(10), // 0 +10
            "Fees moved to market"
        );

        let out_event: OutEvent = event_queue.pop_front()?.try_into()?;
        assert_eq!(out_event.owner, taker_pk);

        let fill_event: FillEvent = event_queue.pop_front()?.try_into()?;
        assert_eq!(fill_event.maker, maker_pk);
        assert_eq!(fill_event.taker, taker_pk);
        assert_eq!(fill_event.quantity, 1);

        Ok(())
    }

    #[test]
    fn test_self_trade_abort_transaction() -> Result<()> {
        // setup market
        let (mut market, oracle_price, mut event_queue, book_accs) = test_setup(1000.0);
        let mut book = book_accs.orderbook();
        let now_ts = 1000000;
        market.taker_fee = I80F48::from_num(0.01);
        market.maker_fee = I80F48::from_num(0.0);
        market.fee_penalty = 5.0;

        // setup maker account
        let maker_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut maker_account = MangoAccountValue::from_bytes(&maker_buffer).unwrap();
        let maker_pk = Pubkey::new_unique();
        maker_account.ensure_perp_position(market.perp_market_index, 0)?;

        // setup taker account
        let taker_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut taker_account = MangoAccountValue::from_bytes(&taker_buffer).unwrap();
        let taker_pk = Pubkey::new_unique();
        taker_account.ensure_perp_position(market.perp_market_index, 0)?;

        // taker limit order
        book.new_order(
            Order {
                side: Side::Ask,
                max_base_lots: 1,
                max_quote_lots: i64::MAX,
                client_order_id: 1,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::default(),
                params: OrderParams::Fixed {
                    price_lots: 1000,
                    order_type: PostOrderType::Limit,
                },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker_account.borrow_mut(),
            &taker_pk,
            now_ts,
            u8::MAX,
        )
        .unwrap();

        // taker failing self-trade
        book.new_order(
            Order {
                side: Side::Bid,
                max_base_lots: 1,
                max_quote_lots: i64::MAX,
                client_order_id: 3,
                time_in_force: 0,
                reduce_only: false,
                self_trade_behavior: SelfTradeBehavior::AbortTransaction,
                params: OrderParams::ImmediateOrCancel { price_lots: 1000 },
            },
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker_account.borrow_mut(),
            &taker_pk,
            now_ts,
            u8::MAX,
        )
        .expect_err("should fail");

        Ok(())
    }
}
