use std::cell::RefMut;

use crate::{
    error::MangoError,
    state::{
        orderbook::{bookside::BookSide, nodes::LeafNode},
        EventQueue, PerpMarket,
    },
};
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;

use super::{
    nodes::NodeHandle,
    order_type::{OrderType, Side},
};
use crate::util::checked_math as cm;

pub const CENTIBPS_PER_UNIT: I80F48 = I80F48!(1_000_000);
// todo move to a constants module or something
pub const MAX_PERP_OPEN_ORDERS: usize = 64;

/// Drop at most this many expired orders from a BookSide when trying to match orders.
/// This exists as a guard against excessive compute use.
const DROP_EXPIRED_ORDER_LIMIT: usize = 5;

pub struct Book<'a> {
    pub bids: RefMut<'a, BookSide>, // todo: why refmut?
    pub asks: RefMut<'a, BookSide>,
}

impl<'a> Book<'a> {
    pub fn load_checked(
        bids_ai: &'a AccountInfo,
        asks_ai: &'a AccountInfo,
        perp_market: &PerpMarket,
    ) -> std::result::Result<Self, Error> {
        require!(bids_ai.key == &perp_market.bids, MangoError::SomeError);
        require!(asks_ai.key == &perp_market.asks, MangoError::SomeError);
        Ok(Self {
            bids: BookSide::load_mut_checked(bids_ai, perp_market)?,
            asks: BookSide::load_mut_checked(asks_ai, perp_market)?,
        })
    }

    /// returns best valid bid
    pub fn get_best_bid_price(&self, now_ts: u64) -> Option<i64> {
        Some(self.bids.iter_valid(now_ts).next()?.1.price())
    }

    /// returns best valid ask
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

    // todo: can new_bid and new_ask be elegantly folded into one method?
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub fn new_bid(
        &mut self,
        // program_id: &Pubkey,
        // mango_group: &MangoGroup,
        // mango_group_pk: &Pubkey,
        // mango_cache: &MangoCache,
        _event_queue: &mut EventQueue,
        market: &mut PerpMarket,
        // oracle_price: I80F48,
        // mango_account: &mut MangoAccount,
        mango_account_pk: &Pubkey,
        // market_index: usize,
        price: i64,
        max_base_quantity: i64, // guaranteed to be greater than zero due to initial check
        max_quote_quantity: i64, // guaranteed to be greater than zero due to initial check
        order_type: OrderType,
        time_in_force: u8,
        client_order_id: u64,
        now_ts: u64,
        // referrer_mango_account_ai: Option<&AccountInfo>,
        mut limit: u8, // max number of FillEvents allowed; guaranteed to be greater than 0
    ) -> std::result::Result<(), Error> {
        // TODO proper error handling
        // TODO handle the case where we run out of compute (right now just fails)
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
        // let info = &mango_group.perp_markets[market_index];
        // if post_allowed {
        //     // price limit check computed lazily to save CU on average
        //     let native_price = market.lot_to_native_price(price);
        //     if native_price.checked_div(oracle_price).unwrap() > info.maint_liab_weight {
        //         msg!("Posting on book disallowed due to price limits");
        //         post_allowed = false;
        //     }
        // }

        // referral fee related variables
        // let mut ref_fee_rate = None;
        // let mut referrer_mango_account_opt = None;

        // generate new order id
        let order_id = market.gen_order_id(Side::Bid, price);

        // Iterate through book and match against this new bid
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
                    // let event = OutEvent::new(
                    //     Side::Ask,
                    //     best_ask.owner_slot,
                    //     now_ts,
                    //     event_queue.header.seq_num,
                    //     best_ask.owner,
                    //     best_ask.quantity,
                    // );
                    // event_queue.push_back(cast(event)).unwrap();
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

            // todo
            // if ref_fee_rate is none, determine it
            // if ref_valid, then pay into referrer, else pay to perp market
            // if ref_fee_rate.is_none() {
            //     let (a, b) = determine_ref_vars(
            //         program_id,
            //         mango_group,
            //         mango_group_pk,
            //         mango_cache,
            //         mango_account,
            //         referrer_mango_account_ai,
            //         now_ts,
            //     )?;
            //     ref_fee_rate = Some(a);
            //     referrer_mango_account_opt = b;
            // }

            // let fill = FillEvent::new(
            //     Side::Bid,
            //     best_ask.owner_slot,
            //     maker_out,
            //     now_ts,
            //     event_queue.header.seq_num,
            //     best_ask.owner,
            //     best_ask.key,
            //     best_ask.client_order_id,
            //     info.maker_fee,
            //     best_ask.best_initial,
            //     best_ask.timestamp,
            //     *mango_account_pk,
            //     order_id,
            //     client_order_id,
            //     info.taker_fee + ref_fee_rate.unwrap(),
            //     best_ask_price,
            //     match_quantity,
            //     best_ask.version,
            // );
            // event_queue.push_back(cast(fill)).unwrap();
            limit -= 1;

            if done {
                break;
            }
        }
        // let total_quote_taken = cm!(max_quote_quantity - rem_quote_quantity);

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
            // if let Some(expired_bid) = self.bids.remove_one_expired(now_ts) {
            //     let event = OutEvent::new(
            //         Side::Bid,
            //         expired_bid.owner_slot,
            //         now_ts,
            //         event_queue.header.seq_num,
            //         expired_bid.owner,
            //         expired_bid.quantity,
            //     );
            //     event_queue.push_back(cast(event)).unwrap();
            // }

            if self.bids.is_full() {
                // If this bid is higher than lowest bid, boot that bid and insert this one
                let min_bid = self.bids.remove_min().unwrap();
                require!(price > min_bid.price(), MangoError::SomeError); // MangoErrorCode::OutOfSpace
                                                                          // let event = OutEvent::new(
                                                                          //     Side::Bid,
                                                                          //     min_bid.owner_slot,
                                                                          //     now_ts,
                                                                          //     event_queue.header.seq_num,
                                                                          //     min_bid.owner,
                                                                          //     min_bid.quantity,
                                                                          // );
                                                                          // event_queue.push_back(cast(event)).unwrap();
            }

            // iterate through book on the bid side
            // let best_initial = if market.meta_data.version == 0 {
            //     match self.get_best_bid_price(now_ts) {
            //         None => price,
            //         Some(p) => p,
            //     }
            // } else {
            //     let max_depth: i64 = market.liquidity_mining_info.max_depth_bps.to_num();
            //     self.get_bids_size_above(price, max_depth, now_ts)
            // };

            // let owner_slot = mango_account
            //     .next_order_slot()
            //     .ok_or(MangoError::SomeError)?; // TooManyOpenOrders
            let new_bid = LeafNode::new(
                1, // todo market.meta_data.version,
                0, // todo owner_slot as u8,
                order_id,
                *mango_account_pk,
                book_base_quantity,
                client_order_id,
                now_ts,
                0, // todo best_initial,
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
        // if total_quote_taken > 0 {
        //     apply_fees(
        //         market,
        //         info,
        //         mango_account,
        //         mango_account_pk,
        //         market_index,
        //         referrer_mango_account_opt,
        //         referrer_mango_account_ai,
        //         total_quote_taken,
        //         ref_fee_rate.unwrap(),
        //         // &mango_cache.perp_market_cache[market_index],
        //     );
        // }

        Ok(())
    }
}
