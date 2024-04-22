use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use openbook_v2::{
    program::OpenbookV2,
    state::{BookSide, Market, OpenOrdersAccount, PostOrderType, SelfTradeBehavior, Side},
};

#[derive(Copy, Clone, TryFromPrimitive, IntoPrimitive, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum OpenbookV2PlaceOrderType {
    Limit = 0,
    ImmediateOrCancel = 1,
    PostOnly = 2,
    Market = 3,
    PostOnlySlide = 4,
}

impl OpenbookV2PlaceOrderType {
    pub fn to_external_post_order_type(&self) -> Result<PostOrderType> {
        match *self {
            Self::Market => Err(MangoError::SomeError.into()),
            Self::ImmediateOrCancel => Err(MangoError::SomeError.into()),
            Self::Limit => Ok(PostOrderType::Limit),
            Self::PostOnly => Ok(PostOrderType::PostOnly),
            Self::PostOnlySlide => Ok(PostOrderType::PostOnlySlide),
        }
    }
}

#[derive(Copy, Clone, TryFromPrimitive, IntoPrimitive, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum OpenbookV2PostOrderType {
    Limit = 0,
    PostOnly = 2,
    PostOnlySlide = 4,
}

#[derive(Copy, Clone, TryFromPrimitive, IntoPrimitive, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum OpenbookV2SelfTradeBehavior {
    DecrementTake = 0,
    CancelProvide = 1,
    AbortTransaction = 2,
}
impl OpenbookV2SelfTradeBehavior {
    pub fn to_external(&self) -> SelfTradeBehavior {
        match *self {
            OpenbookV2SelfTradeBehavior::DecrementTake => SelfTradeBehavior::DecrementTake,
            OpenbookV2SelfTradeBehavior::CancelProvide => SelfTradeBehavior::CancelProvide,
            OpenbookV2SelfTradeBehavior::AbortTransaction => SelfTradeBehavior::AbortTransaction,
        }
    }
}

#[derive(Copy, Clone, TryFromPrimitive, IntoPrimitive, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum OpenbookV2Side {
    Bid = 0,
    Ask = 1,
}
impl OpenbookV2Side {
    pub fn to_external(&self) -> Side {
        match *self {
            Self::Bid => Side::Bid,
            Self::Ask => Side::Ask,
        }
    }
}

#[derive(Accounts)]
pub struct OpenbookV2PlaceOrder<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2PlaceOrder) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // authority is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    pub authority: Signer<'info>,

    #[account(mut)]
    pub open_orders: AccountLoader<'info, OpenOrdersAccount>,

    #[account(
        has_one = group,
        has_one = openbook_v2_market_external,
        has_one = openbook_v2_program,
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    pub openbook_v2_program: Program<'info, OpenbookV2>,

    #[account(
        mut,
        has_one = bids,
        has_one = asks,
        has_one = event_heap,
    )]
    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    #[account(mut)]
    /// CHECK: bids will be checked by openbook_v2
    pub bids: AccountLoader<'info, BookSide>,

    #[account(mut)]
    /// CHECK: asks will be checked by openbook_v2
    pub asks: AccountLoader<'info, BookSide>,

    #[account(mut)]
    /// CHECK: event queue will be checked by openbook_v2
    pub event_heap: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: vault will be checked by openbook_v2
    pub market_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_vault_signer: UncheckedAccount<'info>,

    /// The bank that pays for the order. Bank oracle also expected in remaining_accounts
    //  payer_bank.vault == payer_vault is validated inline at #3
    //  bank.token_index is validated against the openbook market at #4
    #[account(mut, has_one = group)]
    pub payer_bank: AccountLoader<'info, Bank>,
    /// The bank vault that pays for the order
    #[account(mut)]
    pub payer_vault: Box<Account<'info, TokenAccount>>,

    /// The bank that receives the funds upon settlement. Bank oracle also expected in remaining_accounts
    //  bank.token_index is validated against the openbook market at #4
    #[account(mut, has_one = group)]
    pub receiver_bank: AccountLoader<'info, Bank>,

    pub token_program: Program<'info, Token>,
}
