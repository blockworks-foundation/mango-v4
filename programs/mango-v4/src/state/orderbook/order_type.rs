use anchor_lang::prelude::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};

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
}
