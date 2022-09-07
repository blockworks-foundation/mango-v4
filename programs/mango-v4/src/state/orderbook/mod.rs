pub use book::*;
pub use bookside::*;
pub use bookside_iterator::*;
pub use nodes::*;
pub use order_type::*;
pub use queue::*;

pub mod book;
pub mod bookside;
pub mod bookside_iterator;
pub mod nodes;
pub mod order_type;
pub mod queue;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{MangoAccount, MangoAccountValue, PerpMarket, FREE_ORDER_SLOT};
    use anchor_lang::prelude::*;
    use bytemuck::Zeroable;
    use fixed::types::I80F48;
    use solana_program::pubkey::Pubkey;
    use std::cell::RefCell;

    fn new_bookside(book_side_type: BookSideType) -> BookSide {
        BookSide {
            book_side_type,
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

    fn bookside_leaf_by_key(bookside: &BookSide, key: i128) -> Option<&LeafNode> {
        for (_, leaf) in bookside.iter_all_including_invalid() {
            if leaf.key == key {
                return Some(leaf);
            }
        }
        None
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

    fn test_setup(
        price: f64,
    ) -> (
        PerpMarket,
        I80F48,
        EventQueue,
        RefCell<BookSide>,
        RefCell<BookSide>,
    ) {
        let bids = RefCell::new(new_bookside(BookSideType::Bids));
        let asks = RefCell::new(new_bookside(BookSideType::Asks));

        let event_queue = EventQueue::zeroed();

        let oracle_price = I80F48::from_num(price);

        let mut perp_market = PerpMarket::zeroed();
        perp_market.quote_lot_size = 1;
        perp_market.base_lot_size = 1;
        perp_market.maint_asset_weight = I80F48::ONE;
        perp_market.maint_liab_weight = I80F48::ONE;
        perp_market.init_asset_weight = I80F48::ONE;
        perp_market.init_liab_weight = I80F48::ONE;

        (perp_market, oracle_price, event_queue, bids, asks)
    }

    // Check what happens when one side of the book fills up
    #[test]
    fn book_bids_full() {
        let (mut perp_market, oracle_price, mut event_queue, bids, asks) = test_setup(5000.0);
        let mut book = Book {
            bids: bids.borrow_mut(),
            asks: asks.borrow_mut(),
        };

        let mut new_order =
            |book: &mut Book, event_queue: &mut EventQueue, side, price, now_ts| -> i128 {
                let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
                let mut account = MangoAccountValue::from_bytes(&buffer).unwrap();

                let quantity = 1;
                let tif = 100;

                book.new_order(
                    side,
                    &mut perp_market,
                    event_queue,
                    oracle_price,
                    &mut account.borrow_mut(),
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
                account.perp_order_by_raw_index(0).order_id
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

    #[test]
    fn book_new_order() {
        let (mut market, oracle_price, mut event_queue, bids, asks) = test_setup(1000.0);
        let mut book = Book {
            bids: bids.borrow_mut(),
            asks: asks.borrow_mut(),
        };

        // Add lots and fees to make sure to exercise unit conversion
        market.base_lot_size = 10;
        market.quote_lot_size = 100;
        market.maker_fee = I80F48::from_num(-0.001f64);
        market.taker_fee = I80F48::from_num(0.01f64);

        let buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
        let mut maker = MangoAccountValue::from_bytes(&buffer).unwrap();
        let mut taker = MangoAccountValue::from_bytes(&buffer).unwrap();

        let maker_pk = Pubkey::new_unique();
        let taker_pk = Pubkey::new_unique();
        let now_ts = 1000000;

        // Place a maker-bid
        let price = 1000 * market.base_lot_size / market.quote_lot_size;
        let bid_quantity = 10;
        book.new_order(
            Side::Bid,
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut maker.borrow_mut(),
            &maker_pk,
            price,
            bid_quantity,
            i64::MAX,
            OrderType::Limit,
            0,
            42,
            now_ts,
            u8::MAX,
        )
        .unwrap();
        assert_eq!(
            maker.perp_order_mut_by_raw_index(0).order_market,
            market.perp_market_index
        );
        assert_eq!(
            maker.perp_order_mut_by_raw_index(1).order_market,
            FREE_ORDER_SLOT
        );
        assert_ne!(maker.perp_order_mut_by_raw_index(0).order_id, 0);
        assert_eq!(maker.perp_order_mut_by_raw_index(0).client_order_id, 42);
        assert_eq!(maker.perp_order_mut_by_raw_index(0).order_side, Side::Bid);
        assert!(bookside_contains_key(
            &book.bids,
            maker.perp_order_mut_by_raw_index(0).order_id
        ));
        assert!(bookside_contains_price(&book.bids, price));
        assert_eq!(
            maker.perp_position_by_raw_index(0).bids_base_lots,
            bid_quantity
        );
        assert_eq!(maker.perp_position_by_raw_index(0).asks_base_lots, 0);
        assert_eq!(maker.perp_position_by_raw_index(0).taker_base_lots, 0);
        assert_eq!(maker.perp_position_by_raw_index(0).taker_quote_lots, 0);
        assert_eq!(maker.perp_position_by_raw_index(0).base_position_lots(), 0);
        assert_eq!(
            maker
                .perp_position_by_raw_index(0)
                .quote_position_native()
                .to_num::<u32>(),
            0
        );
        assert_eq!(event_queue.len(), 0);

        // Take the order partially
        let match_quantity = 5;
        book.new_order(
            Side::Ask,
            &mut market,
            &mut event_queue,
            oracle_price,
            &mut taker.borrow_mut(),
            &taker_pk,
            price,
            match_quantity,
            i64::MAX,
            OrderType::Limit,
            0,
            43,
            now_ts,
            u8::MAX,
        )
        .unwrap();
        // the remainder of the maker order is still on the book
        // (the maker account is unchanged: it was not even passed in)
        let order =
            bookside_leaf_by_key(&book.bids, maker.perp_order_by_raw_index(0).order_id).unwrap();
        assert_eq!(order.price(), price);
        assert_eq!(order.quantity, bid_quantity - match_quantity);

        // fees were immediately accrued
        let match_quote = I80F48::from(match_quantity * price * market.quote_lot_size);
        assert_eq!(
            market.fees_accrued,
            match_quote * (market.maker_fee + market.taker_fee)
        );

        // the taker account is updated
        assert_eq!(
            taker.perp_order_by_raw_index(0).order_market,
            FREE_ORDER_SLOT
        );
        assert_eq!(taker.perp_position_by_raw_index(0).bids_base_lots, 0);
        assert_eq!(taker.perp_position_by_raw_index(0).asks_base_lots, 0);
        assert_eq!(
            taker.perp_position_by_raw_index(0).taker_base_lots,
            -match_quantity
        );
        assert_eq!(
            taker.perp_position_by_raw_index(0).taker_quote_lots,
            match_quantity * price
        );
        assert_eq!(taker.perp_position_by_raw_index(0).base_position_lots(), 0);
        assert_eq!(
            taker.perp_position_by_raw_index(0).quote_position_native(),
            -match_quote * market.taker_fee
        );

        // the fill gets added to the event queue
        assert_eq!(event_queue.len(), 1);
        let event = event_queue.peek_front().unwrap();
        assert_eq!(event.event_type, EventType::Fill as u8);
        let fill: &FillEvent = bytemuck::cast_ref(event);
        assert_eq!(fill.quantity, match_quantity);
        assert_eq!(fill.price, price);
        assert_eq!(fill.taker_client_order_id, 43);
        assert_eq!(fill.maker_client_order_id, 42);
        assert_eq!(fill.maker, maker_pk);
        assert_eq!(fill.taker, taker_pk);
        assert_eq!(fill.maker_fee, market.maker_fee);
        assert_eq!(fill.taker_fee, market.taker_fee);

        // simulate event queue processing
        maker
            .execute_perp_maker(market.perp_market_index, &mut market, fill)
            .unwrap();
        taker
            .execute_perp_taker(market.perp_market_index, &mut market, fill)
            .unwrap();
        assert_eq!(market.open_interest, 2 * match_quantity);

        assert_eq!(maker.perp_order_by_raw_index(0).order_market, 0);
        assert_eq!(
            maker.perp_position_by_raw_index(0).bids_base_lots,
            bid_quantity - match_quantity
        );
        assert_eq!(maker.perp_position_by_raw_index(0).asks_base_lots, 0);
        assert_eq!(maker.perp_position_by_raw_index(0).taker_base_lots, 0);
        assert_eq!(maker.perp_position_by_raw_index(0).taker_quote_lots, 0);
        assert_eq!(
            maker.perp_position_by_raw_index(0).base_position_lots(),
            match_quantity
        );
        assert_eq!(
            maker.perp_position_by_raw_index(0).quote_position_native(),
            -match_quote - match_quote * market.maker_fee
        );

        assert_eq!(taker.perp_position_by_raw_index(0).bids_base_lots, 0);
        assert_eq!(taker.perp_position_by_raw_index(0).asks_base_lots, 0);
        assert_eq!(taker.perp_position_by_raw_index(0).taker_base_lots, 0);
        assert_eq!(taker.perp_position_by_raw_index(0).taker_quote_lots, 0);
        assert_eq!(
            taker.perp_position_by_raw_index(0).base_position_lots(),
            -match_quantity
        );
        assert_eq!(
            taker.perp_position_by_raw_index(0).quote_position_native(),
            match_quote - match_quote * market.taker_fee
        );
    }
}
