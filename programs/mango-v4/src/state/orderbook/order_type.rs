use anchor_lang::prelude::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::state::BookSide2Component;

#[derive(
    Eq,
    PartialEq,
    Copy,
    Clone,
    TryFromPrimitive,
    IntoPrimitive,
    Debug,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[repr(u8)]
pub enum OrderType {
    /// Take existing orders up to price, max_base_quantity and max_quote_quantity.
    /// If any base_quantity or quote_quantity remains, place an order on the book
    Limit = 0,

    /// Take existing orders up to price, max_base_quantity and max_quote_quantity.
    /// Never place an order on the book.
    ImmediateOrCancel = 1,

    /// Never take any existing orders, post the order on the book if possible.
    /// If existing orders can match with this order, do nothing.
    PostOnly = 2,

    /// Ignore price and take orders up to max_base_quantity and max_quote_quantity.
    /// Never place an order on the book.
    ///
    /// Equivalent to ImmediateOrCancel with price=i64::MAX.
    Market = 3,

    /// If existing orders match with this order, adjust the price to just barely
    /// not match. Always places an order on the book.
    PostOnlySlide = 4,
}

#[derive(
    Eq,
    PartialEq,
    Copy,
    Clone,
    TryFromPrimitive,
    IntoPrimitive,
    Debug,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[repr(u8)]
pub enum Side {
    Bid = 0,
    Ask = 1,
}

impl Side {
    pub fn invert_side(self: &Side) -> Side {
        match self {
            Side::Bid => Side::Ask,
            Side::Ask => Side::Bid,
        }
    }

    /// Is `lhs` is a better order for `side` than `rhs`?
    pub fn is_price_data_better(self: &Side, lhs: u64, rhs: u64) -> bool {
        match self {
            Side::Bid => lhs > rhs,
            Side::Ask => lhs < rhs,
        }
    }

    /// Is `lhs` is a better order for `side` than `rhs`?
    pub fn is_price_better(self: &Side, lhs: i64, rhs: i64) -> bool {
        match self {
            Side::Bid => lhs > rhs,
            Side::Ask => lhs < rhs,
        }
    }

    /// Is `price` acceptable for a `limit` order on `side`?
    pub fn is_price_within_limit(self: &Side, price: i64, limit: i64) -> bool {
        match self {
            Side::Bid => price <= limit,
            Side::Ask => price >= limit,
        }
    }
}

#[derive(
    Eq,
    PartialEq,
    Copy,
    Clone,
    TryFromPrimitive,
    IntoPrimitive,
    Debug,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[repr(u8)]
pub enum SideAndComponent {
    BidDirect = 0,
    AskDirect = 1,
    BidOraclePegged = 2,
    AskOraclePegged = 3,
}

impl SideAndComponent {
    pub fn side(&self) -> Side {
        match self {
            Self::BidDirect | Self::BidOraclePegged => Side::Bid,
            Self::AskDirect | Self::AskOraclePegged => Side::Ask,
        }
    }

    pub fn component(&self) -> BookSide2Component {
        match self {
            Self::BidDirect | Self::AskDirect => BookSide2Component::Direct,
            Self::BidOraclePegged | Self::AskOraclePegged => BookSide2Component::OraclePegged,
        }
    }
}
