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
use static_assertions::const_assert_eq;

use super::*;
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

/// TODO: what if oracle is stale for a while

pub struct Order {
    pub side: Side,
    pub params: OrderParams,
}

pub enum OrderParams {
    Market,
    ImmediateOrCancel {
        price_lots: i64,
    },
    Fixed {
        price_lots: i64,
        order_type: PostOrderType,
    },
    OraclePegged {
        price_offset_lots: i64,
        order_type: PostOrderType,
        peg_limit: i64,
    },
}

impl Order {
    pub fn needs_penalty_fee(&self) -> bool {
        matches!(self.params, OrderParams::ImmediateOrCancel { .. })
    }

    pub fn is_post_only(&self) -> bool {
        let order_type = match self.params {
            OrderParams::Fixed { order_type, .. } => order_type,
            OrderParams::OraclePegged { order_type, .. } => order_type,
            _ => return false,
        };
        order_type == PostOrderType::PostOnly || order_type == PostOrderType::PostOnlySlide
    }

    pub fn post_target(&self) -> Option<BookSideOrderTree> {
        match self.params {
            OrderParams::Fixed { .. } => Some(BookSideOrderTree::Fixed),
            OrderParams::OraclePegged { .. } => Some(BookSideOrderTree::OraclePegged),
            _ => None,
        }
    }

    fn price_for_type(
        &self,
        now_ts: u64,
        oracle_price_lots: i64,
        price_lots: i64,
        order_type: PostOrderType,
        order_book: &OrderBook,
    ) -> i64 {
        if order_type == PostOrderType::PostOnlySlide {
            return if let Some(best_other_price) =
                order_book.best_price(now_ts, oracle_price_lots, self.side.invert_side())
            {
                post_only_slide_limit(self.side, best_other_price, price_lots)
            } else {
                price_lots
            };
        } else {
            price_lots
        }
    }

    pub fn price(
        &self,
        now_ts: u64,
        oracle_price_lots: i64,
        order_book: &OrderBook,
    ) -> Result<(i64, u64)> {
        let price_lots = match self.params {
            OrderParams::Market => market_order_limit_for_side(self.side),
            OrderParams::ImmediateOrCancel { price_lots } => price_lots,
            OrderParams::Fixed {
                price_lots,
                order_type,
            } => self.price_for_type(
                now_ts,
                oracle_price_lots,
                price_lots,
                order_type,
                order_book,
            ),
            OrderParams::OraclePegged {
                price_offset_lots,
                order_type,
                ..
            } => {
                let price_lots = cm!(oracle_price_lots + price_offset_lots);
                self.price_for_type(
                    now_ts,
                    oracle_price_lots,
                    price_lots,
                    order_type,
                    order_book,
                )
            }
        };
        let price_data = match self.params {
            OrderParams::OraclePegged { .. } => {
                oracle_pegged_price_data(cm!(price_lots - oracle_price_lots))
            }
            _ => fixed_price_data(price_lots)?,
        };
        Ok((price_lots, price_data))
    }
}

#[account(zero_copy)]
pub struct OrderBook {
    pub bids_fixed: OrderTree,
    pub asks_fixed: OrderTree,
    pub bids_oracle_pegged: OrderTree,
    pub asks_oracle_pegged: OrderTree,
}
const_assert_eq!(
    std::mem::size_of::<OrderBook>(),
    4 * std::mem::size_of::<OrderTree>()
);
const_assert_eq!(std::mem::size_of::<OrderBook>() % 8, 0);

impl OrderBook {
    pub fn bookside_mut(&mut self, side: Side) -> BookSideRefMut {
        match side {
            Side::Bid => BookSideRefMut {
                fixed: &mut self.bids_fixed,
                oracle_pegged: &mut self.bids_oracle_pegged,
            },
            Side::Ask => BookSideRefMut {
                fixed: &mut self.asks_fixed,
                oracle_pegged: &mut self.asks_oracle_pegged,
            },
        }
    }

    pub fn bookside(&self, side: Side) -> BookSideRef {
        match side {
            Side::Bid => BookSideRef {
                fixed: &self.bids_fixed,
                oracle_pegged: &self.bids_oracle_pegged,
            },
            Side::Ask => BookSideRef {
                fixed: &self.asks_fixed,
                oracle_pegged: &self.asks_oracle_pegged,
            },
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
        let bookside = self.bookside(side);
        let iter = bookside.iter_valid(now_ts, oracle_price_lots);
        for order in iter {
            cm!(sum += order.node.quantity);
            if sum >= quantity {
                return Some(order.price_lots);
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_order(
        &mut self,
        order: Order,
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
        require_gte!(max_base_lots, 0);
        require_gte!(max_quote_lots, 0);

        let side = order.side;
        let other_side = side.invert_side();
        let market = perp_market;
        let oracle_price_lots = market.native_price_to_lot(oracle_price);
        let post_only = order.is_post_only();
        let mut post_target = order.post_target();
        let (price_lots, price_data) = order.price(now_ts, oracle_price_lots, self)?;

        if post_target.is_some() {
            // price limit check computed lazily to save CU on average
            let native_price = market.lot_to_native_price(price_lots);
            if !market.inside_price_limit(side, native_price, oracle_price) {
                msg!("Posting on book disallowed due to price limits");
                post_target = None;
            }
        }

        // generate new order id
        let order_id = market.gen_order_id(side, price_data);

        // Iterate through book and match against this new order.
        //
        // Any changes to matching orders on the other side of the book are collected in
        // matched_changes/matched_deletes and then applied after this loop.
        let mut remaining_base_lots = max_base_lots;
        let mut remaining_quote_lots = max_quote_lots;
        let mut matched_order_changes: Vec<(BookSideOrderHandle, i64)> = vec![];
        let mut matched_order_deletes: Vec<(BookSideOrderTree, u128)> = vec![];
        let mut number_of_dropped_expired_orders = 0;
        let mut opposing_bookside = self.bookside_mut(other_side);
        for best_opposing in opposing_bookside
            .non_mut()
            .iter_all_including_invalid(now_ts, oracle_price_lots)
        {
            if !best_opposing.is_valid {
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
                        .push((best_opposing.handle.order_tree, best_opposing.node.key));
                }
                continue;
            }

            let best_opposing_price = best_opposing.price_lots;

            if !side.is_price_within_limit(best_opposing_price, price_lots) {
                break;
            } else if post_only {
                msg!("Order could not be placed due to PostOnly");
                post_target = None;
                break; // return silently to not fail other instructions in tx
            } else if limit == 0 {
                msg!("Order matching limit reached");
                post_target = None;
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
                    .push((best_opposing.handle.order_tree, best_opposing.node.key));
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
            post_target = None;
        }
        if let Some(order_tree_target) = post_target {
            let mut bookside = self.bookside_mut(side);
            let order_tree = bookside.orders_mut(order_tree_target);

            // Drop an expired order if possible
            if let Some(expired_order) = order_tree.remove_one_expired(now_ts) {
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

            if order_tree.is_full() {
                // If this bid is higher than lowest bid, boot that bid and insert this one
                let worst_order = order_tree.remove_worst().unwrap();
                // MangoErrorCode::OutOfSpace
                require!(
                    side.is_price_data_better(price_data, worst_order.price_data()),
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
                PostOrderType::Limit, // TODO: Support order types? needed?
                time_in_force,
                -1,
            );
            let _result = order_tree.insert_leaf(&new_order)?;

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
                side,
                order_tree_target,
                &new_order,
            )?;
        }

        // if there were matched taker quote apply ref fees
        // we know ref_fee_rate is not None if total_quote_taken > 0
        if total_quote_lots_taken > 0 {
            apply_fees(market, mango_account, total_quote_lots_taken)?;
        }

        // IOC orders have a fee penalty applied regardless of match
        if order.needs_penalty_fee() {
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

            let order_side_and_tree = oo.side_and_tree;
            if let Some(side_to_cancel) = side_to_cancel_option {
                if side_to_cancel != order_side_and_tree.side() {
                    continue;
                }
            }

            let order_id = oo.id;

            self.cancel_order(mango_account, order_id, order_side_and_tree, None)?;

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
        order_id: u128,
        side_and_tree: SideAndOrderTree,
        expected_owner: Option<Pubkey>,
    ) -> Result<LeafNode> {
        let side = side_and_tree.side();
        let book_component = side_and_tree.order_tree();
        let leaf_node = self.bookside_mut(side).orders_mut(book_component).
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
