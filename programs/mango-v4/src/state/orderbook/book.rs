use std::cell::RefMut;

use crate::accounts_zerocopy::*;
use crate::state::MangoAccountRefMut;
use crate::{
    error::*,
    state::{
        orderbook::{bookside::*, nodes::*},
        EventQueue, PerpMarket, FREE_ORDER_SLOT,
    },
};
use anchor_lang::prelude::*;
use bytemuck::cast;
use fixed::types::I80F48;

use super::{
    nodes::NodeHandle,
    order_type::{OrderType, Side, SideAndComponent},
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

pub enum Order2 {
    Market,
    Direct {
        order_type: OrderType,
        price_lots: i64,
    },
    OraclePegged {
        order_type: OrderType,
        price_offset_lots: i64,
    },
}

/// TODO: what if oracle is stale for a while
pub struct Book2<'a> {
    pub bids: BookSide2<'a>,
    pub asks: BookSide2<'a>,
}

impl<'a> Book2<'a> {
    pub fn load_mut<'b: 'a>(
        bids_direct_ai: &'a AccountLoader<'b, BookSide>,
        bids_oracle_pegged_ai: &'a AccountLoader<'b, BookSide>,
        asks_direct_ai: &'a AccountLoader<'b, BookSide>,
        asks_oracle_pegged_ai: &'a AccountLoader<'b, BookSide>,
    ) -> std::result::Result<Self, Error> {
        Ok(Self {
            bids: BookSide2 {
                direct: bids_direct_ai.load_mut()?,
                oracle_pegged: bids_oracle_pegged_ai.load_mut()?,
            },
            asks: BookSide2 {
                direct: asks_direct_ai.load_mut()?,
                oracle_pegged: asks_oracle_pegged_ai.load_mut()?,
            },
        })
    }

    pub fn bookside_mut(&mut self, side: Side) -> &mut BookSide2<'a> {
        match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        }
    }

    pub fn bookside(&self, side: Side) -> &BookSide2<'a> {
        match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        }
    }

    pub fn best_price(&self, now_ts: u64, oracle_price_lots: i64, side: Side) -> Option<i64> {
        Some(
            self.bookside(side)
                .iter_valid(now_ts, oracle_price_lots)
                .next()?
                .price_lots,
        )
    }

    /// Walk up the book `quantity` units and return the price at that level. If `quantity` units
    /// not on book, return None
    pub fn impact_price(
        &self,
        side: Side,
        quantity: i64,
        now_ts: u64,
        oracle_price_lots: i64,
    ) -> Option<i64> {
        let mut sum: i64 = 0;
        let iter = self.bookside(side).iter_valid(now_ts, oracle_price_lots);
        for order in iter {
            cm!(sum += order.node.quantity);
            if sum >= quantity {
                return Some(order.price_lots);
            }
        }
        None
    }

    fn eval_order_type_helper(
        &self,
        side: Side,
        order_type: OrderType,
        price_lots: i64,
        oracle_price_lots: i64,
        now_ts: u64,
    ) -> (bool, bool, i64) {
        match order_type {
            OrderType::Limit => (false, true, price_lots),
            OrderType::ImmediateOrCancel => (false, false, price_lots),
            OrderType::PostOnly => (true, true, price_lots),
            OrderType::Market => (false, false, market_order_limit_for_side(side)),
            OrderType::PostOnlySlide => {
                let price = if let Some(best_other_price) =
                    self.best_price(now_ts, oracle_price_lots, side.invert_side())
                {
                    post_only_slide_limit(side, best_other_price, price_lots)
                } else {
                    price_lots
                };
                (true, true, price)
            }
        }
    }

    fn eval_order_params(
        &self,
        side: Side,
        order: &Order2,
        oracle_price_lots: i64,
        now_ts: u64,
    ) -> (bool, Option<BookSide2Component>, i64, i64) {
        match order {
            Order2::Market => {
                let price_lots = market_order_limit_for_side(side);
                (false, None, price_lots, price_lots)
            }
            Order2::Direct {
                order_type,
                price_lots,
            } => {
                let (post_only, post_allowed, price_lots) = self.eval_order_type_helper(
                    side,
                    *order_type,
                    *price_lots,
                    oracle_price_lots,
                    now_ts,
                );
                (
                    post_only,
                    post_allowed.then(|| BookSide2Component::Direct),
                    price_lots,
                    price_lots,
                )
            }
            Order2::OraclePegged {
                order_type,
                price_offset_lots,
            } => {
                let price_offset_lots = *price_offset_lots;
                let price_lots = cm!(oracle_price_lots + price_offset_lots);
                let (post_only, post_allowed, price_lots) = self.eval_order_type_helper(
                    side,
                    *order_type,
                    price_lots,
                    oracle_price_lots,
                    now_ts,
                );
                let price_offset_lots = cm!(price_lots - oracle_price_lots);
                (
                    post_only,
                    post_allowed.then(|| BookSide2Component::OraclePegged),
                    price_lots,
                    price_offset_lots,
                )
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_order(
        &mut self,
        side_and_component: SideAndComponent,
        order: Order2,
        perp_market: &mut PerpMarket,
        event_queue: &mut EventQueue,
        oracle_price: I80F48,
        mango_account: &mut MangoAccountRefMut,
        mango_account_pk: &Pubkey,
        max_base_lots: i64,
        max_quote_lots: i64,
        time_in_force: u8,
        client_order_id: u64,
        now_ts: u64,
        mut limit: u8,
    ) -> std::result::Result<(), Error> {
        let side = side_and_component.side();
        let other_side = side.invert_side();
        let market = perp_market;
        let oracle_price_lots = market.native_price_to_lot(oracle_price);
        let (post_only, mut post_allowed, price_lots, order_data) =
            self.eval_order_params(side, &order, oracle_price_lots, now_ts);

        if post_allowed.is_some() {
            // price limit check computed lazily to save CU on average
            let native_price = market.lot_to_native_price(price_lots);
            if !market.inside_price_limit(side, native_price, oracle_price) {
                msg!("Posting on book disallowed due to price limits");
                post_allowed = None;
            }
        }

        // generate new order id
        let order_id = market.gen_order_id(side, order_data);

        // Iterate through book and match against this new order.
        //
        // Any changes to matching orders on the other side of the book are collected in
        // matched_changes/matched_deletes and then applied after this loop.
        let mut remaining_base_lots = max_base_lots;
        let mut remaining_quote_lots = max_quote_lots;
        let mut matched_order_changes: Vec<(BookSide2NodeHandle, i64)> = vec![];
        let mut matched_order_deletes: Vec<(BookSide2Component, i128)> = vec![];
        let mut number_of_dropped_expired_orders = 0;
        let opposing_bookside = self.bookside_mut(other_side);
        for best_opposing in opposing_bookside.iter_all_including_invalid(oracle_price_lots) {
            if !best_opposing.node.is_valid(now_ts) {
                // Remove the order from the book unless we've done that enough
                if number_of_dropped_expired_orders < DROP_EXPIRED_ORDER_LIMIT {
                    number_of_dropped_expired_orders += 1;
                    let event = OutEvent::new(
                        other_side,
                        best_opposing.node.owner_slot,
                        now_ts,
                        event_queue.header.seq_num,
                        best_opposing.node.owner,
                        best_opposing.node.quantity,
                    );
                    event_queue.push_back(cast(event)).unwrap();
                    matched_order_deletes
                        .push((best_opposing.handle.component, best_opposing.node.key));
                }
                continue;
            }

            let best_opposing_price = best_opposing.price_lots;

            if !side.is_price_within_limit(best_opposing_price, price_lots) {
                break;
            } else if post_only {
                msg!("Order could not be placed due to PostOnly");
                post_allowed = None;
                break; // return silently to not fail other instructions in tx
            } else if limit == 0 {
                msg!("Order matching limit reached");
                post_allowed = None;
                break;
            }

            let max_match_by_quote = remaining_quote_lots / best_opposing_price;
            let match_base_lots = remaining_base_lots
                .min(best_opposing.node.quantity)
                .min(max_match_by_quote);
            let done =
                match_base_lots == max_match_by_quote || match_base_lots == remaining_base_lots;

            let match_quote_lots = cm!(match_base_lots * best_opposing_price);
            cm!(remaining_base_lots -= match_base_lots);
            cm!(remaining_quote_lots -= match_quote_lots);

            let new_best_opposing_quantity = cm!(best_opposing.node.quantity - match_base_lots);
            let maker_out = new_best_opposing_quantity == 0;
            if maker_out {
                matched_order_deletes
                    .push((best_opposing.handle.component, best_opposing.node.key));
            } else {
                matched_order_changes.push((best_opposing.handle, new_best_opposing_quantity));
            }

            // Record the taker trade in the account already, even though it will only be
            // realized when the fill event gets executed
            let perp_account = mango_account.perp_position_mut(market.perp_market_index)?;
            perp_account.add_taker_trade(side, match_base_lots, match_quote_lots);

            let fill = FillEvent::new(
                side,
                maker_out,
                best_opposing.node.owner_slot,
                now_ts,
                event_queue.header.seq_num,
                best_opposing.node.owner,
                best_opposing.node.key,
                best_opposing.node.client_order_id,
                market.maker_fee,
                best_opposing.node.timestamp,
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
        for (component, key) in matched_order_deletes {
            let _removed_leaf = opposing_bookside.remove_by_key(component, key).unwrap();
        }

        // If there are still quantity unmatched, place on the book
        let book_base_quantity = remaining_base_lots.min(remaining_quote_lots / price_lots);
        if book_base_quantity <= 0 {
            post_allowed = None;
        }
        if let Some(book_component) = post_allowed {
            let bookside = self.bookside_mut(side).component_mut(book_component);

            // Drop an expired order if possible
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
                    side.is_order_better(order_data, worst_order.data()),
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
                OrderType::Limit, // TODO: Support order types? needed?
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

            mango_account.add_perp_order(
                market.perp_market_index,
                side_and_component,
                &new_order,
            )?;
        }

        // if there were matched taker quote apply ref fees
        // we know ref_fee_rate is not None if total_quote_taken > 0
        if total_quote_lots_taken > 0 {
            apply_fees(market, mango_account, total_quote_lots_taken)?;
        }

        // IOC orders have a fee penalty applied regardless of match
        match order {
            Order2::Direct {
                order_type: OrderType::ImmediateOrCancel,
                ..
            }
            | Order2::OraclePegged {
                order_type: OrderType::ImmediateOrCancel,
                ..
            } => {
                apply_penalty(market, mango_account)?;
            }
            _ => {}
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
            if oo.market == FREE_ORDER_SLOT || oo.market != perp_market.perp_market_index {
                continue;
            }

            let order_side_and_component = oo.side_and_component;
            if let Some(side_to_cancel) = side_to_cancel_option {
                if side_to_cancel != order_side_and_component.side() {
                    continue;
                }
            }

            let order_id = oo.id;
            drop(oo);

            self.cancel_order(mango_account, order_id, order_side_and_component, None)?;

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
        side_and_component: SideAndComponent,
        expected_owner: Option<Pubkey>,
    ) -> Result<LeafNode> {
        let side = side_and_component.side();
        let book_component = side_and_component.component();
        let leaf_node = self.bookside_mut(side).component_mut(book_component).
        remove_by_key(order_id).ok_or_else(|| {
                    error_msg!("invalid perp order id {order_id} for side {side:?} and component {book_component:?}")
                })?;
        if let Some(owner) = expected_owner {
            require_keys_eq!(leaf_node.owner, owner);
        }
        mango_account.remove_perp_order(leaf_node.owner_slot as usize, leaf_node.quantity)?;
        Ok(leaf_node)
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
        require!(
            bids_ai.key == &perp_market.bids_direct,
            MangoError::SomeError
        );
        require!(
            asks_ai.key == &perp_market.asks_direct,
            MangoError::SomeError
        );
        Ok(Self::new(
            bids_ai.load_mut::<BookSide>()?,
            asks_ai.load_mut::<BookSide>()?,
        ))
    }

    pub fn bookside_mut(&mut self, side: Side) -> &mut BookSide {
        match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        }
    }

    pub fn bookside(&self, side: Side) -> &BookSide {
        match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        }
    }

    pub fn best_price(&self, now_ts: u64, side: Side) -> Option<i64> {
        Some(self.bookside(side).iter_valid(now_ts).next()?.1.price())
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
        let opposing_bookside = self.bookside_mut(other_side);
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
            let perp_account = mango_account.perp_position_mut(market.perp_market_index)?;
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
            let bookside = self.bookside_mut(side);
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
                    side.is_order_better(price_lots, worst_order.price()),
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

            mango_account.add_perp_order(
                market.perp_market_index,
                match side {
                    Side::Bid => SideAndComponent::BidDirect,
                    Side::Ask => SideAndComponent::AskDirect,
                },
                &new_order,
            )?;
        }

        // if there were matched taker quote apply ref fees
        // we know ref_fee_rate is not None if total_quote_taken > 0
        if total_quote_lots_taken > 0 {
            apply_fees(market, mango_account, total_quote_lots_taken)?;
        }

        // IOC orders have a fee penalty applied regardless of match
        if order_type == OrderType::ImmediateOrCancel {
            apply_penalty(market, mango_account)?;
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
            if oo.market == FREE_ORDER_SLOT || oo.market != perp_market.perp_market_index {
                continue;
            }

            let order_side = oo.side_and_component.side();
            if let Some(side_to_cancel) = side_to_cancel_option {
                if side_to_cancel != order_side {
                    continue;
                }
            }

            let order_id = oo.id;
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
    let maker_fees = cm!(taker_quote_native * market.maker_fee);

    let taker_fees = cm!(taker_quote_native * market.taker_fee);

    let perp_account = mango_account.perp_position_mut(market.perp_market_index)?;
    perp_account.change_quote_position(-taker_fees);
    cm!(market.fees_accrued += taker_fees + maker_fees);

    Ok(())
}

/// Applies a fixed penalty fee to the account, and update the market's fees_accrued
fn apply_penalty(market: &mut PerpMarket, mango_account: &mut MangoAccountRefMut) -> Result<()> {
    let perp_account = mango_account.perp_position_mut(market.perp_market_index)?;
    let fee_penalty = I80F48::from_num(market.fee_penalty);

    perp_account.change_quote_position(-fee_penalty);
    cm!(market.fees_accrued += fee_penalty);

    Ok(())
}
