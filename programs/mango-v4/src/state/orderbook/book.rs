use std::cell::RefMut;

use crate::{
    error::MangoError,
    state::{
        orderbook::{bookside::BookSide, nodes::LeafNode},
        EventQueue, MangoAccount, PerpMarket,
    },
    util::LoadZeroCopy,
};
use anchor_lang::prelude::*;
use bytemuck::cast;
use fixed::types::I80F48;

use super::{
    nodes::NodeHandle,
    order_type::{OrderType, Side},
    FillEvent, OutEvent,
};
use crate::util::checked_math as cm;

/// Drop at most this many expired orders from a BookSide when trying to match orders.
/// This exists as a guard against excessive compute use.
const DROP_EXPIRED_ORDER_LIMIT: usize = 5;

pub struct Book<'a> {
    pub bids: RefMut<'a, BookSide>, // todo: why refmut?
    pub asks: RefMut<'a, BookSide>,
}

impl<'a> Book<'a> {
    pub fn load_mut(
        bids_ai: &'a AccountInfo,
        asks_ai: &'a AccountInfo,
        perp_market: &PerpMarket,
    ) -> std::result::Result<Self, Error> {
        require!(bids_ai.key == &perp_market.bids, MangoError::SomeError);
        require!(asks_ai.key == &perp_market.asks, MangoError::SomeError);
        Ok(Self {
            bids: bids_ai.load_mut::<BookSide>()?,
            asks: asks_ai.load_mut::<BookSide>()?,
        })
    }

    /// Returns best valid bid
    pub fn get_best_bid_price(&self, now_ts: u64) -> Option<i64> {
        Some(self.bids.iter_valid(now_ts).next()?.1.price())
    }

    /// Returns best valid ask
    pub fn get_best_ask_price(&self, now_ts: u64) -> Option<i64> {
        Some(self.asks.iter_valid(now_ts).next()?.1.price())
    }

    /// Get the quantity of valid bids above and including the price
    pub fn get_bids_size_above(&self, price: i64, max_depth: i64, now_ts: u64) -> i64 {
        let mut s = 0;
        for (_, bid) in self.bids.iter_valid(now_ts) {
            if price > bid.price() || s >= max_depth {
                break;
            }
            s += bid.quantity;
        }
        s.min(max_depth)
    }

    /// Walk up the book `quantity` units and return the price at that level. If `quantity` units
    /// not on book, return None
    pub fn get_impact_price(&self, side: Side, quantity: i64, now_ts: u64) -> Option<i64> {
        let mut s = 0;
        let book_side = match side {
            Side::Bid => self.bids.iter_valid(now_ts),
            Side::Ask => self.asks.iter_valid(now_ts),
        };
        for (_, order) in book_side {
            s += order.quantity;
            if s >= quantity {
                return Some(order.price());
            }
        }
        None
    }

    /// Get the quantity of valid asks below and including the price
    pub fn get_asks_size_below(&self, price: i64, max_depth: i64, now_ts: u64) -> i64 {
        let mut s = 0;
        for (_, ask) in self.asks.iter_valid(now_ts) {
            if price < ask.price() || s >= max_depth {
                break;
            }
            s += ask.quantity;
        }
        s.min(max_depth)
    }
    /// Get the quantity of valid bids above this order id. Will return full size of book if order id not found
    pub fn get_bids_size_above_order(&self, order_id: i128, max_depth: i64, now_ts: u64) -> i64 {
        let mut s = 0;
        for (_, bid) in self.bids.iter_valid(now_ts) {
            if bid.key == order_id || s >= max_depth {
                break;
            }
            s += bid.quantity;
        }
        s.min(max_depth)
    }

    /// Get the quantity of valid asks above this order id. Will return full size of book if order id not found
    pub fn get_asks_size_below_order(&self, order_id: i128, max_depth: i64, now_ts: u64) -> i64 {
        let mut s = 0;
        for (_, ask) in self.asks.iter_valid(now_ts) {
            if ask.key == order_id || s >= max_depth {
                break;
            }
            s += ask.quantity;
        }
        s.min(max_depth)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_order(
        &mut self,
        side: Side,
        perp_market: &mut PerpMarket,
        event_queue: &mut EventQueue,
        oracle_price: I80F48,
        mango_account: &mut MangoAccount,
        mango_account_pk: &Pubkey,
        price: i64,
        max_base_quantity: i64,
        max_quote_quantity: i64,
        order_type: OrderType,
        time_in_force: u8,
        client_order_id: u64,
        now_ts: u64,
        limit: u8,
    ) -> std::result::Result<(), Error> {
        match side {
            Side::Bid => self.new_bid(
                perp_market,
                event_queue,
                oracle_price,
                mango_account,
                mango_account_pk,
                price,
                max_base_quantity,
                max_quote_quantity,
                order_type,
                time_in_force,
                client_order_id,
                now_ts,
                limit,
            ),
            Side::Ask => self.new_bid(
                perp_market,
                event_queue,
                oracle_price,
                mango_account,
                mango_account_pk,
                price,
                max_base_quantity,
                max_quote_quantity,
                order_type,
                time_in_force,
                client_order_id,
                now_ts,
                limit,
            ),
        }
    }

    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub fn new_bid(
        &mut self,
        market: &mut PerpMarket,
        event_queue: &mut EventQueue,
        oracle_price: I80F48,
        mango_account: &mut MangoAccount,
        mango_account_pk: &Pubkey,
        price: i64,
        max_base_quantity: i64, // guaranteed to be greater than zero due to initial check
        max_quote_quantity: i64, // guaranteed to be greater than zero due to initial check
        order_type: OrderType,
        time_in_force: u8,
        client_order_id: u64,
        now_ts: u64,
        mut limit: u8, // max number of FillEvents allowed; guaranteed to be greater than 0
    ) -> std::result::Result<(), Error> {
        let (post_only, mut post_allowed, price) = match order_type {
            OrderType::Limit => (false, true, price),
            OrderType::ImmediateOrCancel => (false, false, price),
            OrderType::PostOnly => (true, true, price),
            OrderType::Market => (false, false, i64::MAX),
            OrderType::PostOnlySlide => {
                let price = if let Some(best_ask_price) = self.get_best_ask_price(now_ts) {
                    price.min(best_ask_price.checked_sub(1).ok_or(MangoError::SomeError)?)
                // math_err
                } else {
                    price
                };
                (true, true, price)
            }
        };

        if post_allowed {
            // price limit check computed lazily to save CU on average
            let native_price = market.lot_to_native_price(price);
            if native_price.checked_div(oracle_price).unwrap() > market.maint_liab_weight {
                msg!("Posting on book disallowed due to price limits");
                post_allowed = false;
            }
        }

        // generate new order id
        let order_id = market.gen_order_id(Side::Bid, price);

        // Iterate through book and match against this new bid.
        //
        // Any changes to matching asks are collected in ask_changes
        // and then applied after this loop.
        let mut rem_base_quantity = max_base_quantity; // base lots (aka contracts)
        let mut rem_quote_quantity = max_quote_quantity;
        let mut ask_changes: Vec<(NodeHandle, i64)> = vec![];
        let mut ask_deletes: Vec<i128> = vec![];
        let mut number_of_dropped_expired_orders = 0;
        for (best_ask_h, best_ask) in self.asks.iter_all_including_invalid() {
            if !best_ask.is_valid(now_ts) {
                // Remove the order from the book unless we've done that enough
                if number_of_dropped_expired_orders < DROP_EXPIRED_ORDER_LIMIT {
                    number_of_dropped_expired_orders += 1;
                    let event = OutEvent::new(
                        Side::Ask,
                        now_ts,
                        event_queue.header.seq_num,
                        best_ask.owner,
                        best_ask.quantity,
                    );
                    event_queue.push_back(cast(event)).unwrap();
                    ask_deletes.push(best_ask.key);
                }
                continue;
            }

            let best_ask_price = best_ask.price();

            if price < best_ask_price {
                break;
            } else if post_only {
                msg!("Order could not be placed due to PostOnly");
                post_allowed = false;
                break; // return silently to not fail other instructions in tx
            } else if limit == 0 {
                msg!("Order matching limit reached");
                post_allowed = false;
                break;
            }

            let max_match_by_quote = rem_quote_quantity / best_ask_price;
            let match_quantity = rem_base_quantity
                .min(best_ask.quantity)
                .min(max_match_by_quote);
            let done = match_quantity == max_match_by_quote || match_quantity == rem_base_quantity;

            let match_quote = cm!(match_quantity * best_ask_price);
            rem_base_quantity = cm!(rem_base_quantity - match_quantity);
            rem_quote_quantity = cm!(rem_quote_quantity - match_quote);
            // mango_account.perp_accounts[market_index].add_taker_trade(match_quantity, -match_quote);

            let new_best_ask_quantity = cm!(best_ask.quantity - match_quantity);
            let maker_out = new_best_ask_quantity == 0;
            if maker_out {
                ask_deletes.push(best_ask.key);
            } else {
                ask_changes.push((best_ask_h, new_best_ask_quantity));
            }

            let fill = FillEvent::new(
                Side::Bid,
                maker_out,
                now_ts,
                event_queue.header.seq_num,
                best_ask.owner,
                best_ask.key,
                best_ask.client_order_id,
                market.maker_fee,
                best_ask.timestamp,
                *mango_account_pk,
                order_id,
                client_order_id,
                market.taker_fee,
                best_ask_price,
                match_quantity,
            );
            event_queue.push_back(cast(fill)).unwrap();
            limit -= 1;

            if done {
                break;
            }
        }
        let total_quote_taken = cm!(max_quote_quantity - rem_quote_quantity);

        // Apply changes to matched asks (handles invalidate on delete!)
        for (handle, new_quantity) in ask_changes {
            self.asks
                .get_mut(handle)
                .unwrap()
                .as_leaf_mut()
                .unwrap()
                .quantity = new_quantity;
        }
        for key in ask_deletes {
            let _removed_leaf = self.asks.remove_by_key(key).unwrap();
        }

        // If there are still quantity unmatched, place on the book
        let book_base_quantity = rem_base_quantity.min(rem_quote_quantity / price);
        if post_allowed && book_base_quantity > 0 {
            // Drop an expired order if possible
            if let Some(expired_bid) = self.bids.remove_one_expired(now_ts) {
                let event = OutEvent::new(
                    Side::Bid,
                    now_ts,
                    event_queue.header.seq_num,
                    expired_bid.owner,
                    expired_bid.quantity,
                );
                event_queue.push_back(cast(event)).unwrap();
            }

            if self.bids.is_full() {
                // If this bid is higher than lowest bid, boot that bid and insert this one
                let min_bid = self.bids.remove_min().unwrap();
                // MangoErrorCode::OutOfSpace
                require!(price > min_bid.price(), MangoError::SomeError);
                let event = OutEvent::new(
                    Side::Bid,
                    now_ts,
                    event_queue.header.seq_num,
                    min_bid.owner,
                    min_bid.quantity,
                );
                event_queue.push_back(cast(event)).unwrap();
            }

            let new_bid = LeafNode::new(
                order_id,
                *mango_account_pk,
                book_base_quantity,
                client_order_id,
                now_ts,
                order_type,
                time_in_force,
            );
            let _result = self.bids.insert_leaf(&new_bid)?;

            // TODO OPT remove if PlacePerpOrder needs more compute
            msg!(
                "bid on book order_id={} quantity={} price={}",
                order_id,
                book_base_quantity,
                price
            );
            // mango_account.add_order(market_index, Side::Bid, &new_bid)?;
        }

        // if there were matched taker quote apply ref fees
        // we know ref_fee_rate is not None if total_quote_taken > 0
        if total_quote_taken > 0 {
            apply_fees(market, mango_account, total_quote_taken)?;
        }

        Ok(())
    }

    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub fn new_ask(
        &mut self,
        market: &mut PerpMarket,
        event_queue: &mut EventQueue,
        oracle_price: I80F48,
        mango_account: &mut MangoAccount,
        mango_account_pk: &Pubkey,
        price: i64,
        max_base_quantity: i64, // guaranteed to be greater than zero due to initial check
        max_quote_quantity: i64, // guaranteed to be greater than zero due to initial check
        order_type: OrderType,
        time_in_force: u8,
        client_order_id: u64,
        now_ts: u64,
        mut limit: u8, // max number of FillEvents allowed; guaranteed to be greater than 0
    ) -> std::result::Result<(), Error> {
        let (post_only, mut post_allowed, price) = match order_type {
            OrderType::Limit => (false, true, price),
            OrderType::ImmediateOrCancel => (false, false, price),
            OrderType::PostOnly => (true, true, price),
            OrderType::Market => (false, false, 1),
            OrderType::PostOnlySlide => {
                let price = if let Some(best_bid_price) = self.get_best_bid_price(now_ts) {
                    price.max(best_bid_price.checked_add(1).ok_or(MangoError::SomeError)?)
                } else {
                    price
                };
                (true, true, price)
            }
        };

        if post_allowed {
            // price limit check computed lazily to save CU on average
            let native_price = market.lot_to_native_price(price);
            if native_price.checked_div(oracle_price).unwrap() < market.maint_asset_weight {
                msg!("Posting on book disallowed due to price limits");
                post_allowed = false;
            }
        }

        // generate new order id
        let order_id = market.gen_order_id(Side::Ask, price);

        // Iterate through book and match against this new ask
        //
        // Any changes to matching bids are collected in bid_changes
        // and then applied after this loop.
        let mut rem_base_quantity = max_base_quantity; // base lots (aka contracts)
        let mut rem_quote_quantity = max_quote_quantity;
        let mut bid_changes: Vec<(NodeHandle, i64)> = vec![];
        let mut bid_deletes: Vec<i128> = vec![];
        let mut number_of_dropped_expired_orders = 0;
        for (best_bid_h, best_bid) in self.bids.iter_all_including_invalid() {
            if !best_bid.is_valid(now_ts) {
                // Remove the order from the book unless we've done that enough
                if number_of_dropped_expired_orders < DROP_EXPIRED_ORDER_LIMIT {
                    number_of_dropped_expired_orders += 1;
                    let event = OutEvent::new(
                        Side::Bid,
                        now_ts,
                        event_queue.header.seq_num,
                        best_bid.owner,
                        best_bid.quantity,
                    );
                    event_queue.push_back(cast(event)).unwrap();
                    bid_deletes.push(best_bid.key);
                }
                continue;
            }

            let best_bid_price = best_bid.price();

            if price > best_bid_price {
                break;
            } else if post_only {
                msg!("Order could not be placed due to PostOnly");
                post_allowed = false;
                break; // return silently to not fail other instructions in tx
            } else if limit == 0 {
                msg!("Order matching limit reached");
                post_allowed = false;
                break;
            }

            let max_match_by_quote = rem_quote_quantity / best_bid_price;
            let match_quantity = rem_base_quantity
                .min(best_bid.quantity)
                .min(max_match_by_quote);
            let done = match_quantity == max_match_by_quote || match_quantity == rem_base_quantity;

            let match_quote = match_quantity * best_bid_price;
            rem_base_quantity -= match_quantity;
            rem_quote_quantity -= match_quote;
            // mango_account.perp_accounts[market_index].add_taker_trade(-match_quantity, match_quote);

            let new_best_bid_quantity = best_bid.quantity - match_quantity;
            let maker_out = new_best_bid_quantity == 0;
            if maker_out {
                bid_deletes.push(best_bid.key);
            } else {
                bid_changes.push((best_bid_h, new_best_bid_quantity));
            }

            let fill = FillEvent::new(
                Side::Ask,
                maker_out,
                now_ts,
                event_queue.header.seq_num,
                best_bid.owner,
                best_bid.key,
                best_bid.client_order_id,
                market.maker_fee,
                best_bid.timestamp,
                *mango_account_pk,
                order_id,
                client_order_id,
                market.taker_fee,
                best_bid_price,
                match_quantity,
            );

            event_queue.push_back(cast(fill)).unwrap();
            limit -= 1;

            if done {
                break;
            }
        }
        let total_quote_taken = max_quote_quantity - rem_quote_quantity;

        // Apply changes to matched bids (handles invalidate on delete!)
        for (handle, new_quantity) in bid_changes {
            self.bids
                .get_mut(handle)
                .unwrap()
                .as_leaf_mut()
                .unwrap()
                .quantity = new_quantity;
        }
        for key in bid_deletes {
            let _removed_leaf = self.bids.remove_by_key(key).unwrap();
        }

        // If there are still quantity unmatched, place on the book
        let book_base_quantity = rem_base_quantity.min(rem_quote_quantity / price);
        if book_base_quantity > 0 && post_allowed {
            // Drop an expired order if possible
            if let Some(expired_ask) = self.asks.remove_one_expired(now_ts) {
                let event = OutEvent::new(
                    Side::Ask,
                    now_ts,
                    event_queue.header.seq_num,
                    expired_ask.owner,
                    expired_ask.quantity,
                );
                event_queue.push_back(cast(event)).unwrap();
            }

            if self.asks.is_full() {
                // If this asks is lower than highest ask, boot that ask and insert this one
                let max_ask = self.asks.remove_max().unwrap();
                require!(price < max_ask.price(), MangoError::SomeError);
                let event = OutEvent::new(
                    Side::Ask,
                    now_ts,
                    event_queue.header.seq_num,
                    max_ask.owner,
                    max_ask.quantity,
                );
                event_queue.push_back(cast(event)).unwrap();
            }

            let new_ask = LeafNode::new(
                order_id,
                *mango_account_pk,
                book_base_quantity,
                client_order_id,
                now_ts,
                order_type,
                time_in_force,
            );
            let _result = self.asks.insert_leaf(&new_ask)?;

            // TODO OPT remove if PlacePerpOrder needs more compute
            msg!(
                "ask on book order_id={} quantity={} price={}",
                order_id,
                book_base_quantity,
                price
            );

            // mango_account.add_order(market_index, Side::Ask, &new_ask)?;
        }

        // if there were matched taker quote apply ref fees
        // we know ref_fee_rate is not None if total_quote_taken > 0
        if total_quote_taken > 0 {
            apply_fees(market, mango_account, total_quote_taken)?;
        }

        Ok(())
    }
}

/// Apply taker fees to the taker account and update the markets' fees_accrued for
/// both the maker and taker fees.
fn apply_fees(
    market: &mut PerpMarket,
    mango_account: &mut MangoAccount,
    total_quote_taken: i64,
) -> Result<()> {
    let taker_quote_native = I80F48::from_num(
        market
            .quote_lot_size
            .checked_mul(total_quote_taken)
            .unwrap(),
    );

    // Track maker fees immediately: they can be negative and applying them later
    // risks that fees_accrued is settled to 0 before they apply. It going negative
    // breaks assumptions.
    // The maker fees apply to the maker's account only when the fill event is consumed.
    let maker_fees = taker_quote_native * market.maker_fee;

    let taker_fees = taker_quote_native * market.taker_fee;
    let perp_account = mango_account
        .perp_account_map
        .get_mut_or_create(market.perp_market_index)?
        .0;
    perp_account.quote_position -= taker_fees;
    market.fees_accrued += taker_fees + maker_fees;

    Ok(())
}
