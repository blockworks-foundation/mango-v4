use std::cell::RefMut;

use crate::accounts_zerocopy::*;
use crate::state::MangoAccountRefMut;
use crate::{
    error::*,
    state::{
        orderbook::{bookside::BookSide, nodes::LeafNode},
        EventQueue, PerpMarket, FREE_ORDER_SLOT,
    },
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

/// The implicit limit price to use for market orders
fn market_order_limit_for_side(side: Side) -> i64 {
    match side {
        Side::Bid => i64::MAX,
        Side::Ask => 1,
    }
}

/// The limit to use for PostOnlySlide orders: the tinyest bit better than
/// the best opposing order
fn post_only_slide_limit(side: Side, best_other_side: i64, limit: i64) -> i64 {
    match side {
        Side::Bid => limit.min(cm!(best_other_side - 1)),
        Side::Ask => limit.max(cm!(best_other_side + 1)),
    }
}

pub struct Book<'a> {
    pub bids: RefMut<'a, BookSide>, // todo: why refmut?
    pub asks: RefMut<'a, BookSide>,
}

impl<'a> Book<'a> {
    pub fn new(bids: RefMut<'a, BookSide>, asks: RefMut<'a, BookSide>) -> Self {
        Self { bids, asks }
    }

    pub fn load_mut(
        bids_ai: &'a AccountInfo,
        asks_ai: &'a AccountInfo,
        perp_market: &PerpMarket,
    ) -> std::result::Result<Self, Error> {
        require!(bids_ai.key == &perp_market.bids, MangoError::SomeError);
        require!(asks_ai.key == &perp_market.asks, MangoError::SomeError);
        Ok(Self::new(
            bids_ai.load_mut::<BookSide>()?,
            asks_ai.load_mut::<BookSide>()?,
        ))
    }

    pub fn bookside(&mut self, side: Side) -> &mut BookSide {
        match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        }
    }

    /// Returns best valid bid
    pub fn best_bid_price(&self, now_ts: u64) -> Option<i64> {
        Some(self.bids.iter_valid(now_ts).next()?.1.price())
    }

    /// Returns best valid ask
    pub fn best_ask_price(&self, now_ts: u64) -> Option<i64> {
        Some(self.asks.iter_valid(now_ts).next()?.1.price())
    }

    pub fn best_price(&self, now_ts: u64, side: Side) -> Option<i64> {
        match side {
            Side::Bid => self.best_bid_price(now_ts),
            Side::Ask => self.best_ask_price(now_ts),
        }
    }

    /// Get the quantity of valid bids above and including the price
    pub fn bids_size_above(&self, price: i64, max_depth: i64, now_ts: u64) -> i64 {
        let mut sum: i64 = 0;
        for (_, bid) in self.bids.iter_valid(now_ts) {
            if price > bid.price() || sum >= max_depth {
                break;
            }
            sum = sum.checked_add(bid.quantity).unwrap();
        }
        sum.min(max_depth)
    }

    /// Walk up the book `quantity` units and return the price at that level. If `quantity` units
    /// not on book, return None
    pub fn impact_price(&self, side: Side, quantity: i64, now_ts: u64) -> Option<i64> {
        let mut sum: i64 = 0;
        let book_side = match side {
            Side::Bid => self.bids.iter_valid(now_ts),
            Side::Ask => self.asks.iter_valid(now_ts),
        };
        for (_, order) in book_side {
            sum = sum.checked_add(order.quantity).unwrap();
            if sum >= quantity {
                return Some(order.price());
            }
        }
        None
    }

    /// Get the quantity of valid asks below and including the price
    pub fn asks_size_below(&self, price: i64, max_depth: i64, now_ts: u64) -> i64 {
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
    pub fn bids_size_above_order(&self, order_id: i128, max_depth: i64, now_ts: u64) -> i64 {
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
    pub fn asks_size_below_order(&self, order_id: i128, max_depth: i64, now_ts: u64) -> i64 {
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
        mango_account: &mut MangoAccountRefMut,
        mango_account_pk: &Pubkey,
        price_lots: i64,
        max_base_lots: i64,
        max_quote_lots: i64,
        order_type: OrderType,
        time_in_force: u8,
        client_order_id: u64,
        now_ts: u64,
        mut limit: u8,
    ) -> std::result::Result<(), Error> {
        let other_side = side.invert_side();
        let market = perp_market;
        let (post_only, mut post_allowed, price_lots) = match order_type {
            OrderType::Limit => (false, true, price_lots),
            OrderType::ImmediateOrCancel => (false, false, price_lots),
            OrderType::PostOnly => (true, true, price_lots),
            OrderType::Market => (false, false, market_order_limit_for_side(side)),
            OrderType::PostOnlySlide => {
                let price = if let Some(best_other_price) = self.best_price(now_ts, other_side) {
                    post_only_slide_limit(side, best_other_price, price_lots)
                } else {
                    price_lots
                };
                (true, true, price)
            }
        };

        if post_allowed {
            // price limit check computed lazily to save CU on average
            let native_price = market.lot_to_native_price(price_lots);
            if !market.inside_price_limit(side, native_price, oracle_price) {
                msg!("Posting on book disallowed due to price limits");
                post_allowed = false;
            }
        }

        // generate new order id
        let order_id = market.gen_order_id(side, price_lots);

        // Iterate through book and match against this new order.
        //
        // Any changes to matching orders on the other side of the book are collected in
        // matched_changes/matched_deletes and then applied after this loop.
        let mut remaining_base_lots = max_base_lots;
        let mut remaining_quote_lots = max_quote_lots;
        let mut matched_order_changes: Vec<(NodeHandle, i64)> = vec![];
        let mut matched_order_deletes: Vec<i128> = vec![];
        let mut number_of_dropped_expired_orders = 0;
        let opposing_bookside = self.bookside(other_side);
        for (best_opposing_h, best_opposing) in opposing_bookside.iter_all_including_invalid() {
            if !best_opposing.is_valid(now_ts) {
                // Remove the order from the book unless we've done that enough
                if number_of_dropped_expired_orders < DROP_EXPIRED_ORDER_LIMIT {
                    number_of_dropped_expired_orders += 1;
                    let event = OutEvent::new(
                        other_side,
                        best_opposing.owner_slot,
                        now_ts,
                        event_queue.header.seq_num,
                        best_opposing.owner,
                        best_opposing.quantity,
                    );
                    event_queue.push_back(cast(event)).unwrap();
                    matched_order_deletes.push(best_opposing.key);
                }
                continue;
            }

            let best_opposing_price = best_opposing.price();

            if !side.is_price_within_limit(best_opposing_price, price_lots) {
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

            let max_match_by_quote = remaining_quote_lots / best_opposing_price;
            let match_base_lots = remaining_base_lots
                .min(best_opposing.quantity)
                .min(max_match_by_quote);
            let done =
                match_base_lots == max_match_by_quote || match_base_lots == remaining_base_lots;

            let match_quote_lots = cm!(match_base_lots * best_opposing_price);
            cm!(remaining_base_lots -= match_base_lots);
            cm!(remaining_quote_lots -= match_quote_lots);

            let new_best_opposing_quantity = cm!(best_opposing.quantity - match_base_lots);
            let maker_out = new_best_opposing_quantity == 0;
            if maker_out {
                matched_order_deletes.push(best_opposing.key);
            } else {
                matched_order_changes.push((best_opposing_h, new_best_opposing_quantity));
            }

            // Record the taker trade in the account already, even though it will only be
            // realized when the fill event gets executed
            let perp_account = mango_account
                .ensure_perp_position(market.perp_market_index)?
                .0;
            perp_account.add_taker_trade(side, match_base_lots, match_quote_lots);

            let fill = FillEvent::new(
                side,
                maker_out,
                best_opposing.owner_slot,
                now_ts,
                event_queue.header.seq_num,
                best_opposing.owner,
                best_opposing.key,
                best_opposing.client_order_id,
                market.maker_fee,
                best_opposing.timestamp,
                *mango_account_pk,
                order_id,
                client_order_id,
                market.taker_fee,
                best_opposing_price,
                match_base_lots,
            );
            event_queue.push_back(cast(fill)).unwrap();
            limit -= 1;

            if done {
                break;
            }
        }
        let total_quote_lots_taken = cm!(max_quote_lots - remaining_quote_lots);

        // Apply changes to matched asks (handles invalidate on delete!)
        for (handle, new_quantity) in matched_order_changes {
            opposing_bookside
                .node_mut(handle)
                .unwrap()
                .as_leaf_mut()
                .unwrap()
                .quantity = new_quantity;
        }
        for key in matched_order_deletes {
            let _removed_leaf = opposing_bookside.remove_by_key(key).unwrap();
        }

        // If there are still quantity unmatched, place on the book
        let book_base_quantity = remaining_base_lots.min(remaining_quote_lots / price_lots);
        msg!("{:?}", post_allowed);
        if post_allowed && book_base_quantity > 0 {
            // Drop an expired order if possible
            let bookside = self.bookside(side);
            if let Some(expired_order) = bookside.remove_one_expired(now_ts) {
                let event = OutEvent::new(
                    side,
                    expired_order.owner_slot,
                    now_ts,
                    event_queue.header.seq_num,
                    expired_order.owner,
                    expired_order.quantity,
                );
                event_queue.push_back(cast(event)).unwrap();
            }

            if bookside.is_full() {
                // If this bid is higher than lowest bid, boot that bid and insert this one
                let worst_order = bookside.remove_worst().unwrap();
                // MangoErrorCode::OutOfSpace
                require!(
                    side.is_price_better(price_lots, worst_order.price()),
                    MangoError::SomeError
                );
                let event = OutEvent::new(
                    side,
                    worst_order.owner_slot,
                    now_ts,
                    event_queue.header.seq_num,
                    worst_order.owner,
                    worst_order.quantity,
                );
                event_queue.push_back(cast(event)).unwrap();
            }

            let owner_slot = mango_account.perp_next_order_slot()?;
            let new_order = LeafNode::new(
                owner_slot as u8,
                order_id,
                *mango_account_pk,
                book_base_quantity,
                client_order_id,
                now_ts,
                order_type,
                time_in_force,
            );
            let _result = bookside.insert_leaf(&new_order)?;

            // TODO OPT remove if PlacePerpOrder needs more compute
            msg!(
                "{} on book order_id={} quantity={} price={}",
                match side {
                    Side::Bid => "bid",
                    Side::Ask => "ask",
                },
                order_id,
                book_base_quantity,
                price_lots
            );

            mango_account.add_perp_order(market.perp_market_index, side, &new_order)?;
        }

        // if there were matched taker quote apply ref fees
        // we know ref_fee_rate is not None if total_quote_taken > 0
        if total_quote_lots_taken > 0 {
            apply_fees(market, mango_account, total_quote_lots_taken)?;
        }

        Ok(())
    }

    /// Cancels up to `limit` orders that are listed on the mango account for the given perp market.
    /// Optionally filters by `side_to_cancel_option`.
    /// The orders are removed from the book and from the mango account open order list.
    pub fn cancel_all_orders(
        &mut self,
        mango_account: &mut MangoAccountRefMut,
        perp_market: &mut PerpMarket,
        mut limit: u8,
        side_to_cancel_option: Option<Side>,
    ) -> Result<()> {
        for i in 0..mango_account.header.perp_oo_count() {
            let oo = mango_account.perp_order_by_raw_index(i);
            if oo.order_market == FREE_ORDER_SLOT
                || oo.order_market != perp_market.perp_market_index
            {
                continue;
            }

            let order_side = oo.order_side;
            if let Some(side_to_cancel) = side_to_cancel_option {
                if side_to_cancel != order_side {
                    continue;
                }
            }

            let order_id = oo.order_id;
            self.cancel_order(mango_account, order_id, order_side, None)?;

            limit -= 1;
            if limit == 0 {
                break;
            }
        }

        Ok(())
    }

    /// Cancels an order on a side, removing it from the book and the mango account orders list
    pub fn cancel_order(
        &mut self,
        mango_account: &mut MangoAccountRefMut,
        order_id: i128,
        side: Side,
        expected_owner: Option<Pubkey>,
    ) -> Result<LeafNode> {
        let leaf_node =
            match side {
                Side::Bid => self.bids.remove_by_key(order_id).ok_or_else(|| {
                    error_msg!("invalid perp order id {order_id} for side {side:?}")
                }),
                Side::Ask => self.asks.remove_by_key(order_id).ok_or_else(|| {
                    error_msg!("invalid perp order id {order_id} for side {side:?}")
                }),
            }?;
        if let Some(owner) = expected_owner {
            require_keys_eq!(leaf_node.owner, owner);
        }
        mango_account.remove_perp_order(leaf_node.owner_slot as usize, leaf_node.quantity)?;
        Ok(leaf_node)
    }
}

/// Apply taker fees to the taker account and update the markets' fees_accrued for
/// both the maker and taker fees.
fn apply_fees(
    market: &mut PerpMarket,
    mango_account: &mut MangoAccountRefMut,
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
        .ensure_perp_position(market.perp_market_index)?
        .0;
    perp_account.quote_position_native -= taker_fees;
    market.fees_accrued += taker_fees + maker_fees;

    Ok(())
}
