use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use derivative::Derivative;
use fixed::types::I80F48;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::error::*;
use crate::PerpMarketIndex;

#[account(zero_copy)]
#[derive(Debug)]
pub struct TriggerAction {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,

    // ABI: Clients rely on this being at offset 40
    pub account: Pubkey,

    pub owner: Pubkey,

    // For recreating the seed
    pub trigger_num: u64,

    // TODO: expiry?
    // TODO: incentive (just SOL on this account maybe?)
    pub condition_type: u32,
    pub condition_bytes: u32,
    pub action_type: u32,
    pub action_bytes: u32,

    pub reserved: [u8; 1000],
    // Condition and Action bytes trail this struct in the account!
}
const_assert_eq!(size_of::<TriggerAction>(), 32 * 3 + 8 + 4 * 4 + 1000);
const_assert_eq!(size_of::<TriggerAction>(), 1120);
const_assert_eq!(size_of::<TriggerAction>() % 8, 0);

impl TriggerAction {
    // TODO: check owner, doesn't work with Ref<>
    pub fn from_account_bytes<'a>(
        bytes: &'a [u8],
    ) -> Result<(&TriggerAction, ConditionRef<'a>, ActionRef<'a>)> {
        if bytes.len() < 8 {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        let disc_bytes = &bytes[0..8];
        if disc_bytes != &TriggerAction::DISCRIMINATOR {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        let (fixed_bytes, tail_bytes) = bytes.split_at(8 + std::mem::size_of::<TriggerAction>());
        let fixed: &TriggerAction = bytemuck::from_bytes(&fixed_bytes[8..]);

        let (condition_bytes, action_bytes) =
            tail_bytes.split_at(fixed.condition_bytes.try_into().unwrap());
        require_eq!(action_bytes.len(), fixed.action_bytes as usize);

        let condition_type: u32 = *bytemuck::from_bytes(&condition_bytes[0..4]);
        require_eq!(condition_type, fixed.condition_type);
        let condition = ConditionRef::from_bytes(condition_bytes)?;

        let action_type: u32 = *bytemuck::from_bytes(&action_bytes[0..4]);
        require_eq!(action_type, fixed.action_type);
        let action = ActionRef::from_bytes(action_bytes)?;

        Ok((fixed, condition, action))
    }
}

#[repr(u32)]
#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive)]
pub enum ConditionType {
    OraclePrice,
}

#[derive(Debug)]
pub enum ConditionRef<'a> {
    OraclePrice(&'a OraclePriceCondition),
}

impl<'a> ConditionRef<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<ConditionRef<'a>> {
        let raw_type: u32 = *bytemuck::from_bytes(&bytes[0..4]);
        let condition_type: ConditionType = raw_type.try_into().unwrap();
        match condition_type {
            ConditionType::OraclePrice => {
                Ok(ConditionRef::OraclePrice(bytemuck::from_bytes(bytes)))
            }
            _ => {
                error_msg!("bad condition type")
            }
        }
    }
}

#[zero_copy]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct OraclePriceCondition {
    pub condition_type: u32, // always ConditionType::OraclePrice
    #[derivative(Debug = "ignore")]
    pub padding0: u32,
    pub oracle: Pubkey,
    pub threshold: I80F48,
    pub trigger_when_above: u8, // TODO: don't use a bool
    // TODO: also oracle staleness and confidence
    #[derivative(Debug = "ignore")]
    pub padding: [u8; 7],
}

#[repr(u32)]
#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive)]
pub enum ActionType {
    PerpPlaceOrder,
}

#[derive(Debug)]
pub enum ActionRef<'a> {
    PerpPlaceOrder(&'a PerpPlaceOrderAction),
}

impl<'a> ActionRef<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<ActionRef<'a>> {
        let raw_type: u32 = *bytemuck::from_bytes(&bytes[0..4]);
        let action_type: ActionType = raw_type.try_into().unwrap();
        match action_type {
            ActionType::PerpPlaceOrder => {
                Ok(ActionRef::PerpPlaceOrder(bytemuck::from_bytes(bytes)))
            }
            _ => {
                error_msg!("bad action type")
            }
        }
    }
}

#[zero_copy]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct PerpPlaceOrderAction {
    pub action_type: u32, // always ActionType::PerpPlaceOrder
    pub perp_market_index: PerpMarketIndex,
    // TODO: basically the Order struct, though that one is internal; and this struct is a public interface
    #[derivative(Debug = "ignore")]
    pub padding: u16,
}
