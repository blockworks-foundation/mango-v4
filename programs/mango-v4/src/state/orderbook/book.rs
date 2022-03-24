use std::cell::RefMut;

use crate::{
    error::MangoError,
    state::{
        orderbook::{bookside::BookSide, nodes::LeafNode},
        EventQueue, PerpMarket,
    },
};
use anchor_lang::prelude::*;
use bytemuck::cast;
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
        require!(bids_ai.key == &perp_market.bids, MangoError::SomeError); // MangoErrorCode::InvalidAccount
        require!(asks_ai.key == &perp_market.asks, MangoError::SomeError); //    MangoErrorCode::InvalidAccount
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

    // #[inline(never)]
    // pub fn new_order(
    //     &mut self,
    //     program_id: &Pubkey,
    //     // mango_group: &MangoGroup,
    //     mango_group_pk: &Pubkey,
    //     // mango_cache: &MangoCache,
    //     event_queue: &mut EventQueue,
    //     market: &mut PerpMarket,
    //     oracle_price: I80F48,
    //     mango_account: &mut MangoAccount,
    //     mango_account_pk: &Pubkey,
    //     market_index: usize,
    //     side: Side,
    //     price: i64,
    //     max_base_quantity: i64, // guaranteed to be greater than zero due to initial check
    //     max_quote_quantity: i64, // guaranteed to be greater than zero due to initial check
    //     order_type: OrderType,
    //     time_in_force: u8,
    //     client_order_id: u64,
    //     now_ts: u64,
    //     referrer_mango_account_ai: Option<&AccountInfo>,
    //     limit: u8,
    // ) -> std::result::Result<(), Error> {
    //     match side {
    //         Side::Bid => self.new_bid(
    //             program_id,
    //             // mango_group,
    //             mango_group_pk,
    //             // mango_cache,
    //             event_queue,
    //             market,
    //             oracle_price,
    //             mango_account,
    //             mango_account_pk,
    //             market_index,
    //             price,
    //             max_base_quantity,
    //             max_quote_quantity,
    //             order_type,
    //             time_in_force,
    //             client_order_id,
    //             now_ts,
    //             referrer_mango_account_ai,
    //             limit,
    //         ),
    //         Side::Ask => self.new_ask(
    //             program_id,
    //             // mango_group,
    //             mango_group_pk,
    //             // mango_cache,
    //             event_queue,
    //             market,
    //             oracle_price,
    //             mango_account,
    //             mango_account_pk,
    //             market_index,
    //             price,
    //             max_base_quantity,
    //             max_quote_quantity,
    //             order_type,
    //             time_in_force,
    //             client_order_id,
    //             now_ts,
    //             referrer_mango_account_ai,
    //             limit,
    //         ),
    //     }
    // }

    /// Iterate over the book and return
    /// return changes to (taker_base, taker_quote, bids_quantity, asks_quantity)
    // pub fn sim_new_bid(
    //     &self,
    //     market: &PerpMarket,
    //     // info: &PerpMarketInfo,
    //     oracle_price: I80F48,
    //     price: i64,
    //     max_base_quantity: i64, // guaranteed to be greater than zero due to initial check
    //     max_quote_quantity: i64, // guaranteed to be greater than zero due to initial check
    //     order_type: OrderType,
    //     now_ts: u64,
    // ) -> std::result::Result<(i64, i64, i64, i64), Error> {
    //     let (mut taker_base, mut taker_quote, mut bids_quantity, asks_quantity) = (0, 0, 0i64, 0);

    //     let (post_only, mut post_allowed, price) = match order_type {
    //         OrderType::Limit => (false, true, price),
    //         OrderType::ImmediateOrCancel => (false, false, price),
    //         OrderType::PostOnly => (true, true, price),
    //         OrderType::Market => (false, false, i64::MAX),
    //         OrderType::PostOnlySlide => {
    //             let price = if let Some(best_ask_price) = self.get_best_ask_price(now_ts) {
    //                 price.min(best_ask_price.checked_sub(1).ok_or(MangoError::SomeError)?)
    //             // math_err
    //             } else {
    //                 price
    //             };
    //             (true, true, price)
    //         }
    //     };
    //     // if post_allowed {
    //     //     // price limit check computed lazily to save CU on average
    //     //     let native_price = market.lot_to_native_price(price);
    //     //     if native_price.checked_div(oracle_price).unwrap() > info.maint_liab_weight {
    //     //         msg!("Posting on book disallowed due to price limits");
    //     //         post_allowed = false;
    //     //     }
    //     // }

    //     let mut rem_base_quantity = max_base_quantity; // base lots (aka contracts)
    //     let mut rem_quote_quantity = max_quote_quantity;

    //     for (_, best_ask) in self.asks.iter_valid(now_ts) {
    //         let best_ask_price = best_ask.price();
    //         if price < best_ask_price {
    //             break;
    //         } else if post_only {
    //             return Ok((taker_base, taker_quote, bids_quantity, asks_quantity));
    //         }

    //         let max_match_by_quote = rem_quote_quantity / best_ask_price;
    //         let match_quantity = rem_base_quantity
    //             .min(best_ask.quantity)
    //             .min(max_match_by_quote);

    //         let match_quote = match_quantity * best_ask_price;
    //         rem_base_quantity -= match_quantity;
    //         rem_quote_quantity -= match_quote;

    //         taker_base += match_quantity;
    //         taker_quote -= match_quote;
    //         if match_quantity == max_match_by_quote || rem_base_quantity == 0 {
    //             break;
    //         }
    //     }
    //     let book_base_quantity = rem_base_quantity.min(rem_quote_quantity / price);
    //     if post_allowed && book_base_quantity > 0 {
    //         bids_quantity = bids_quantity.checked_add(book_base_quantity).unwrap();
    //     }
    //     Ok((taker_base, taker_quote, bids_quantity, asks_quantity))
    // }

    // pub fn sim_new_ask(
    //     &self,
    //     market: &PerpMarket,
    //     // info: &PerpMarketInfo,
    //     oracle_price: I80F48,
    //     price: i64,
    //     max_base_quantity: i64, // guaranteed to be greater than zero due to initial check
    //     max_quote_quantity: i64, // guaranteed to be greater than zero due to initial check
    //     order_type: OrderType,
    //     now_ts: u64,
    // ) -> std::result::Result<(i64, i64, i64, i64), Error> {
    //     let (mut taker_base, mut taker_quote, bids_quantity, mut asks_quantity) = (0, 0, 0, 0i64);

    //     let (post_only, mut post_allowed, price) = match order_type {
    //         OrderType::Limit => (false, true, price),
    //         OrderType::ImmediateOrCancel => (false, false, price),
    //         OrderType::PostOnly => (true, true, price),
    //         OrderType::Market => (false, false, 1),
    //         OrderType::PostOnlySlide => {
    //             let price = if let Some(best_bid_price) = self.get_best_bid_price(now_ts) {
    //                 price.max(best_bid_price.checked_add(1).ok_or(MangoError::SomeError)?)
    //             // todo math_err
    //             } else {
    //                 price
    //             };
    //             (true, true, price)
    //         }
    //     };
    //     // if post_allowed {
    //     //     // price limit check computed lazily to save CU on average
    //     //     let native_price = market.lot_to_native_price(price);
    //     //     if native_price.checked_div(oracle_price).unwrap() < info.maint_asset_weight {
    //     //         msg!("Posting on book disallowed due to price limits");
    //     //         post_allowed = false;
    //     //     }
    //     // }

    //     let mut rem_base_quantity = max_base_quantity; // base lots (aka contracts)
    //     let mut rem_quote_quantity = max_quote_quantity;

    //     for (_, best_bid) in self.bids.iter_valid(now_ts) {
    //         let best_bid_price = best_bid.price();
    //         if price > best_bid_price {
    //             break;
    //         } else if post_only {
    //             return Ok((taker_base, taker_quote, bids_quantity, asks_quantity));
    //         }

    //         let max_match_by_quote = rem_quote_quantity / best_bid_price;
    //         let match_quantity = rem_base_quantity
    //             .min(best_bid.quantity)
    //             .min(max_match_by_quote);

    //         let match_quote = match_quantity * best_bid_price;
    //         rem_base_quantity -= match_quantity;
    //         rem_quote_quantity -= match_quote;

    //         taker_base -= match_quantity;
    //         taker_quote += match_quote;
    //         if match_quantity == max_match_by_quote || rem_base_quantity == 0 {
    //             break;
    //         }
    //     }

    //     let book_base_quantity = rem_base_quantity.min(rem_quote_quantity / price);
    //     if post_allowed && book_base_quantity > 0 {
    //         asks_quantity = asks_quantity.checked_add(book_base_quantity).unwrap();
    //     }
    //     Ok((taker_base, taker_quote, bids_quantity, asks_quantity))
    // }

    // todo: can new_bid and new_ask be elegantly folded into one method?
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub fn new_bid(
        &mut self,
        // program_id: &Pubkey,
        // mango_group: &MangoGroup,
        // mango_group_pk: &Pubkey,
        // mango_cache: &MangoCache,
        event_queue: &mut EventQueue,
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

    // #[inline(never)]
    // pub fn new_ask(
    //     &mut self,
    //     program_id: &Pubkey,
    //     // mango_group: &MangoGroup,
    //     mango_group_pk: &Pubkey,
    //     // mango_cache: &MangoCache,
    //     event_queue: &mut EventQueue,
    //     market: &mut PerpMarket,
    //     oracle_price: I80F48,
    //     mango_account: &mut MangoAccount,
    //     mango_account_pk: &Pubkey,
    //     market_index: usize,
    //     price: i64,
    //     max_base_quantity: i64, // guaranteed to be greater than zero due to initial check
    //     max_quote_quantity: i64, // guaranteed to be greater than zero due to initial check
    //     order_type: OrderType,
    //     time_in_force: u8,
    //     client_order_id: u64,
    //     now_ts: u64,
    //     referrer_mango_account_ai: Option<&AccountInfo>,
    //     mut limit: u8, // max number of FillEvents allowed; guaranteed to be greater than 0
    // ) -> Result<()> {
    //     let (post_only, mut post_allowed, price) = match order_type {
    //         OrderType::Limit => (false, true, price),
    //         OrderType::ImmediateOrCancel => (false, false, price),
    //         OrderType::PostOnly => (true, true, price),
    //         OrderType::Market => (false, false, 1),
    //         OrderType::PostOnlySlide => {
    //             let price = if let Some(best_bid_price) = self.get_best_bid_price(now_ts) {
    //                 price.max(best_bid_price.checked_add(1).ok_or(MangoError::SomeError)?)
    //             // math_err
    //             } else {
    //                 price
    //             };
    //             (true, true, price)
    //         }
    //     };

    //     // let info = &mango_group.perp_markets[market_index];
    //     // if post_allowed {
    //     //     // price limit check computed lazily to save CU on average
    //     //     let native_price = market.lot_to_native_price(price);
    //     //     if native_price.checked_div(oracle_price).unwrap() < info.maint_asset_weight {
    //     //         msg!("Posting on book disallowed due to price limits");
    //     //         post_allowed = false;
    //     //     }
    //     // }

    //     // referral fee related variables
    //     // let mut ref_fee_rate = None;
    //     // let mut referrer_mango_account_opt = None;

    //     // generate new order id
    //     let order_id = market.gen_order_id(Side::Ask, price);

    //     // Iterate through book and match against this new ask
    //     //
    //     // Any changes to matching bids are collected in bid_changes
    //     // and then applied after this loop.
    //     let mut rem_base_quantity = max_base_quantity; // base lots (aka contracts)
    //     let mut rem_quote_quantity = max_quote_quantity;
    //     let mut bid_changes: Vec<(NodeHandle, i64)> = vec![];
    //     let mut bid_deletes: Vec<i128> = vec![];
    //     let mut number_of_dropped_expired_orders = 0;
    //     for (best_bid_h, best_bid) in self.bids.iter_all_including_invalid() {
    //         if !best_bid.is_valid(now_ts) {
    //             // Remove the order from the book unless we've done that enough
    //             if number_of_dropped_expired_orders < DROP_EXPIRED_ORDER_LIMIT {
    //                 number_of_dropped_expired_orders += 1;
    //                 let event = OutEvent::new(
    //                     Side::Bid,
    //                     best_bid.owner_slot,
    //                     now_ts,
    //                     event_queue.header.seq_num,
    //                     best_bid.owner,
    //                     best_bid.quantity,
    //                 );
    //                 event_queue.push_back(cast(event)).unwrap();
    //                 bid_deletes.push(best_bid.key);
    //             }
    //             continue;
    //         }

    //         let best_bid_price = best_bid.price();

    //         if price > best_bid_price {
    //             break;
    //         } else if post_only {
    //             msg!("Order could not be placed due to PostOnly");
    //             post_allowed = false;
    //             break; // return silently to not fail other instructions in tx
    //         } else if limit == 0 {
    //             msg!("Order matching limit reached");
    //             post_allowed = false;
    //             break;
    //         }

    //         let max_match_by_quote = rem_quote_quantity / best_bid_price;
    //         let match_quantity = rem_base_quantity
    //             .min(best_bid.quantity)
    //             .min(max_match_by_quote);
    //         let done = match_quantity == max_match_by_quote || match_quantity == rem_base_quantity;

    //         let match_quote = match_quantity * best_bid_price;
    //         rem_base_quantity -= match_quantity;
    //         rem_quote_quantity -= match_quote;
    //         // mango_account.perp_accounts[market_index].add_taker_trade(-match_quantity, match_quote);

    //         let new_best_bid_quantity = best_bid.quantity - match_quantity;
    //         let maker_out = new_best_bid_quantity == 0;
    //         if maker_out {
    //             bid_deletes.push(best_bid.key);
    //         } else {
    //             bid_changes.push((best_bid_h, new_best_bid_quantity));
    //         }

    //         // todo
    //         // if ref_fee_rate is none, determine it
    //         // if ref_valid, then pay into referrer, else pay to perp market
    //         // if ref_fee_rate.is_none() {
    //         //     let (a, b) = determine_ref_vars(
    //         //         program_id,
    //         //         mango_group,
    //         //         mango_group_pk,
    //         //         mango_cache,
    //         //         mango_account,
    //         //         referrer_mango_account_ai,
    //         //         now_ts,
    //         //     )?;
    //         //     ref_fee_rate = Some(a);
    //         //     referrer_mango_account_opt = b;
    //         // }

    //         // let fill = FillEvent::new(
    //         //     Side::Ask,
    //         //     best_bid.owner_slot,
    //         //     maker_out,
    //         //     now_ts,
    //         //     event_queue.header.seq_num,
    //         //     best_bid.owner,
    //         //     best_bid.key,
    //         //     best_bid.client_order_id,
    //         //     info.maker_fee,
    //         //     best_bid.best_initial,
    //         //     best_bid.timestamp,
    //         //     *mango_account_pk,
    //         //     order_id,
    //         //     client_order_id,
    //         //     info.taker_fee + ref_fee_rate.unwrap(),
    //         //     best_bid_price,
    //         //     match_quantity,
    //         //     best_bid.version,
    //         // );

    //         // event_queue.push_back(cast(fill)).unwrap();
    //         limit -= 1;

    //         if done {
    //             break;
    //         }
    //     }
    //     let total_quote_taken = max_quote_quantity - rem_quote_quantity;

    //     // Apply changes to matched bids (handles invalidate on delete!)
    //     for (handle, new_quantity) in bid_changes {
    //         self.bids
    //             .get_mut(handle)
    //             .unwrap()
    //             .as_leaf_mut()
    //             .unwrap()
    //             .quantity = new_quantity;
    //     }
    //     for key in bid_deletes {
    //         let _removed_leaf = self.bids.remove_by_key(key).unwrap();
    //     }

    //     // If there are still quantity unmatched, place on the book
    //     let book_base_quantity = rem_base_quantity.min(rem_quote_quantity / price);
    //     if book_base_quantity > 0 && post_allowed {
    //         // Drop an expired order if possible
    //         if let Some(expired_ask) = self.asks.remove_one_expired(now_ts) {
    //             let event = OutEvent::new(
    //                 Side::Ask,
    //                 expired_ask.owner_slot,
    //                 now_ts,
    //                 event_queue.header.seq_num,
    //                 expired_ask.owner,
    //                 expired_ask.quantity,
    //             );
    //             event_queue.push_back(cast(event)).unwrap();
    //         }

    //         if self.asks.is_full() {
    //             // If this asks is lower than highest ask, boot that ask and insert this one
    //             let max_ask = self.asks.remove_max().unwrap();
    //             require!(price < max_ask.price(), MangoError::SomeError); // OutOfSpace
    //             let event = OutEvent::new(
    //                 Side::Ask,
    //                 max_ask.owner_slot,
    //                 now_ts,
    //                 event_queue.header.seq_num,
    //                 max_ask.owner,
    //                 max_ask.quantity,
    //             );
    //             event_queue.push_back(cast(event)).unwrap();
    //         }

    //         // let best_initial = if market.meta_data.version == 0 {
    //         //     match self.get_best_ask_price(now_ts) {
    //         //         None => price,
    //         //         Some(p) => p,
    //         //     }
    //         // } else {
    //         //     let max_depth: i64 = market.liquidity_mining_info.max_depth_bps.to_num();
    //         //     self.get_asks_size_below(price, max_depth, now_ts)
    //         // };

    //         // let owner_slot = mango_account
    //         //     .next_order_slot()
    //         //     .ok_or(MangoError::SomeError)?; // TooManyOpenOrders
    //         let new_ask = LeafNode::new(
    //             1, // todo market.meta_data.version,
    //             0, // todo owner_slot as u8,
    //             order_id,
    //             *mango_account_pk,
    //             book_base_quantity,
    //             client_order_id,
    //             now_ts,
    //             0, // todo best_initial,
    //             order_type,
    //             time_in_force,
    //         );
    //         let _result = self.asks.insert_leaf(&new_ask)?;

    //         // TODO OPT remove if PlacePerpOrder needs more compute
    //         msg!(
    //             "ask on book order_id={} quantity={} price={}",
    //             order_id,
    //             book_base_quantity,
    //             price
    //         );

    //         // mango_account.add_order(market_index, Side::Ask, &new_ask)?;
    //     }

    //     // if there were matched taker quote apply ref fees
    //     // we know ref_fee_rate is not None if total_quote_taken > 0
    //     // if total_quote_taken > 0 {
    //     //     apply_fees(
    //     //         market,
    //     //         info,
    //     //         mango_account,
    //     //         mango_account_pk,
    //     //         market_index,
    //     //         referrer_mango_account_opt,
    //     //         referrer_mango_account_ai,
    //     //         total_quote_taken,
    //     //         ref_fee_rate.unwrap(),
    //     //         // &mango_cache.perp_market_cache[market_index],
    //     //     );
    //     // }

    //     Ok(())
    // }

    // pub fn cancel_order(&mut self, order_id: i128, side: Side) -> Result<()> {
    //     match side {
    //         Side::Bid => self
    //             .bids
    //             .remove_by_key(order_id)
    //             .ok_or(MangoError::SomeError), // InvalidOrderId
    //         Side::Ask => self
    //             .asks
    //             .remove_by_key(order_id)
    //             .ok_or(MangoError::SomeError), // InvalidOrderId
    //     }
    // }

    // /// Used by force cancel so does not need to give liquidity incentives
    // pub fn cancel_all(
    //     &mut self,
    //     mango_account: &mut MangoAccount,
    //     market_index: usize,
    //     mut limit: u8,
    // ) -> Result<()> {
    //     let market_index = market_index as u8;
    //     for i in 0..MAX_PERP_OPEN_ORDERS {
    //         if mango_account.order_market[i] != market_index {
    //             // means slot is free or belongs to different perp market
    //             continue;
    //         }
    //         let order_id = mango_account.orders[i];
    //         match self.cancel_order(order_id, mango_account.order_side[i]) {
    //             Ok(order) => {
    //                 mango_account.remove_order(order.owner_slot as usize, order.quantity)?;
    //             }
    //             Err(_) => {
    //                 // If it's not on the book, then it has been matched and only Keeper can remove
    //             }
    //         };

    //         limit -= 1;
    //         if limit == 0 {
    //             break;
    //         }
    //     }
    //     Ok(())
    // }

    // pub fn cancel_all_side_with_size_incentives(
    //     &mut self,
    //     mango_account: &mut MangoAccount,
    //     perp_market: &mut PerpMarket,
    //     market_index: usize,
    //     side: Side,
    //     mut limit: u8,
    // ) -> std::result::Result<(Vec<i128>, Vec<i128>), MangoError> {
    //     // TODO - test different limits
    //     let now_ts = Clock::get()?.unix_timestamp as u64;
    //     let max_depth: i64 = perp_market.liquidity_mining_info.max_depth_bps.to_num();

    //     let mut all_order_ids = vec![];
    //     let mut canceled_order_ids = vec![];
    //     let mut keys = vec![];
    //     let market_index_u8 = market_index as u8;
    //     for i in 0..MAX_PERP_OPEN_ORDERS {
    //         if mango_account.order_market[i] == market_index_u8
    //             && mango_account.order_side[i] == side
    //         {
    //             all_order_ids.push(mango_account.orders[i]);
    //             keys.push(mango_account.orders[i])
    //         }
    //     }
    //     match side {
    //         Side::Bid => self.cancel_all_bids_with_size_incentives(
    //             mango_account,
    //             perp_market,
    //             market_index,
    //             max_depth,
    //             now_ts,
    //             &mut limit,
    //             keys,
    //             &mut canceled_order_ids,
    //         )?,
    //         Side::Ask => self.cancel_all_asks_with_size_incentives(
    //             mango_account,
    //             perp_market,
    //             market_index,
    //             max_depth,
    //             now_ts,
    //             &mut limit,
    //             keys,
    //             &mut canceled_order_ids,
    //         )?,
    //     };
    //     Ok((all_order_ids, canceled_order_ids))
    // }
    // pub fn cancel_all_with_size_incentives(
    //     &mut self,
    //     mango_account: &mut MangoAccount,
    //     perp_market: &mut PerpMarket,
    //     market_index: usize,
    //     mut limit: u8,
    // ) -> std::result::Result<(Vec<i128>, Vec<i128>), Error> {
    //     // TODO - test different limits
    //     let now_ts = Clock::get()?.unix_timestamp as u64;
    //     let max_depth: i64 = perp_market.liquidity_mining_info.max_depth_bps.to_num();

    //     let mut all_order_ids = vec![];
    //     let mut canceled_order_ids = vec![];

    //     let market_index_u8 = market_index as u8;
    //     let mut bids_keys = vec![];
    //     let mut asks_keys = vec![];
    //     for i in 0..MAX_PERP_OPEN_ORDERS {
    //         if mango_account.order_market[i] != market_index_u8 {
    //             continue;
    //         }
    //         all_order_ids.push(mango_account.orders[i]);
    //         match mango_account.order_side[i] {
    //             Side::Bid => bids_keys.push(mango_account.orders[i]),
    //             Side::Ask => asks_keys.push(mango_account.orders[i]),
    //         }
    //     }
    //     self.cancel_all_bids_with_size_incentives(
    //         mango_account,
    //         perp_market,
    //         market_index,
    //         max_depth,
    //         now_ts,
    //         &mut limit,
    //         bids_keys,
    //         &mut canceled_order_ids,
    //     )?;
    //     self.cancel_all_asks_with_size_incentives(
    //         mango_account,
    //         perp_market,
    //         market_index,
    //         max_depth,
    //         now_ts,
    //         &mut limit,
    //         asks_keys,
    //         &mut canceled_order_ids,
    //     )?;
    //     Ok((all_order_ids, canceled_order_ids))
    // }

    // /// Internal
    // fn cancel_all_bids_with_size_incentives(
    //     &mut self,
    //     mango_account: &mut MangoAccount,
    //     perp_market: &mut PerpMarket,
    //     market_index: usize,
    //     max_depth: i64,
    //     now_ts: u64,
    //     limit: &mut u8,
    //     mut my_bids: Vec<i128>,
    //     canceled_order_ids: &mut Vec<i128>,
    // ) -> Result<()> {
    //     my_bids.sort_unstable();
    //     let mut bids_and_sizes = vec![];
    //     let mut cuml_bids = 0;

    //     let mut iter = self.bids.iter_all_including_invalid();
    //     let mut curr = iter.next();
    //     while let Some((_, bid)) = curr {
    //         match my_bids.last() {
    //             None => break,
    //             Some(&my_highest_bid) => {
    //                 if bid.key > my_highest_bid {
    //                     if bid.is_valid(now_ts) {
    //                         // if bid is not valid, it doesn't count towards book liquidity
    //                         cuml_bids += bid.quantity;
    //                     }
    //                     curr = iter.next();
    //                 } else if bid.key == my_highest_bid {
    //                     bids_and_sizes.push((bid.key, cuml_bids));
    //                     my_bids.pop();
    //                     curr = iter.next();
    //                 } else {
    //                     // my_highest_bid is not on the book; it must be on EventQueue waiting to be processed
    //                     // check the next my_highest_bid against bid
    //                     my_bids.pop();
    //                 }

    //                 if cuml_bids >= max_depth {
    //                     for bid_key in my_bids {
    //                         bids_and_sizes.push((bid_key, max_depth));
    //                     }
    //                     break;
    //                 }
    //             }
    //         }
    //     }

    //     for (key, cuml_size) in bids_and_sizes {
    //         if *limit == 0 {
    //             return Ok(());
    //         } else {
    //             *limit -= 1;
    //         }

    //         match self.cancel_order(key, Side::Bid) {
    //             Ok(order) => {
    //                 mango_account.remove_order(order.owner_slot as usize, order.quantity)?;
    //                 canceled_order_ids.push(key);
    //                 if order.version == perp_market.meta_data.version
    //                     && order.version != 0
    //                     && order.is_valid(now_ts)
    //                 {
    //                     mango_account.perp_accounts[market_index].apply_size_incentives(
    //                         perp_market,
    //                         order.best_initial,
    //                         cuml_size,
    //                         order.timestamp,
    //                         now_ts,
    //                         order.quantity,
    //                     )?;
    //                 }
    //             }
    //             Err(_) => {
    //                 msg!("Failed to cancel bid oid: {}; Either error state or bid is on EventQueue unprocessed", key)
    //             }
    //         }
    //     }
    //     Ok(())
    // }

    // /// Internal
    // fn cancel_all_asks_with_size_incentives(
    //     &mut self,
    //     mango_account: &mut MangoAccount,
    //     perp_market: &mut PerpMarket,
    //     market_index: usize,
    //     max_depth: i64,
    //     now_ts: u64,
    //     limit: &mut u8,
    //     mut my_asks: Vec<i128>,
    //     canceled_order_ids: &mut Vec<i128>,
    // ) -> Result<()> {
    //     my_asks.sort_unstable_by(|a, b| b.cmp(a));
    //     let mut asks_and_sizes = vec![];
    //     let mut cuml_asks = 0;

    //     let mut iter = self.asks.iter_all_including_invalid();
    //     let mut curr = iter.next();
    //     while let Some((_, ask)) = curr {
    //         match my_asks.last() {
    //             None => break,
    //             Some(&my_lowest_ask) => {
    //                 if ask.key < my_lowest_ask {
    //                     if ask.is_valid(now_ts) {
    //                         // if ask is not valid, it doesn't count towards book liquidity
    //                         cuml_asks += ask.quantity;
    //                     }
    //                     curr = iter.next();
    //                 } else if ask.key == my_lowest_ask {
    //                     asks_and_sizes.push((ask.key, cuml_asks));
    //                     my_asks.pop();
    //                     curr = iter.next();
    //                 } else {
    //                     // my_lowest_ask is not on the book; it must be on EventQueue waiting to be processed
    //                     // check the next my_lowest_ask against ask
    //                     my_asks.pop();
    //                 }
    //                 if cuml_asks >= max_depth {
    //                     for key in my_asks {
    //                         asks_and_sizes.push((key, max_depth))
    //                     }
    //                     break;
    //                 }
    //             }
    //         }
    //     }

    //     for (key, cuml_size) in asks_and_sizes {
    //         if *limit == 0 {
    //             return Ok(());
    //         } else {
    //             *limit -= 1;
    //         }
    //         match self.cancel_order(key, Side::Ask) {
    //             Ok(order) => {
    //                 mango_account.remove_order(order.owner_slot as usize, order.quantity)?;
    //                 canceled_order_ids.push(key);
    //                 if order.version == perp_market.meta_data.version
    //                     && order.version != 0
    //                     && order.is_valid(now_ts)
    //                 {
    //                     mango_account.perp_accounts[market_index].apply_size_incentives(
    //                         perp_market,
    //                         order.best_initial,
    //                         cuml_size,
    //                         order.timestamp,
    //                         now_ts,
    //                         order.quantity,
    //                     )?;
    //                 }
    //             }
    //             Err(_) => {
    //                 msg!("Failed to cancel ask oid: {}; Either error state or ask is on EventQueue unprocessed", key);
    //             }
    //         }
    //     }

    //     Ok(())
    // }
    // /// Cancel all the orders for MangoAccount for this PerpMarket up to `limit`
    // /// Only used when PerpMarket version == 0
    // pub fn cancel_all_with_price_incentives(
    //     &mut self,
    //     mango_account: &mut MangoAccount,
    //     perp_market: &mut PerpMarket,
    //     market_index: usize,
    //     mut limit: u8,
    // ) -> Result<()> {
    //     let now_ts = Clock::get()?.unix_timestamp as u64;

    //     for i in 0..MAX_PERP_OPEN_ORDERS {
    //         if mango_account.order_market[i] != market_index as u8 {
    //             // means slot is free or belongs to different perp market
    //             continue;
    //         }
    //         let order_id = mango_account.orders[i];
    //         let order_side = mango_account.order_side[i];

    //         let best_final = match order_side {
    //             Side::Bid => self.get_best_bid_price(now_ts).unwrap(),
    //             Side::Ask => self.get_best_ask_price(now_ts).unwrap(),
    //         };

    //         match self.cancel_order(order_id, order_side) {
    //             Ok(order) => {
    //                 // technically these should be the same. Can enable this check to be extra sure
    //                 // check!(i == order.owner_slot as usize, MathError)?;
    //                 mango_account.remove_order(order.owner_slot as usize, order.quantity)?;
    //                 if order.version != perp_market.meta_data.version {
    //                     continue;
    //                 }
    //                 mango_account.perp_accounts[market_index].apply_price_incentives(
    //                     perp_market,
    //                     order_side,
    //                     order.price(),
    //                     order.best_initial,
    //                     best_final,
    //                     order.timestamp,
    //                     now_ts,
    //                     order.quantity,
    //                 )?;
    //             }
    //             Err(_) => {
    //                 // If it's not on the book, then it has been matched and only Keeper can remove
    //             }
    //         };

    //         limit -= 1;
    //         if limit == 0 {
    //             break;
    //         }
    //     }
    //     Ok(())
    // }
}

// fn determine_ref_vars<'a>(
//     program_id: &Pubkey,
//     mango_group: &MangoGroup,
//     mango_group_pk: &Pubkey,
//     mango_cache: &MangoCache,
//     mango_account: &MangoAccount,
//     referrer_mango_account_ai: Option<&'a AccountInfo>,
//     now_ts: u64,
// ) -> Result<(I80F48, Option<RefMut<'a, MangoAccount>>)> {
//     let mngo_index = match mango_group.find_token_index(&mngo_token::id()) {
//         None => return Ok((I80F48::ZERO, None)),
//         Some(i) => i,
//     };

//     let mngo_cache = &mango_cache.root_bank_cache[mngo_index];

//     // If the user's MNGO deposit is non-zero then the rootbank cache will be checked already in `place_perp_order`.
//     // If it's zero then cache may be out of date, but it doesn't matter because 0 * index = 0
//     let mngo_deposits = mango_account.get_native_deposit(mngo_cache, mngo_index)?;
//     let ref_mngo_req = I80F48::from_num(mango_group.ref_mngo_required);
//     if mngo_deposits >= ref_mngo_req {
//         return Ok((I80F48::ZERO, None));
//     } else if let Some(referrer_mango_account_ai) = referrer_mango_account_ai {
//         // If referrer_mango_account is invalid, just treat it as if it doesn't exist
//         if let Ok(referrer_mango_account) =
//             MangoAccount::load_mut_checked(referrer_mango_account_ai, program_id, mango_group_pk)
//         {
//             // Need to check if it's valid because user may not have mngo in active assets
//             mngo_cache.check_valid(mango_group, now_ts)?;
//             let ref_mngo_deposits =
//                 referrer_mango_account.get_native_deposit(mngo_cache, mngo_index)?;

//             if !referrer_mango_account.is_bankrupt
//                 && !referrer_mango_account.being_liquidated
//                 && ref_mngo_deposits >= ref_mngo_req
//             {
//                 return Ok((
//                     I80F48::from_num(mango_group.ref_share_centibps) / CENTIBPS_PER_UNIT,
//                     Some(referrer_mango_account),
//                 ));
//             }
//         }
//     }
//     Ok((
//         I80F48::from_num(mango_group.ref_surcharge_centibps) / CENTIBPS_PER_UNIT,
//         None,
//     ))
// }

// /// Apply taker fees to the taker account and update the markets' fees_accrued for
// /// both the maker and taker fees.
// fn apply_fees(
//     market: &mut PerpMarket,
//     info: &PerpMarketInfo,
//     mango_account: &mut MangoAccount,
//     mango_account_pk: &Pubkey,
//     market_index: usize,
//     referrer_mango_account_opt: Option<RefMut<MangoAccount>>,
//     referrer_mango_account_ai: Option<&AccountInfo>,
//     total_quote_taken: i64,
//     ref_fee_rate: I80F48,
//     // perp_market_cache: &PerpMarketCache,
// ) {
//     let taker_quote_native = I80F48::from_num(
//         market
//             .quote_lot_size
//             .checked_mul(total_quote_taken)
//             .unwrap(),
//     );

//     if ref_fee_rate > I80F48::ZERO {
//         let ref_fees = taker_quote_native * ref_fee_rate;

//         // if ref mango account is some, then we send some fees over
//         if let Some(mut referrer_mango_account) = referrer_mango_account_opt {
//             mango_account.perp_accounts[market_index].transfer_quote_position(
//                 &mut referrer_mango_account.perp_accounts[market_index],
//                 ref_fees,
//             );
//             // todo
//             // emit_perp_balances(
//             //     referrer_mango_account.mango_group,
//             //     *referrer_mango_account_ai.unwrap().key,
//             //     market_index as u64,
//             //     &referrer_mango_account.perp_accounts[market_index],
//             //     perp_market_cache,
//             // );
//             // mango_emit_stack::<_, 200>(ReferralFeeAccrualLog {
//             //     mango_group: referrer_mango_account.mango_group,
//             //     referrer_mango_account: *referrer_mango_account_ai.unwrap().key,
//             //     referree_mango_account: *mango_account_pk,
//             //     market_index: market_index as u64,
//             //     referral_fee_accrual: ref_fees.to_bits(),
//             // });
//         } else {
//             // else user didn't have valid amount of MNGO and no valid referrer
//             mango_account.perp_accounts[market_index].quote_position -= ref_fees;
//             market.fees_accrued += ref_fees;
//         }
//     }

//     // Track maker fees immediately: they can be negative and applying them later
//     // risks that fees_accrued is settled to 0 before they apply. It going negative
//     // breaks assumptions.
//     // The maker fees apply to the maker's account only when the fill event is consumed.
//     let maker_fees = taker_quote_native * info.maker_fee;

//     let taker_fees = taker_quote_native * info.taker_fee;
//     mango_account.perp_accounts[market_index].quote_position -= taker_fees;
//     market.fees_accrued += taker_fees + maker_fees;

//     // todo
//     // emit_perp_balances(
//     //     mango_account.mango_group,
//     //     *mango_account_pk,
//     //     market_index as u64,
//     //     &mango_account.perp_accounts[market_index],
//     //     perp_market_cache,
//     // )
// }
