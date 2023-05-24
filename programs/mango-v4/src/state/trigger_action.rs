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

    pub fn check(&self, accounts: &[AccountInfo]) -> Result<()> {
        match self {
            ConditionRef::OraclePrice(c) => c.check(accounts),
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

impl OraclePriceCondition {
    pub fn check(&self, accounts: &[AccountInfo]) -> Result<()> {
        require_eq!(accounts.len(), 1);
        let oracle_ai = &accounts[0];
        require_keys_eq!(*oracle_ai.key, self.oracle);

        // TODO: grab the price and compare it

        Ok(())
    }
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

    pub fn execute(&self, accounts: &[AccountInfo]) -> Result<()> {
        match self {
            ActionRef::PerpPlaceOrder(p) => p.execute(accounts),
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

impl PerpPlaceOrderAction {
    pub fn execute(accounts: &[AccountInfo]) -> Result<()> {
        // TODO: Grab all the accounts needed for calling perp_place_order()
        // and either make a context object or something intermediate that can be shared.
        let mut mut_accounts = accounts;
        // TODO: owner doesn't need to be a signer, doesn't even need to be passed...
        // TODO: need to validate the inner and outer group accounts match
        // TODO: need to validate the inner account is owned/delegated by the external owner
        // TODO: We're in a tricky situation here:
        //       - Anchor constructs the idl from the accounts struct, that's good, but means we can't
        //         make `owner` not a signer without making bad changes to the idl.
        //       - We _do_ want to reuse most of the validation logic from perp_place_order.
        //       - Some of this validation logic is currently in the accounts struct constraints.
        //         And some of it also ends up affecting the idl (has_one constraints...).
        //       - Also, some validation might be in lib.rs, double check this.
        // One solution might be to duplicate the validation in the actual program code that we reuse here.
        // And also keep it in the accounts struct to generate a nice idl.
        // Then calling the actual instruction code should no longer use the ctx object and instead use something
        // sharable.
        let mut place_order_accts = crate::accounts_ix::PerpPlaceOrder::try_accounts(
            &crate::id(),
            &mut mut_accounts,
            &[],
            bumps,
            reallocs,
        )?;
        let ctx = Context {
            program_id: &crate::id(),
            accounts: &mut place_order_accts,
            remaining_accounts: &mut mut_accounts,
            bumps: (),
        };

        // TODO: Make an "Order" struct from self
        let order = crate::state::Order {};

        // TODO: ?
        let limit = 10;

        // TODO: probably move this whole set of functions into GPL?
        #[cfg(feature = "enable-gpl")]
        return crate::instructions::perp_place_order(ctx, order, limit).map(|_| ());

        #[cfg(not(feature = "enable-gpl"))]
        error_msg!("not gpl")
    }
}
